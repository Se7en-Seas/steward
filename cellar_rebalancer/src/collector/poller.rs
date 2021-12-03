//! Poller
/// The collector's [`Poller`] collects information from external sources
/// which aren't capable of pushing data.
use crate::{
    cellar_uniswap_wrapper::{ContractStateUpdate, UniswapV3CellarState},
    collector, config,
    error::Error,
    gas::CellarGas,
    prelude::*,
    time_range::{TickWeight, TimeRange},
    uniswap_pool::PoolState,
};
use abscissa_core::error::BoxError;
use ethers::prelude::*;
use rebalancer_abi::cellar_uniswap::*;
use std::{result::Result, sync::Arc, time::Duration};
use tokio::{time, try_join};
use tower::Service;

// Struct poller to collect poll_interval etc. from external sources which aren't capable of pushing data
#[allow(dead_code)]
pub struct Poller<T: Middleware> {
    poll_interval: Duration,
    time_range: TimeRange,
    cellar_gas: CellarGas,
    contract_state: UniswapV3CellarState<T>,
    pool: PoolState<T>,
}

pub fn from_tick_weight(tick_weight: TickWeight) -> CellarTickInfo {
    CellarTickInfo {
        token_id: U256::zero(),
        tick_upper: tick_weight.upper_bound,
        tick_lower: tick_weight.lower_bound,
        weight: tick_weight.weight,
    }
}

// Implement poller middleware
impl<T: 'static + Middleware> Poller<T> {
    pub async fn new(
        cellar: &config::CellarConfig,
        client: Arc<T>,
        mongo: &config::MongoSection,
    ) -> Result<Self, Error> {
        let pool = PoolState::new(cellar.pool_address, client.clone());
        let spacing = pool
            .contract
            .tick_spacing()
            .call()
            .await
            .expect("Could not get spacing by querying contract");

        let poller = Poller {
            poll_interval: cellar.duration,
            time_range: TimeRange {
                time: None,
                previous_update: None,
                pair_id: cellar.pair_id,
                token_info: (cellar.token_0.clone(), cellar.token_1.clone()),
                weight_factor: cellar.weight_factor,
                tick_weights: vec![],
                monogo_uri: mongo.host.clone(),
                pair_database: cellar.pair_database.clone(),
                tick_spacing: spacing,
            },
            cellar_gas: CellarGas {
                max_gas_price: ethers::utils::parse_units(cellar.max_gas_price_gwei, "gwei")
                    .unwrap(),
                current_gas: None,
            },
            contract_state: UniswapV3CellarState::new(cellar.cellar_address, client),
            pool,
        };

        Ok(poller)
    }

    // Retrieve poll time range
    pub async fn poll_time_range(&self) -> Result<TimeRange, Error> {
        info!("{} polling time range", self.time_range.pair_database);

        let mut time_range = self.time_range.clone();

        time_range.poll().await;
        Ok(time_range)
    }

    // Retrieve current standard gas price from etherscan
    pub async fn poll_cellar_gas(&self) -> Result<U256, Error> {
        CellarGas::etherscan_standard().await.map_err(|e| e.into())
    }

    // Retrieve the current contract state
    pub async fn poll_contract_state(&self) -> Result<ContractStateUpdate, Error> {
        Ok(ContractStateUpdate {})
    }

    // Update poller with time_range, gas price and contract_state
    pub fn update_poller(
        &mut self,
        time_range: TimeRange,
        gas: U256,
        _contract_state: ContractStateUpdate,
    ) {
        self.cellar_gas.current_gas = Some(gas);
        self.contract_state.gas_price = Some(gas);
        self.time_range = time_range;
    }

    pub async fn decide_rebalance(&mut self) -> Result<(), Error> {
        let mut tick_info: Vec<CellarTickInfo> = Vec::new();
        for ref tick_weight in self.time_range.tick_weights.clone() {
            if tick_weight.weight > 0 {
                tick_info.push(from_tick_weight(tick_weight.clone()))
            }
        }

        if std::env::var("CELLAR_DRY_RUN").expect("Expect CELLAR_DRY_RUN var") == "TRUE" {
            Ok(())
        } else {
            tick_info.reverse();
            self.contract_state.rebalance(tick_info).await
        }
    }

    // Route incoming requests.
    pub async fn run<S>(mut self, collector: S)
    where
        S: Service<collector::Request, Response = collector::Response, Error = BoxError>
            + Send
            + Clone
            + 'static,
    {
        info!(
            "{} polling every {:?}",
            self.time_range.pair_database, self.poll_interval
        );

        let mut interval = time::interval(self.poll_interval);
        loop {
            interval.tick().await;
            self.poll(&collector).await;
            info!(
                "{} waiting for {:?}",
                self.time_range.pair_database, self.poll_interval
            );
        }
    }

    async fn poll<S>(&mut self, _collector: &S)
    where
        S: Service<collector::Request, Response = collector::Response, Error = BoxError>
            + Send
            + Clone
            + 'static,
    {
        let res = try_join!(
            self.poll_time_range(),
            self.poll_cellar_gas(),
            self.poll_contract_state(),
        );
        match res {
            Ok((time_range, gas, contract_state_update)) => {
                self.update_poller(time_range, gas, contract_state_update);
                self.decide_rebalance().await.unwrap();
            }
            Err(e) => error!("Error fetching data {}", e),
        }
    }
}
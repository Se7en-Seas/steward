//! Poller
/// The collector's [`Poller`] collects information from external sources
/// which aren't capable of pushing data.
use crate::{
    cellar_wrapper::{ContractState, ContractStateUpdate,CellarTickInfo},
    collector, config,
    error::Error,
    gas::CellarGas,
    prelude::*,
    time_range::TimeRange,
};
use abscissa_core::error::BoxError;
use ethers::prelude::*;
use std::{sync::Arc, time::Duration};
use tokio::time;
use tokio::try_join;
use tower::Service;

// Struct poller to collect poll_interval etc. from external sources which aren't capable of pushing data
pub struct Poller<T: Middleware> {
    poll_interval: Duration,
    time_range: TimeRange,
    cellar_gas: CellarGas,
    contract_state: ContractState<T>,
}

// Implement poller middleware
impl<T: 'static + Middleware> Poller<T> {
    pub fn new(config: &config::CellarRebalancerConfig, client: Arc<T>) -> Result<Self, Error> {
        Ok(Poller {
            poll_interval: config.cellar.duration,
            time_range: TimeRange {
                time: None,
                previous_update: None,
                pair_id: config.cellar.pair_id,
                token_info: (config.cellar.token_0.clone(), config.cellar.token_1.clone()),
                weight_factor: config.cellar.weight_factor,
                tick_weights: vec![],
                monogo_uri: config.mongo.host.clone(),
            },
            cellar_gas: CellarGas {
                max_gas_price: ethers::utils::parse_units(config.cellar.max_gas_price_gwei, "gwei").unwrap(),
                current_gas: None   ,
            },
            contract_state: ContractState::new(config.cellar.cellar_addresses, client),
        })
    }

    // Retrieve poll time range
    pub async fn poll_time_range(&self) -> Result<TimeRange, Error> {
        info!("Polling time range");

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
    self.time_range = time_range;
    }

    pub async fn decide_rebalance(&mut self) -> Result<(), Error> {
        
        let mut tick_info:Vec<CellarTickInfo> = Vec::new();
        for ref tick_weight in self.time_range.tick_weights.clone() {
            tick_info.push(CellarTickInfo::from_tick_weight(self.time_range.pair_id, tick_weight))
        }
        self.contract_state.rebalance(tick_info).await

    }

    // Route incoming requests.
    pub async fn run<S>(mut self, collector: S)
    where
        S: Service<collector::Request, Response = collector::Response, Error = BoxError>
            + Send
            + Clone
            + 'static,
    {
        info!("polling every {:?}", self.poll_interval);
        let mut interval = time::interval(self.poll_interval);

        loop {
            interval.tick().await;
            self.poll(&collector).await;
            info!("waiting for {:?}", self.poll_interval);
        }
    }

    async fn poll<S>(&mut self, collector: &S)
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

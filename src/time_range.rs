//! Time independent bollinger ranges
use crate::config::TokenInfo;
/// This is a Rust type for the JSON data from time independent bollinger ranges.
use ethers::prelude::*;
use futures::TryStreamExt;
use num_bigint::ToBigInt;
use uniswap_v3_sdk::{Price, Token};

use crate::prelude::*;
use chrono::DateTime;
use mongodb::{
    bson::{doc},
    options::FindOptions,
    Client,
};
use serde::{Deserialize, Serialize};

// Struct TimeRange for time independent bollinger ranges
#[derive(Serialize, Deserialize, Clone)]
pub struct TimeRange {
    pub time: Option<DateTime<chrono::Utc>>,
    pub previous_update: Option<DateTime<chrono::Utc>>,
    pub pair_id: U256,
    pub token_info: (TokenInfo, TokenInfo),
    pub weight_factor: u32,
    pub tick_weights: Vec<TickWeight>,
    pub monogo_uri: String,
}

impl Default for TimeRange {
    fn default() -> Self {
        TimeRange {
            time: None,
            previous_update: None,
            pair_id: U256::zero(),
            tick_weights: Vec::new(),
            weight_factor: 100,
            token_info: (TokenInfo::default(), TokenInfo::default()),
            monogo_uri: "mongodb://localhost:27017/?directconnection=true".to_string(),
        }
    }
}

/// Implementation for TimeRange field format
impl std::fmt::Debug for TimeRange {
    // Implement TimeRange field format for time, previous_update, pair_id and tick_weight
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = f.debug_struct("TimeRange");
        fields
            .field("time", &self.time)
            .field("previous_update", &self.previous_update)
            .field("pair_id", &self.pair_id)
            .field("token_info_0", &self.token_info.0)
            .field("token_info_1", &self.token_info.1);
        for (i, tick) in self.tick_weights.iter().enumerate() {
            fields.field(&format!("tick_weight #:{}", i), tick);
        }
        fields.finish()
    }
}

/// Struct TickWeights for time independent bollinger ranges
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickWeight {
    pub upper_bound: i32,
    pub lower_bound: i32,
    pub weight: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MongoData {
    pub _id: mongodb::bson::Bson,
    pub created_timestamp: mongodb::bson::Bson,
    pub pair_id: ethers::prelude::U256,
    pub symbol: String,
    pub tick_weights: Vec<MongoTickWeights>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MongoTickWeights {
    pub lower: mongodb::bson::Bson,
    pub upper: mongodb::bson::Bson,
    pub weight: mongodb::bson::Bson,
}

// Implement TimeRange for time independent bollinger ranges
impl TimeRange {
    // Instantiate TimeRange for toime independent bollinger ranges with fn new.
    pub fn new(
        time: Option<DateTime<chrono::Utc>>,
        previous_update: Option<DateTime<chrono::Utc>>,
        pair_id: U256,
        weight_factor: u32,
        tick_weights: Vec<TickWeight>,
        token_0_info: TokenInfo,
        token_1_info: TokenInfo,
        monogo_uri: String,
    ) -> Self {
        TimeRange {
            time,
            previous_update,
            pair_id,
            weight_factor,
            tick_weights: tick_weights,
            token_info: (token_0_info, token_1_info),
            monogo_uri,
        }
    }

    pub async fn poll(&mut self) {
        let client = Client::with_uri_str(self.monogo_uri.clone()).await.unwrap();

        let db = client.database("predictions");

        // Get a handle to a collection in the database.
        let collection = db.collection::<MongoData>("tick_range_predictions");

        let find_options = FindOptions::builder()
            .sort(doc! { "created_timestamp": -1 })
            .build();

        let mut sorted_predictions = collection.find(None, find_options).await.unwrap();

        if let Some(latest_prediction) = sorted_predictions.try_next().await.unwrap() {
            info!("Latest prediction: {:?}", latest_prediction);
            self.previous_update = self.time;
            self.time = Some(
                latest_prediction
                    .created_timestamp
                    .as_datetime()
                    .unwrap()
                    .to_chrono(),
            );
            self.pair_id = latest_prediction.pair_id;
            self.tick_weights.clear();
            for tick_weight in latest_prediction.tick_weights {
                let upper_float = tick_weight.upper.as_f64().unwrap();
                let lower_float = tick_weight.lower.as_f64().unwrap();
                let upper_price =
                    f64_unit_to_price(upper_float, &self.token_info.0, &self.token_info.1);
                let lower_price =
                    f64_unit_to_price(lower_float, &self.token_info.0, &self.token_info.1);
                let upper_tick = uniswap_v3_sdk::priceToTick(upper_price);
                let lower_tick = uniswap_v3_sdk::priceToTick(lower_price);
                let weight: u32 =
                    (self.weight_factor as f64 * tick_weight.weight.as_f64().unwrap()) as u32;
                self.tick_weights.push(TickWeight {
                    upper_bound: upper_tick,
                    lower_bound: lower_tick,
                    weight: weight,
                });
            }
        }
        info!("TimeRange: {:?}", self);
    }
}

fn f64_unit_to_price(f64: f64, token_0: &TokenInfo, token_1: &TokenInfo) -> Price {
    Price {
        token_0: Token {
            symbol: token_0.symbol.clone(),
            address: token_0.address.to_string(),
        },
        token_1: Token {
            symbol: token_1.symbol.clone(),
            address: token_1.address.to_string(),
        },
        amount_0: f64.to_bigint().unwrap()
            * (10i32.to_bigint().unwrap().pow(token_0.decimals.into())),
        amount_1: (1 * (10i32.to_bigint().unwrap().pow(token_1.decimals.into()))),
    }
}

use chrono::{NaiveDate, DateTime, Utc};
use serde::Deserialize;

pub type MarketRegionHistory = Vec<MarketRegionHistoryItem>;

#[derive(Debug, Deserialize)]
pub struct MarketRegionHistoryItem {
    pub average: f64,
    pub date: NaiveDate,
    pub highest: f64,
    pub lowest: f64,
    pub order_count: i64,
    pub volume: i64,
}

pub type MarketRegionOrders = Vec<MarketRegionOrdersItem>;

#[derive(Debug, Deserialize)]
pub struct MarketRegionOrdersItem {
    pub duration: u64,
    pub is_buy_order: bool,
    pub issued: DateTime<Utc>,
    pub location_id: u64,
    pub min_volume: u64,
    pub order_id: u64,
    pub price: f64,
    pub range: MarketRegionOrderRange,
    pub system_id: u64,
    pub type_id: u64,
    pub volume_remain: u64,
    pub volume_total: u64,
}

#[derive(Debug, Deserialize)]
pub enum MarketRegionOrderRange {
    #[serde(rename = "station")]
    Station,
    #[serde(rename = "region")]
    Region,
    #[serde(rename = "solarsystem")]
    SolarSystem,
    #[serde(rename = "1")]
    R1,
    #[serde(rename = "2")]
    R2,
    #[serde(rename = "3")]
    R3,
    #[serde(rename = "4")]
    R4,
    #[serde(rename = "5")]
    R5,
    #[serde(rename = "10")]
    R10,
    #[serde(rename = "20")]
    R20,
    #[serde(rename = "30")]
    R30,
    #[serde(rename = "40")]
    R40,
}

pub type MarketRegionTypes = Vec<i32>;

#[derive(Debug, Deserialize)]
pub struct UniverseTypeId {
    pub name: String,
    pub published: bool,
}

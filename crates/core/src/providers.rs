//! Provider interfaces (Phase 0). Every external data source sits behind one of
//! these traits so core domain entities never depend on provider formats.
//! Concrete providers arrive in their phase (atlas: P3, market: P5, crowd: P8).

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Health snapshot for any provider/dataset pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider: String,
    pub dataset: String,
    pub last_success_at: Option<i64>,
    pub last_attempt_at: i64,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub next_retry_at: i64,
}

/// Minimal HTTP fetch abstraction. Implemented by the Tauri shell (`reqwest`);
/// core and its tests use mock implementations. Sync by design (D04).
pub trait HttpFetcher: Send + Sync {
    fn get(&self, url: &str, max_bytes: usize) -> Result<String>;
}

/// Normalized market row produced by any market provider's parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRow {
    pub item_id: i64,
    pub market_value: i64,
    pub min_buyout: i64,
    pub historical: i64,
    pub sale_avg: i64,
    pub sale_rate: f64,
    pub quantity: i64,
}

/// Market data provider: knows dataset URLs and how to parse their bytes.
pub trait MarketProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn dataset_urls(&self, region: &str) -> Vec<(String, String)>;
    fn parse(&self, dataset: &str, bytes: &[u8]) -> Result<ParsedMarket>;
}

/// Result of parsing one dataset: rows plus skip accounting.
#[derive(Debug, Default)]
pub struct ParsedMarket {
    pub rows: Vec<MarketRow>,
    pub rows_total: usize,
    pub rows_skipped: usize,
}

/// Normalized atlas observation produced by any atlas provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasObservation {
    pub zone_id: String,
    pub resource_id: String,
    pub map_id: Option<i64>,
    pub x: f64,
    pub y: f64,
    pub yield_qty: Option<f64>,
    pub observed_at: i64,
}

/// Atlas provider: seeds or imports canonical zones/resources/observations.
pub trait AtlasProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn observations(&self, zone_id: &str) -> Result<Vec<AtlasObservation>>;
}

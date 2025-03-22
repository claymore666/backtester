use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BinanceCandle {
    pub id: i32,
    pub symbol: String,
    pub interval: String,
    pub open_time: DateTime<Utc>,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume: f64,
    pub close_time: DateTime<Utc>,
    pub quote_asset_volume: f64,
    pub number_of_trades: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IndicatorConfig {
    pub id: i32,
    pub symbol: String,
    pub interval: String, 
    pub indicator_type: String,  // "oscillator", "overlap", "volume", "volatility", "pattern"
    pub indicator_name: String,  // e.g., "RSI", "SMA", "BBANDS"
    pub parameters: serde_json::Value, // JSON object containing parameters
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CalculatedIndicator {
    pub id: i32,
    pub symbol: String,
    pub interval: String,
    pub indicator_type: String,
    pub indicator_name: String,
    pub parameters: serde_json::Value,
    pub time: DateTime<Utc>,
    pub value: serde_json::Value, // Can store different types of results as JSON
    pub created_at: DateTime<Utc>,
}

// This struct will be used to batch insert calculated indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedIndicatorBatch {
    pub symbol: String,
    pub interval: String,
    pub indicator_type: String,
    pub indicator_name: String,
    pub parameters: serde_json::Value,
    pub time: DateTime<Utc>,
    pub value: serde_json::Value,
}

// This struct represents the data we'll fetch from the database
#[derive(Debug, Clone)]
pub struct CandleData {
    pub symbol: String,
    pub interval: String,
    pub open_time: Vec<DateTime<Utc>>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Vec<f64>,
}

impl CandleData {
    pub fn new(symbol: String, interval: String) -> Self {
        Self {
            symbol,
            interval,
            open_time: Vec::new(),
            open: Vec::new(),
            high: Vec::new(),
            low: Vec::new(),
            close: Vec::new(),
            volume: Vec::new(),
        }
    }

    pub fn from_candles(candles: Vec<BinanceCandle>) -> Self {
        if candles.is_empty() {
            return Self::new(String::new(), String::new());
        }

        let first_candle = &candles[0];
        let mut data = Self::new(first_candle.symbol.clone(), first_candle.interval.clone());

        for candle in candles {
            data.open_time.push(candle.open_time);
            data.open.push(candle.open_price);
            data.high.push(candle.high_price);
            data.low.push(candle.low_price);
            data.close.push(candle.close_price);
            data.volume.push(candle.volume);
        }

        data
    }
}

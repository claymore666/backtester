use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Binance candle model from database
#[derive(Debug, Clone, FromRow)]
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

// Candle data for calculations
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
    pub close_time: Vec<DateTime<Utc>>,
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
            close_time: Vec::new(),
        }
    }

    pub fn from_candles(candles: Vec<BinanceCandle>) -> Self {
        let symbol = if let Some(candle) = candles.first() {
            candle.symbol.clone()
        } else {
            String::new()
        };

        let interval = if let Some(candle) = candles.first() {
            candle.interval.clone()
        } else {
            String::new()
        };

        let mut data = Self::new(symbol, interval);

        for candle in candles {
            data.open_time.push(candle.open_time);
            data.open.push(candle.open_price);
            data.high.push(candle.high_price);
            data.low.push(candle.low_price);
            data.close.push(candle.close_price);
            data.volume.push(candle.volume);
            data.close_time.push(candle.close_time);
        }

        data
    }

    pub fn len(&self) -> usize {
        self.close.len()
    }

    pub fn is_empty(&self) -> bool {
        self.close.is_empty()
    }
}

// Technical indicator configuration from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct IndicatorConfig {
    pub id: i32,
    pub symbol: String,
    pub interval: String,
    pub indicator_type: String,
    pub indicator_name: String,
    pub parameters: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Batch data for calculated indicators
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

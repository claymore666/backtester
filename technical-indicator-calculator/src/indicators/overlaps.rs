use crate::database::models::CandleData;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use ta::indicators::{
    SimpleMovingAverage, ExponentialMovingAverage, WeightedMovingAverage,
    DoubleExponentialMovingAverage, TripleExponentialMovingAverage,
    TriangularMovingAverage, KaufmanAdaptiveMovingAverage, 
    BollingerBands, ParabolicSar,
};
use ta::Next;
use tracing::{debug, error, info};

pub struct OverlapCalculator;

impl OverlapCalculator {
    // Calculate SMA (Simple Moving Average)
    pub fn calculate_sma(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for SMA calculation"));
        }

        let mut sma = SimpleMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = sma.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate EMA (Exponential Moving Average)
    pub fn calculate_ema(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for EMA calculation"));
        }

        let mut ema = ExponentialMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = ema.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate WMA (Weighted Moving Average)
    pub fn calculate_wma(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for WMA calculation"));
        }

        let mut wma = WeightedMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = wma.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate DEMA (Double Exponential Moving Average)
    pub fn calculate_dema(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period * 2 {
            return Err(anyhow::anyhow!("Not enough data points for DEMA calculation"));
        }

        let mut dema = DoubleExponentialMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = dema.next(candle_data.close[i]);
            
            // Only include values after sufficient data (initial values are NaN)
            if i >= period * 2 - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate TEMA (Triple Exponential Moving Average)
    pub fn calculate_tema(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period * 3 {
            return Err(anyhow::anyhow!("Not enough data points for TEMA calculation"));
        }

        let mut tema = TripleExponentialMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = tema.next(candle_data.close[i]);
            
            // Only include values after sufficient data (initial values are NaN)
            if i >= period * 3 - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate TRIMA (Triangular Moving Average)
    pub fn calculate_trima(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period * 2 {
            return Err(anyhow::anyhow!("Not enough data points for TRIMA calculation"));
        }

        let mut trima = TriangularMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = trima.next(candle_data.close[i]);
            
            // Only include values after sufficient data (initial values are NaN)
            if i >= period * 2 - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate KAMA (Kaufman Adaptive Moving Average)
    pub fn calculate_kama(
        candle_data: &CandleData,
        period: usize,
        fast_ema: usize,
        slow_ema: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for KAMA calculation"));
        }

        let mut kama = KaufmanAdaptiveMovingAverage::new(period, fast_ema as f64, slow_ema as f64)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = kama.next(candle_data.close[i]);
            
            // Only include values after sufficient data (initial values are NaN)
            if i >= period && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate Bollinger Bands
    pub fn calculate_bollinger_bands(
        candle_data: &CandleData,
        period: usize,
        deviation_multiplier: f64,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for Bollinger Bands calculation"));
        }

        let mut bb = BollingerBands::new(period, deviation_multiplier)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = bb.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period - 1 && !value.middle.is_nan() && !value.upper.is_nan() && !value.lower.is_nan() {
                let bb_value = json!({
                    "middle": value.middle,
                    "upper": value.upper,
                    "lower": value.lower,
                });
                
                results.push((candle_data.open_time[i], bb_value));
            }
        }

        Ok(results)
    }

    // Calculate Parabolic SAR
    pub fn calculate_parabolic_sar(
        candle_data: &CandleData,
        acceleration: f64,
        maximum: f64,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < 2 {
            return Err(anyhow::anyhow!("Not enough data points for Parabolic SAR calculation"));
        }

        let mut psar = ParabolicSar::new(acceleration, maximum)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        // SAR requires at least 2 candles, and the first value is usually not meaningful
        for i in 2..candle_data.close.len() {
            let value = psar.next(candle_data.high[i], candle_data.low[i]);
            
            if !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }
}

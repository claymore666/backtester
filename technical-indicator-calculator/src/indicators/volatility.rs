use crate::database::models::CandleData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use ta::indicators::{
    AverageTrueRange, 
    StandardDeviation,
};
use ta::Next;
use tracing::{debug, error, info};

pub struct VolatilityCalculator;

impl VolatilityCalculator {
    // Calculate ATR (Average True Range)
    pub fn calculate_atr(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for ATR calculation"));
        }

        let mut atr = AverageTrueRange::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());
        
        // Calculate true range values first
        let tr_values = Self::calculate_true_range(candle_data)?;
        
        // Use true range values to calculate ATR
        for (i, (time, tr)) in tr_values.iter().enumerate() {
            if i >= period - 1 {
                // Feed the true range value to the ATR indicator
                let value = atr.next(*tr);
                
                if !value.is_nan() {
                    results.push((*time, value));
                }
            }
        }

        Ok(results)
    }

    // Calculate NATR (Normalized ATR - as percentage of close price)
    pub fn calculate_natr(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for NATR calculation"));
        }

        // Get ATR values
        let atr_results = Self::calculate_atr(candle_data, period)?;
        let mut natr_results = Vec::with_capacity(atr_results.len());
        
        // Convert to percentage of close price
        for (i, (time, atr_value)) in atr_results.iter().enumerate() {
            // Find the corresponding candle index
            if let Some(candle_index) = candle_data.open_time.iter().position(|&t| t == *time) {
                let close_price = candle_data.close[candle_index];
                
                if close_price > 0.0 {
                    let natr = (atr_value / close_price) * 100.0;
                    natr_results.push((*time, natr));
                }
            }
        }

        Ok(natr_results)
    }

    // Calculate True Range
    pub fn calculate_true_range(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < 2 {
            return Err(anyhow::anyhow!("Not enough data points for True Range calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len() - 1);

        // Skip first candle as we need a previous close for calculation
        for i in 1..candle_data.close.len() {
            // Calculate true range manually
            // TR = max(high - low, |high - prev_close|, |low - prev_close|)
            let high = candle_data.high[i];
            let low = candle_data.low[i];
            let prev_close = candle_data.close[i - 1];
            
            let range1 = high - low;
            let range2 = (high - prev_close).abs();
            let range3 = (low - prev_close).abs();
            
            let tr = range1.max(range2).max(range3);
            
            results.push((candle_data.open_time[i], tr));
        }

        Ok(results)
    }

    // Calculate Standard Deviation
    pub fn calculate_standard_deviation(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for Standard Deviation calculation"));
        }

        let mut stddev = StandardDeviation::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = stddev.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }
}

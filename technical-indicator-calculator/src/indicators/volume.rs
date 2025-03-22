use crate::database::models::CandleData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::json;
use tracing::debug;

pub struct VolumeCalculator;

impl VolumeCalculator {
    // Calculate OBV (On Balance Volume)
    pub fn calculate_obv(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < 2 {
            return Err(anyhow::anyhow!("Not enough data points for OBV calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());
        let mut obv = 0.0;

        // Skip first candle as we need a previous close for comparison
        for i in 1..candle_data.close.len() {
            let current_close = candle_data.close[i];
            let previous_close = candle_data.close[i - 1];
            let volume = candle_data.volume[i];
            
            // Accumulate OBV based on price direction
            if current_close > previous_close {
                obv += volume;
            } else if current_close < previous_close {
                obv -= volume;
            }
            // If equal, OBV remains unchanged
            
            results.push((candle_data.open_time[i], obv));
        }

        Ok(results)
    }

    // Calculate Accumulation/Distribution Line
    pub fn calculate_ad_line(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < 1 {
            return Err(anyhow::anyhow!("Not enough data points for A/D Line calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());
        let mut ad_line = 0.0;

        for i in 0..candle_data.close.len() {
            let high = candle_data.high[i];
            let low = candle_data.low[i];
            let close = candle_data.close[i];
            let volume = candle_data.volume[i];
            
            // Calculate Money Flow Multiplier
            let range = high - low;
            let mfm = if range > 0.0 {
                ((close - low) - (high - close)) / range
            } else {
                0.0 // Avoid division by zero
            };
            
            // Calculate Money Flow Volume
            let mfv = mfm * volume;
            
            // Accumulate A/D Line
            ad_line += mfv;
            
            results.push((candle_data.open_time[i], ad_line));
        }

        Ok(results)
    }

    // Calculate Chaikin Oscillator
    pub fn calculate_chaikin_oscillator(
        candle_data: &CandleData,
        fast_period: usize,
        slow_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < slow_period {
            return Err(anyhow::anyhow!("Not enough data points for Chaikin Oscillator calculation"));
        }

        // First calculate A/D Line
        let ad_line = Self::calculate_ad_line(candle_data)?;
        
        // We need to calculate EMAs of the A/D Line
        // For simplicity, let's use a naive EMA implementation
        let mut results = Vec::with_capacity(ad_line.len());
        
        // Only proceed if we have enough data
        if ad_line.len() < slow_period {
            return Ok(results);
        }
        
        // Calculate fast and slow EMAs of A/D Line
        let mut fast_ema = Self::calculate_ema(&ad_line, fast_period)?;
        let mut slow_ema = Self::calculate_ema(&ad_line, slow_period)?;
        
        // Trim to the same length (slow EMA will start later)
        let start_index = slow_period - 1;
        if fast_ema.len() > start_index {
            fast_ema = fast_ema[start_index..].to_vec();
        }
        
        // Calculate Chaikin Oscillator = Fast EMA - Slow EMA
        for i in 0..slow_ema.len() {
            if i < fast_ema.len() {
                let time = slow_ema[i].0;
                let value = fast_ema[i].1 - slow_ema[i].1;
                results.push((time, value));
            }
        }

        Ok(results)
    }

    // Calculate Volume Rate of Change
    pub fn calculate_volume_roc(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.volume.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for Volume ROC calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.volume.len() - period);

        for i in period..candle_data.volume.len() {
            let current_volume = candle_data.volume[i];
            let past_volume = candle_data.volume[i - period];
            
            // Avoid division by zero
            if past_volume > 0.0 {
                let roc = ((current_volume - past_volume) / past_volume) * 100.0;
                results.push((candle_data.open_time[i], roc));
            } else {
                // If past volume is zero, default to zero change
                results.push((candle_data.open_time[i], 0.0));
            }
        }

        Ok(results)
    }

    // Calculate Price Volume Trend
    pub fn calculate_price_volume_trend(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < 2 {
            return Err(anyhow::anyhow!("Not enough data points for PVT calculation"));
        }

        let mut pvt = 0.0;  // Initial PVT
        let mut results = Vec::with_capacity(candle_data.close.len() - 1);

        // Skip first candle as we need a previous close for comparison
        for i in 1..candle_data.close.len() {
            let current_close = candle_data.close[i];
            let previous_close = candle_data.close[i - 1];
            let volume = candle_data.volume[i];
            
            // Calculate percentage price change
            if previous_close > 0.0 {
                let price_change = (current_close - previous_close) / previous_close;
                
                // Add volume * price change to PVT
                pvt += volume * price_change;
                
                results.push((candle_data.open_time[i], pvt));
            }
        }

        Ok(results)
    }
    
    // Helper function to calculate EMA
    fn calculate_ema(
        data: &[(DateTime<Utc>, f64)],
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if data.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for EMA calculation"));
        }
        
        let mut results = Vec::with_capacity(data.len());
        let multiplier = 2.0 / (period as f64 + 1.0);
        
        // First EMA value is SMA
        let mut sum = 0.0;
        for i in 0..period {
            sum += data[i].1;
        }
        let mut ema = sum / period as f64;
        
        results.push((data[period - 1].0, ema));
        
        // Calculate EMA for remaining data points
        for i in period..data.len() {
            ema = (data[i].1 - ema) * multiplier + ema;
            results.push((data[i].0, ema));
        }
        
        Ok(results)
    }
}

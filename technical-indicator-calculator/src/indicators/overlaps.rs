use crate::database::models::CandleData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
// Replace ta::indicators with our local implementation
use crate::indicators::ta::indicators;
use crate::indicators::ta::Next;

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

        let mut results = Vec::with_capacity(candle_data.close.len() - period + 1);
        
        // Calculate SMA for each window
        for i in period..=candle_data.close.len() {
            let window_start = i - period;
            let window_end = i;
            
            let sum: f64 = candle_data.close[window_start..window_end].iter().sum();
            let sma = sum / period as f64;
            
            results.push((candle_data.open_time[window_end - 1], sma));
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

        let mut ema = indicators::ExponentialMovingAverage::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = ema.next(candle_data.close[i]);
            
            // Only include values after SMA initialization period
            if i >= period - 1 && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }
    
    // Calculate Bollinger Bands
    pub fn calculate_bollinger_bands(
        candle_data: &CandleData,
        period: usize,
        deviation_up: f64,
        deviation_down: f64,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for Bollinger Bands calculation"));
        }

        // Calculate SMA
        let sma_results = Self::calculate_sma(candle_data, period)?;
        
        // Calculate Standard Deviation
        let mut stddev = indicators::StandardDeviation::new(period)?;
        let mut stddev_values = Vec::with_capacity(candle_data.close.len());
        
        for i in 0..candle_data.close.len() {
            let value = stddev.next(candle_data.close[i]);
            
            if i >= period - 1 && !value.is_nan() {
                stddev_values.push((candle_data.open_time[i], value));
            }
        }
        
        // Ensure we have matching time points
        if sma_results.len() != stddev_values.len() {
            return Err(anyhow::anyhow!("Mismatch in calculated values for Bollinger Bands"));
        }
        
        // Calculate bands
        let mut results = Vec::with_capacity(sma_results.len());
        
        for i in 0..sma_results.len() {
            let (time, sma) = sma_results[i];
            let (_, std_dev) = stddev_values[i];
            
            let upper_band = sma + (std_dev * deviation_up);
            let lower_band = sma - (std_dev * deviation_down);
            
            let bb_value = json!({
                "middle": sma,
                "upper": upper_band,
                "lower": lower_band,
                "width": (upper_band - lower_band) / sma, // Bandwidth
            });
            
            results.push((time, bb_value));
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

        // Calculate first EMA
        let ema_results = Self::calculate_ema(candle_data, period)?;
        
        // Extract EMA values to use as input for second EMA
        let ema_values: Vec<f64> = ema_results.iter().map(|(_, v)| *v).collect();
        
        // Create a temporary CandleData structure for second EMA calculation
        let mut temp_data = CandleData::new(candle_data.symbol.clone(), candle_data.interval.clone());
        
        // Copy timestamps from original data
        for (i, (time, _)) in ema_results.iter().enumerate() {
            temp_data.open_time.push(*time);
            temp_data.close.push(ema_values[i]);
        }
        
        // Calculate EMA of EMA
        let ema_of_ema_results = Self::calculate_ema(&temp_data, period)?;
        
        // Calculate DEMA: 2 * EMA - EMA(EMA)
        let mut results = Vec::with_capacity(ema_of_ema_results.len());
        
        for i in 0..ema_of_ema_results.len() {
            let (time, ema_of_ema) = ema_of_ema_results[i];
            let ema = ema_values[ema_values.len() - ema_of_ema_results.len() + i];
            
            let dema = 2.0 * ema - ema_of_ema;
            results.push((time, dema));
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

        let mut results = Vec::with_capacity(candle_data.close.len() - period + 1);
        
        // Calculate sum of weights: 1+2+3+...+period
        let weight_sum = period * (period + 1) / 2;
        
        // Calculate WMA for each window
        for i in period..=candle_data.close.len() {
            let window_start = i - period;
            let window_end = i;
            
            let mut weighted_sum = 0.0;
            for j in 0..period {
                // More recent prices get higher weights
                let weight = j + 1;
                weighted_sum += candle_data.close[window_start + j] * weight as f64;
            }
            
            let wma = weighted_sum / weight_sum as f64;
            results.push((candle_data.open_time[window_end - 1], wma));
        }

        Ok(results)
    }
    
    // Calculate HMA (Hull Moving Average)
    pub fn calculate_hma(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period * 2 {
            return Err(anyhow::anyhow!("Not enough data points for HMA calculation"));
        }

        // Calculate WMA with period/2
        let half_period = period / 2;
        if half_period == 0 {
            return Err(anyhow::anyhow!("Period is too small for HMA calculation"));
        }
        
        let wma_half = Self::calculate_wma(candle_data, half_period)?;
        
        // Calculate regular WMA
        let wma_full = Self::calculate_wma(candle_data, period)?;
        
        // Create intermediate data: 2*WMA(n/2) - WMA(n)
        let mut temp_data = CandleData::new(candle_data.symbol.clone(), candle_data.interval.clone());
        
        // Align the data (they might have different lengths)
        let min_len = wma_half.len().min(wma_full.len());
        for i in 0..min_len {
            let half_idx = wma_half.len() - min_len + i;
            let full_idx = wma_full.len() - min_len + i;
            
            let (time, half_val) = wma_half[half_idx];
            let (_, full_val) = wma_full[full_idx];
            
            temp_data.open_time.push(time);
            temp_data.close.push(2.0 * half_val - full_val);
        }
        
        // Calculate WMA of sqrt(n) on the intermediate data
        let sqrt_period = (period as f64).sqrt().round() as usize;
        if sqrt_period == 0 {
            return Err(anyhow::anyhow!("Square root of period is too small for HMA calculation"));
        }
        
        // Final HMA is the WMA of intermediate data
        Self::calculate_wma(&temp_data, sqrt_period)
    }
    
    // Calculate VWAP (Volume Weighted Average Price)
    pub fn calculate_vwap(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period {
            return Err(anyhow::anyhow!("Not enough data points for VWAP calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len() - period + 1);
        
        // Calculate VWAP for each rolling window
        for i in period..=candle_data.close.len() {
            let window_start = i - period;
            let window_end = i;
            
            let mut price_volume_sum = 0.0;
            let mut volume_sum = 0.0;
            
            for j in window_start..window_end {
                // Typical price = (High + Low + Close) / 3
                let typical_price = (candle_data.high[j] + candle_data.low[j] + candle_data.close[j]) / 3.0;
                let volume = candle_data.volume[j];
                
                price_volume_sum += typical_price * volume;
                volume_sum += volume;
            }
            
            let vwap = if volume_sum > 0.0 {
                price_volume_sum / volume_sum
            } else {
                // If no volume, use simple average
                let sum: f64 = candle_data.close[window_start..window_end].iter().sum();
                sum / period as f64
            };
            
            results.push((candle_data.open_time[window_end - 1], vwap));
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

        // Calculate first EMA
        let ema1_results = Self::calculate_ema(candle_data, period)?;
        
        // Extract EMA values to use as input for second EMA
        let ema1_values: Vec<f64> = ema1_results.iter().map(|(_, v)| *v).collect();
        
        // Create a temporary CandleData structure for second EMA calculation
        let mut temp_data1 = CandleData::new(candle_data.symbol.clone(), candle_data.interval.clone());
        
        // Copy timestamps and EMA values
        for (i, (time, _)) in ema1_results.iter().enumerate() {
            temp_data1.open_time.push(*time);
            temp_data1.close.push(ema1_values[i]);
        }
        
        // Calculate EMA of EMA
        let ema2_results = Self::calculate_ema(&temp_data1, period)?;
        let ema2_values: Vec<f64> = ema2_results.iter().map(|(_, v)| *v).collect();
        
        // Create a temp data structure for third EMA calculation
        let mut temp_data2 = CandleData::new(candle_data.symbol.clone(), candle_data.interval.clone());
        
        // Copy timestamps and EMA of EMA values
        for (i, (time, _)) in ema2_results.iter().enumerate() {
            temp_data2.open_time.push(*time);
            temp_data2.close.push(ema2_values[i]);
        }
        
        // Calculate EMA of EMA of EMA
        let ema3_results = Self::calculate_ema(&temp_data2, period)?;
        
        // Calculate TEMA: 3*EMA - 3*EMA(EMA) + EMA(EMA(EMA))
        let mut results = Vec::with_capacity(ema3_results.len());
        
        for i in 0..ema3_results.len() {
            let (time, ema3) = ema3_results[i];
            
            // Find corresponding indices in the other EMAs
            let ema1_idx = ema1_values.len() - ema3_results.len() + i;
            let ema2_idx = ema2_values.len() - ema3_results.len() + i;
            
            if ema1_idx < ema1_values.len() && ema2_idx < ema2_values.len() {
                let ema1 = ema1_values[ema1_idx];
                let ema2 = ema2_values[ema2_idx];
                
                let tema = 3.0 * ema1 - 3.0 * ema2 + ema3;
                results.push((time, tema));
            }
        }

        Ok(results)
    }
}

use crate::database::models::CandleData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use ta::indicators::{
    RelativeStrengthIndex, MovingAverageConvergenceDivergence, CommodityChannelIndex,
    MoneyFlowIndex, RateOfChange, PercentagePriceOscillator,
};
use ta::Next;

pub struct OscillatorCalculator;

impl OscillatorCalculator {
    // Calculate RSI (Relative Strength Index)
    pub fn calculate_rsi(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for RSI calculation"));
        }

        let mut rsi = RelativeStrengthIndex::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        // The first `period` values will be NaN, so we'll skip them in the result
        for i in 0..candle_data.close.len() {
            let value = rsi.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate MACD (Moving Average Convergence Divergence)
    pub fn calculate_macd(
        candle_data: &CandleData,
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < slow_period + signal_period {
            return Err(anyhow::anyhow!("Not enough data points for MACD calculation"));
        }

        let mut macd = MovingAverageConvergenceDivergence::new(
            fast_period, slow_period, signal_period,
        )?;
        
        let mut results = Vec::with_capacity(candle_data.close.len());

        // Skip the first `slow_period` values as they'll be unreliable
        for i in 0..candle_data.close.len() {
            let value = macd.next(candle_data.close[i]);
            
            // Only include values that have meaningful MACD values
            if i >= slow_period && !value.macd.is_nan() && !value.signal.is_nan() {
                let macd_value = json!({
                    "macd": value.macd,
                    "signal": value.signal,
                    "histogram": value.histogram,
                });
                
                results.push((candle_data.open_time[i], macd_value));
            }
        }

        Ok(results)
    }

    // Calculate Stochastic Oscillator (custom implementation)
    pub fn calculate_stochastic(
        candle_data: &CandleData,
        k_period: usize,
        k_slowing: usize,
        d_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < k_period + k_slowing + d_period {
            return Err(anyhow::anyhow!("Not enough data points for Stochastic calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());
        
        // Calculate %K for each period
        let mut k_values = Vec::with_capacity(candle_data.close.len());
        
        for i in (k_period - 1)..candle_data.close.len() {
            // Find highest high and lowest low over k_period
            let mut highest_high = f64::NEG_INFINITY;
            let mut lowest_low = f64::INFINITY;
            
            for j in (i - (k_period - 1))..=i {
                highest_high = highest_high.max(candle_data.high[j]);
                lowest_low = lowest_low.min(candle_data.low[j]);
            }
            
            // Calculate %K: (Current Close - Lowest Low) / (Highest High - Lowest Low) * 100
            let range = highest_high - lowest_low;
            let k_value = if range > 0.0 {
                (candle_data.close[i] - lowest_low) / range * 100.0
            } else {
                50.0 // Default to middle value if no range
            };
            
            k_values.push(k_value);
        }
        
        // Apply %K Slowing (simple moving average of %K values)
        let mut slowed_k_values = Vec::with_capacity(k_values.len());
        
        for i in (k_slowing - 1)..k_values.len() {
            let mut sum = 0.0;
            for j in (i - (k_slowing - 1))..=i {
                sum += k_values[j];
            }
            slowed_k_values.push(sum / k_slowing as f64);
        }
        
        // Calculate %D (simple moving average of %K_slowed values)
        for i in (d_period - 1)..slowed_k_values.len() {
            let mut sum = 0.0;
            for j in (i - (d_period - 1))..=i {
                sum += slowed_k_values[j];
            }
            let d_value = sum / d_period as f64;
            
            // Map back to original time index
            let time_index = i + (k_period - 1) + (k_slowing - 1);
            
            let stoch_value = json!({
                "k": slowed_k_values[i],
                "d": d_value,
            });
            
            results.push((candle_data.open_time[time_index], stoch_value));
        }

        Ok(results)
    }

    // Calculate Stochastic RSI (custom implementation)
    pub fn calculate_stoch_rsi(
        candle_data: &CandleData,
        period: usize,
        k_period: usize,
        d_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < period + k_period + d_period {
            return Err(anyhow::anyhow!("Not enough data points for Stochastic RSI calculation"));
        }

        // First calculate RSI
        let rsi_values = Self::calculate_rsi(candle_data, period)?;
        
        if rsi_values.len() < k_period + d_period {
            return Err(anyhow::anyhow!("Not enough RSI values for Stochastic RSI calculation"));
        }
        
        let mut results = Vec::with_capacity(rsi_values.len());
        
        // Extract just the RSI values (without time)
        let rsi_only: Vec<f64> = rsi_values.iter().map(|&(_, value)| value).collect();
        
        // Calculate Stochastic of RSI values
        for i in (k_period - 1)..rsi_only.len() {
            // Find highest high and lowest low RSI over k_period
            let mut highest_high = f64::NEG_INFINITY;
            let mut lowest_low = f64::INFINITY;
            
            for j in (i - (k_period - 1))..=i {
                highest_high = highest_high.max(rsi_only[j]);
                lowest_low = lowest_low.min(rsi_only[j]);
            }
            
            // Calculate %K: (Current RSI - Lowest Low) / (Highest High - Lowest Low) * 100
            let range = highest_high - lowest_low;
            let k_value = if range > 0.0 {
                (rsi_only[i] - lowest_low) / range * 100.0
            } else {
                50.0 // Default to middle value if no range
            };
            
            // Store K values for D calculation
            if i >= k_period + d_period - 2 {
                // Calculate %D (simple moving average of %K values)
                let mut d_sum = 0.0;
                for j in 0..(d_period) {
                    d_sum += (rsi_only[i-j] - lowest_low) / range * 100.0;
                }
                let d_value = d_sum / d_period as f64;
                
                // Get the corresponding time from rsi_values
                let stoch_rsi_value = json!({
                    "k": k_value,
                    "d": d_value,
                });
                
                results.push((rsi_values[i].0, stoch_rsi_value));
            }
        }

        Ok(results)
    }

    // Calculate CCI (Commodity Channel Index)
    pub fn calculate_cci(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for CCI calculation"));
        }

        let mut cci = CommodityChannelIndex::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            // Calculate typical price (high + low + close) / 3
            let typical_price = (candle_data.high[i] + candle_data.low[i] + candle_data.close[i]) / 3.0;
            
            // Pass the typical price to the CCI indicator - fix by passing reference to the value
            let value = cci.next(&typical_price);
            
            // Only include values after the period (initial values are NaN)
            if i >= period && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate MFI (Money Flow Index)
    pub fn calculate_mfi(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for MFI calculation"));
        }

        let mut mfi = MoneyFlowIndex::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            // Calculate typical price
            let typical_price = (candle_data.high[i] + candle_data.low[i] + candle_data.close[i]) / 3.0;
            
            // Calculate money flow (typical price * volume)
            let money_flow = typical_price * candle_data.volume[i];
            
            // Pass the money flow to the MFI indicator - fix by passing reference to the value
            let value = mfi.next(&money_flow);
            
            // Only include values after the period (initial values are NaN)
            if i >= period && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate Ultimate Oscillator (custom implementation)
    pub fn calculate_ultimate_oscillator(
        candle_data: &CandleData,
        short_period: usize,
        medium_period: usize,
        long_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < long_period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for Ultimate Oscillator calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());
        
        // Calculate buying pressure and true range for each candle
        let mut buying_pressure = Vec::with_capacity(candle_data.close.len());
        let mut true_range = Vec::with_capacity(candle_data.close.len());
        
        // Skip first candle as we need the previous close
        for i in 1..candle_data.close.len() {
            let close = candle_data.close[i];
            let prev_close = candle_data.close[i-1];
            let low = candle_data.low[i];
            
            // Buying Pressure = Close - Minimum(Low, Previous Close)
            let bp = close - low.min(prev_close);
            buying_pressure.push(bp);
            
            // True Range = Maximum(High, Previous Close) - Minimum(Low, Previous Close)
            let high = candle_data.high[i];
            let tr = high.max(prev_close) - low.min(prev_close);
            true_range.push(tr);
        }
        
        // Calculate the three averages (short, medium, long) starting from index long_period
        for i in long_period..candle_data.close.len() {
            // Adjust indices for buying_pressure and true_range (which start at index 1)
            let adj_i = i - 1;
            
            // Calculate average for short period
            let mut short_bp_sum = 0.0;
            let mut short_tr_sum = 0.0;
            for j in 0..short_period {
                short_bp_sum += buying_pressure[adj_i - j];
                short_tr_sum += true_range[adj_i - j];
            }
            let avg1 = if short_tr_sum > 0.0 { short_bp_sum / short_tr_sum } else { 0.0 };
            
            // Calculate average for medium period
            let mut medium_bp_sum = 0.0;
            let mut medium_tr_sum = 0.0;
            for j in 0..medium_period {
                medium_bp_sum += buying_pressure[adj_i - j];
                medium_tr_sum += true_range[adj_i - j];
            }
            let avg2 = if medium_tr_sum > 0.0 { medium_bp_sum / medium_tr_sum } else { 0.0 };
            
            // Calculate average for long period
            let mut long_bp_sum = 0.0;
            let mut long_tr_sum = 0.0;
            for j in 0..long_period {
                long_bp_sum += buying_pressure[adj_i - j];
                long_tr_sum += true_range[adj_i - j];
            }
            let avg3 = if long_tr_sum > 0.0 { long_bp_sum / long_tr_sum } else { 0.0 };
            
            // Ultimate Oscillator formula with typical weights (4, 2, 1)
            let uo = (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0 * 100.0;
            
            results.push((candle_data.open_time[i], uo));
        }

        Ok(results)
    }

    // Calculate Williams %R (custom implementation)
    pub fn calculate_williams_r(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for Williams %R calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in (period - 1)..candle_data.close.len() {
            // Find highest high and lowest low over period
            let mut highest_high = f64::NEG_INFINITY;
            let mut lowest_low = f64::INFINITY;
            
            for j in (i - (period - 1))..=i {
                highest_high = highest_high.max(candle_data.high[j]);
                lowest_low = lowest_low.min(candle_data.low[j]);
            }
            
            // Calculate Williams %R: (Highest High - Close) / (Highest High - Lowest Low) * -100
            let range = highest_high - lowest_low;
            let w_r = if range > 0.0 {
                (highest_high - candle_data.close[i]) / range * -100.0
            } else {
                -50.0 // Default to middle value if no range
            };
            
            results.push((candle_data.open_time[i], w_r));
        }

        Ok(results)
    }

    // Calculate Momentum
    pub fn calculate_momentum(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for Momentum calculation"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len() - period);

        // Calculate momentum: Current Price - Price N periods ago
        for i in period..candle_data.close.len() {
            let momentum = candle_data.close[i] - candle_data.close[i - period];
            results.push((candle_data.open_time[i], momentum));
        }

        Ok(results)
    }

    // Calculate Rate of Change (ROC)
    pub fn calculate_roc(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, f64)>> {
        if candle_data.close.len() < period + 1 {
            return Err(anyhow::anyhow!("Not enough data points for ROC calculation"));
        }

        let mut roc = RateOfChange::new(period)?;
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = roc.next(candle_data.close[i]);
            
            // Only include values after the period (initial values are NaN)
            if i >= period && !value.is_nan() {
                results.push((candle_data.open_time[i], value));
            }
        }

        Ok(results)
    }

    // Calculate Percentage Price Oscillator (PPO)
    pub fn calculate_ppo(
        candle_data: &CandleData,
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < slow_period + signal_period {
            return Err(anyhow::anyhow!("Not enough data points for PPO calculation"));
        }

        let mut ppo = PercentagePriceOscillator::new(
            fast_period, slow_period, signal_period,
        )?;
        
        let mut results = Vec::with_capacity(candle_data.close.len());

        for i in 0..candle_data.close.len() {
            let value = ppo.next(candle_data.close[i]);
            
            // Only include values after sufficient data is processed
            if i >= slow_period && !value.ppo.is_nan() && !value.signal.is_nan() {
                let ppo_value = json!({
                    "ppo": value.ppo,
                    "signal": value.signal,
                    "histogram": value.histogram,
                });
                
                results.push((candle_data.open_time[i], ppo_value));
            }
        }

        Ok(results)
    }
}

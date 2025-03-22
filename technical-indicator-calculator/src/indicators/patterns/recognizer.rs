use crate::database::models::CandleData;
use crate::indicators::patterns::{single_candle, double_candle, triple_candle};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use tracing::{debug, error, info};

pub struct PatternRecognizer;

impl PatternRecognizer {
    // Calculate all candlestick patterns at once
    pub fn calculate_all_patterns(
        candle_data: &CandleData,
        penetration: f64, // Penetration factor for some patterns (e.g., 0.5 for Dark Cloud Cover)
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.len() < 5 {  // Most patterns need at least 2-5 candles
            return Err(anyhow::anyhow!("Not enough data points for pattern recognition"));
        }

        let mut results = Vec::with_capacity(candle_data.close.len());

        // Start from index where we have enough previous candles for all patterns
        for i in 4..candle_data.close.len() {
            let mut patterns = json!({});
            let time = candle_data.open_time[i];
            
            // Single candle patterns
            single_candle::check_doji(candle_data, i, &mut patterns)?;
            single_candle::check_hammer(candle_data, i, &mut patterns)?;
            single_candle::check_inverted_hammer(candle_data, i, &mut patterns)?;
            single_candle::check_spinning_top(candle_data, i, &mut patterns)?;
            
            // Double candle patterns
            double_candle::check_engulfing(candle_data, i, &mut patterns)?;
            double_candle::check_harami(candle_data, i, &mut patterns)?;
            double_candle::check_piercing_line(candle_data, i, penetration, &mut patterns)?;
            double_candle::check_dark_cloud_cover(candle_data, i, penetration, &mut patterns)?;
            
            // Triple candle patterns
            triple_candle::check_morning_star(candle_data, i, &mut patterns)?;
            triple_candle::check_evening_star(candle_data, i, &mut patterns)?;
            triple_candle::check_three_white_soldiers(candle_data, i, &mut patterns)?;
            triple_candle::check_three_black_crows(candle_data, i, &mut patterns)?;
            
            // Only add to results if at least one pattern was detected
            if patterns.as_object().unwrap().len() > 0 {
                results.push((time, patterns));
            }
        }

        Ok(results)
    }
}

use crate::database::models::CandleData;
use crate::indicators::patterns::utils::*;
use anyhow::Result;
use serde_json::{json, Value};

// Check for Engulfing pattern
pub fn check_engulfing(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 1 {
        return Ok(());
    }
    
    let curr_open = candle_data.open[index];
    let curr_close = candle_data.close[index];
    let prev_open = candle_data.open[index-1];
    let prev_close = candle_data.close[index-1];
    
    let curr_body_size = body_size(curr_open, curr_close);
    let prev_body_size = body_size(prev_open, prev_close);
    
    let current_is_bullish = is_bullish(curr_open, curr_close);
    let previous_is_bearish = is_bearish(prev_open, prev_close);
    
    // Bullish Engulfing
    if current_is_bullish && previous_is_bearish &&
       curr_open < prev_close && curr_close > prev_open {
        
        // Stronger if current body much larger than previous
        let relative_size = curr_body_size / prev_body_size;
        let strength = 0.7 + (relative_size - 1.0).min(0.3); // Cap at 1.0
        
        patterns["engulfing"] = json!({
            "type": "bullish",
            "strength": strength,
        });
    }
    
    // Bearish Engulfing
    let current_is_bearish = is_bearish(curr_open, curr_close);
    let previous_is_bullish = is_bullish(prev_open, prev_close);
    
    if current_is_bearish && previous_is_bullish &&
       curr_open > prev_close && curr_close < prev_open {
        
        // Stronger if current body much larger than previous
        let relative_size = curr_body_size / prev_body_size;
        let strength = 0.7 + (relative_size - 1.0).min(0.3); // Cap at 1.0
        
        patterns["engulfing"] = json!({
            "type": "bearish",
            "strength": strength,
        });
    }
    
    Ok(())
}

// Check for Harami pattern
pub fn check_harami(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 1 {
        return Ok(());
    }
    
    let curr_open = candle_data.open[index];
    let curr_close = candle_data.close[index];
    let prev_open = candle_data.open[index-1];
    let prev_close = candle_data.close[index-1];
    
    let curr_body_size = body_size(curr_open, curr_close);
    let prev_body_size = body_size(prev_open, prev_close);
    
    // Harami requires current body to be contained within previous body
    if curr_body_size < prev_body_size {
        let current_is_bullish = is_bullish(curr_open, curr_close);
        let previous_is_bearish = is_bearish(prev_open, prev_close);
        
        // Bullish Harami
        if current_is_bullish && previous_is_bearish &&
           curr_open > prev_close && curr_close < prev_open {
            
            let containment_ratio = curr_body_size / prev_body_size;
            let strength = 0.7 + (1.0 - containment_ratio) * 0.3; // Higher for smaller bodies
            
            patterns["harami"] = json!({
                "type": "bullish",
                "strength": strength,
            });
        }
        
        // Bearish Harami
        let current_is_bearish = is_bearish(curr_open, curr_close);
        let previous_is_bullish = is_bullish(prev_open, prev_close);
        
        if current_is_bearish && previous_is_bullish &&
           curr_open < prev_close && curr_close > prev_open {
            
            let containment_ratio = curr_body_size / prev_body_size;
            let strength = 0.7 + (1.0 - containment_ratio) * 0.3; // Higher for smaller bodies
            
            patterns["harami"] = json!({
                "type": "bearish",
                "strength": strength,
            });
        }
    }
    
    Ok(())
}

// Check for Piercing Line pattern
pub fn check_piercing_line(
    candle_data: &CandleData,
    index: usize,
    penetration: f64,
    patterns: &mut Value,
) -> Result<()> {
    if index < 1 {
        return Ok(());
    }
    
    let curr_open = candle_data.open[index];
    let curr_close = candle_data.close[index];
    let prev_open = candle_data.open[index-1];
    let prev_close = candle_data.close[index-1];
    let prev_low = candle_data.low[index-1];
    
    // Check for downtrend
    let has_downtrend = has_downtrend(candle_data, index, 3);
    
    // Piercing line criteria:
    // 1. First day is a bearish candle (close < open)
    // 2. Second day opens below first day's low
    // 3. Second day closes more than halfway up first day's body
    // 4. Market is in a downtrend
    
    let previous_is_bearish = is_bearish(prev_open, prev_close);
    let current_is_bullish = is_bullish(curr_open, curr_close);
    
    if has_downtrend && previous_is_bearish && current_is_bullish && 
       curr_open < prev_low {
        
        let prev_body_size = prev_open - prev_close;
        let penetration_point = prev_close + (prev_body_size * penetration);
        
        if curr_close > penetration_point {
            // Calculate actual penetration percentage
            let actual_penetration = (curr_close - prev_close) / prev_body_size;
            
            // Strength based on how far into the body it penetrates
            let strength = 0.7 + (actual_penetration - penetration) * (0.3 / (1.0 - penetration));
            
            patterns["piercing_line"] = json!({
                "type": "bullish",
                "strength": strength,
            });
        }
    }
    
    Ok(())
}

// Check for Dark Cloud Cover pattern
pub fn check_dark_cloud_cover(
    candle_data: &CandleData,
    index: usize,
    penetration: f64,
    patterns: &mut Value,
) -> Result<()> {
    if index < 1 {
        return Ok(());
    }
    
    let curr_open = candle_data.open[index];
    let curr_close = candle_data.close[index];
    let prev_open = candle_data.open[index-1];
    let prev_close = candle_data.close[index-1];
    let prev_high = candle_data.high[index-1];
    
    // Check for uptrend
    let has_uptrend = has_uptrend(candle_data, index, 3);
    
    // Dark Cloud Cover criteria:
    // 1. First day is a bullish candle (close > open)
    // 2. Second day opens above first day's high
    // 3. Second day closes more than halfway down first day's body
    // 4. Market is in an uptrend
    
    let previous_is_bullish = is_bullish(prev_open, prev_close);
    let current_is_bearish = is_bearish(curr_open, curr_close);
    
    if has_uptrend && previous_is_bullish && current_is_bearish && 
       curr_open > prev_high {
        
        let prev_body_size = prev_close - prev_open;
        let penetration_point = prev_close - (prev_body_size * penetration);
        
        if curr_close < penetration_point {
            // Calculate actual penetration percentage
            let actual_penetration = (prev_close - curr_close) / prev_body_size;
            
            // Strength based on how far into the body it penetrates
            let strength = 0.7 + (actual_penetration - penetration) * (0.3 / (1.0 - penetration));
            
            patterns["dark_cloud_cover"] = json!({
                "type": "bearish",
                "strength": strength,
            });
        }
    }
    
    Ok(())
}

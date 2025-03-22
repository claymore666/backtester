use crate::database::models::CandleData;
use crate::indicators::patterns::utils::*;
use anyhow::Result;
use serde_json::{json, Value};

// Check for Doji pattern
pub fn check_doji(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    let open = candle_data.open[index];
    let close = candle_data.close[index];
    let high = candle_data.high[index];
    let low = candle_data.low[index];
    
    // Calculate body size relative to range
    let body_size = (open - close).abs();
    let range = high - low;
    
    // If range is zero, avoid division by zero
    if range == 0.0 {
        return Ok(());
    }
    
    let body_to_range_ratio = body_size / range;
    
    // Doji has very small body compared to range
    if body_to_range_ratio < 0.1 {
        patterns["doji"] = json!({
            "type": "neutral",
            "strength": 1.0 - body_to_range_ratio * 10.0, // Higher for smaller bodies
        });
    }
    
    Ok(())
}

// Check for Hammer pattern
pub fn check_hammer(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    let open = candle_data.open[index];
    let close = candle_data.close[index];
    let high = candle_data.high[index];
    let low = candle_data.low[index];
    
    let body_size = (open - close).abs();
    let range = high - low;
    
    if range == 0.0 {
        return Ok(());
    }
    
    let body_to_range_ratio = body_size / range;
    let is_bullish = close > open;
    
    // Check for downtrend in previous candles
    let has_downtrend = index >= 3 && 
        candle_data.close[index-3] > candle_data.close[index-2] && 
        candle_data.close[index-2] > candle_data.close[index-1];
    
    // Criteria for hammer:
    // 1. Small body (less than 1/3 of total range)
    // 2. Long lower shadow (at least 2x body size)
    // 3. Little or no upper shadow
    // 4. In a downtrend
    
    let upper_shadow = upper_shadow(high, open, close);
    let lower_shadow = lower_shadow(low, open, close);
    
    let upper_shadow_ratio = upper_shadow / range;
    let lower_shadow_ratio = lower_shadow / range;
    
    if body_to_range_ratio < 0.33 && // Small body
       lower_shadow_ratio > 0.66 &&  // Long lower shadow
       upper_shadow_ratio < 0.1 &&   // Small upper shadow
       has_downtrend {               // In a downtrend
        
        patterns["hammer"] = json!({
            "type": "bullish",
            "strength": 0.7 + (lower_shadow_ratio - 0.66) * 0.9, // Higher for longer lower shadows
        });
    }
    
    Ok(())
}

// Check for Inverted Hammer pattern
pub fn check_inverted_hammer(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    let open = candle_data.open[index];
    let close = candle_data.close[index];
    let high = candle_data.high[index];
    let low = candle_data.low[index];
    
    let body_size = (open - close).abs();
    let range = high - low;
    
    if range == 0.0 {
        return Ok(());
    }
    
    let body_to_range_ratio = body_size / range;
    let is_bullish = close > open;
    
    // Check for downtrend in previous candles
    let has_downtrend = index >= 3 && 
        candle_data.close[index-3] > candle_data.close[index-2] && 
        candle_data.close[index-2] > candle_data.close[index-1];
    
    // Criteria for inverted hammer:
    // 1. Small body (less than 1/3 of total range)
    // 2. Long upper shadow (at least 2x body size)
    // 3. Little or no lower shadow
    // 4. In a downtrend
    
    let upper_shadow = upper_shadow(high, open, close);
    let lower_shadow = lower_shadow(low, open, close);
    
    let upper_shadow_ratio = upper_shadow / range;
    let lower_shadow_ratio = lower_shadow / range;
    
    if body_to_range_ratio < 0.33 && // Small body
       upper_shadow_ratio > 0.66 &&  // Long upper shadow
       lower_shadow_ratio < 0.1 &&   // Small lower shadow
       has_downtrend {               // In a downtrend
        
        patterns["inverted_hammer"] = json!({
            "type": "bullish",
            "strength": 0.7 + (upper_shadow_ratio - 0.66) * 0.9, // Higher for longer upper shadows
        });
    }
    
    Ok(())
}

// Check for Spinning Top pattern
pub fn check_spinning_top(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    let open = candle_data.open[index];
    let close = candle_data.close[index];
    let high = candle_data.high[index];
    let low = candle_data.low[index];
    
    let body_size = (open - close).abs();
    let range = high - low;
    
    if range == 0.0 {
        return Ok(());
    }
    
    let body_to_range_ratio = body_size / range;
    let is_bullish = close > open;
    
    // Criteria for spinning top:
    // 1. Small body in the middle (less than 1/3 of total range)
    // 2. Upper and lower shadows of similar length
    
    let upper_shadow = upper_shadow(high, open, close);
    let lower_shadow = lower_shadow(low, open, close);
    
    let upper_shadow_ratio = upper_shadow / range;
    let lower_shadow_ratio = lower_shadow / range;
    
    // Shadows should be roughly equal and body should be small
    if body_to_range_ratio < 0.33 && // Small body
       upper_shadow_ratio > 0.25 &&  // Substantial upper shadow
       lower_shadow_ratio > 0.25 &&  // Substantial lower shadow
       (upper_shadow_ratio / lower_shadow_ratio).abs() < 2.0 { // Shadows roughly equal
        
        // Pattern type depends on trend
        let trend_up = index >= 3 && 
            candle_data.close[index-3] < candle_data.close[index-2] && 
            candle_data.close[index-2] < candle_data.close[index-1];
            
        let trend_down = index >= 3 && 
            candle_data.close[index-3] > candle_data.close[index-2] && 
            candle_data.close[index-2] > candle_data.close[index-1];
        
        let pattern_type = if trend_up {
            "bearish"
        } else if trend_down {
            "bullish"
        } else {
            "neutral"
        };
        
        patterns["spinning_top"] = json!({
            "type": pattern_type,
            "strength": 0.7 + (1.0 - body_to_range_ratio) * 0.3, // Higher for smaller bodies
        });
    }
    
    Ok(())
}

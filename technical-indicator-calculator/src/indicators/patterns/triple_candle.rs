use crate::database::models::CandleData;
use crate::indicators::patterns::utils::*;
use anyhow::Result;
use serde_json::{json, Value};

// Check for Morning Star pattern
pub fn check_morning_star(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 2 {
        return Ok(());
    }
    
    let first_open = candle_data.open[index-2];
    let first_close = candle_data.close[index-2];
    let second_open = candle_data.open[index-1];
    let second_close = candle_data.close[index-1];
    let third_open = candle_data.open[index];
    let third_close = candle_data.close[index];
    
    // Check for downtrend
    let has_downtrend = has_downtrend(candle_data, index-2, 3);
    
    // Morning Star criteria:
    // 1. First day is a large bearish candle (close < open)
    // 2. Second day is a small-bodied candle (star) that gaps down
    // 3. Third day is a bullish candle that closes well into the first candle's body
    // 4. Market is in a downtrend
    
    let first_is_bearish = is_bearish(first_open, first_close);
    let third_is_bullish = is_bullish(third_open, third_close);
    
    let first_body_size = body_size(first_open, first_close);
    let second_body_size = body_size(second_open, second_close);
    let third_body_size = body_size(third_open, third_close);
    
    // Star should have a small body
    let second_is_small = second_body_size < 0.3 * first_body_size;
    
    // Star should gap down from first candle
    let gaps_down = second_open < first_close && second_close < first_close;
    
    // Third candle should close into first candle's body
    let penetration = (third_close - first_close) / first_body_size;
    let closes_into_first = penetration > 0.3; // Closes at least 30% into first candle
    
    if has_downtrend && first_is_bearish && second_is_small && 
       third_is_bullish && gaps_down && closes_into_first {
        
        // Strength depends on third candle's penetration into first
        let strength = 0.7 + penetration * 0.3;
        
        patterns["morning_star"] = json!({
            "type": "bullish",
            "strength": strength,
        });
    }
    
    Ok(())
}

// Check for Evening Star pattern
pub fn check_evening_star(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 2 {
        return Ok(());
    }
    
    let first_open = candle_data.open[index-2];
    let first_close = candle_data.close[index-2];
    let second_open = candle_data.open[index-1];
    let second_close = candle_data.close[index-1];
    let third_open = candle_data.open[index];
    let third_close = candle_data.close[index];
    
    // Check for uptrend
    let has_uptrend = has_uptrend(candle_data, index-2, 3);
    
    // Evening Star criteria:
    // 1. First day is a large bullish candle (close > open)
    // 2. Second day is a small-bodied candle (star) that gaps up
    // 3. Third day is a bearish candle that closes well into the first candle's body
    // 4. Market is in an uptrend
    
    let first_is_bullish = is_bullish(first_open, first_close);
    let third_is_bearish = is_bearish(third_open, third_close);
    
    let first_body_size = body_size(first_open, first_close);
    let second_body_size = body_size(second_open, second_close);
    let third_body_size = body_size(third_open, third_close);
    
    // Star should have a small body
    let second_is_small = second_body_size < 0.3 * first_body_size;
    
    // Star should gap up from first candle
    let gaps_up = second_open > first_close && second_close > first_close;
    
    // Third candle should close into first candle's body
    let penetration = (first_close - third_close) / first_body_size;
    let closes_into_first = penetration > 0.3; // Closes at least 30% into first candle
    
    if has_uptrend && first_is_bullish && second_is_small && 
       third_is_bearish && gaps_up && closes_into_first {
        
        // Strength depends on third candle's penetration into first
        let strength = 0.7 + penetration * 0.3;
        
        patterns["evening_star"] = json!({
            "type": "bearish",
            "strength": strength,
        });
    }
    
    Ok(())
}

// Check for Three White Soldiers pattern
pub fn check_three_white_soldiers(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 2 {
        return Ok(());
    }
    
    // Check for downtrend before the pattern
    let has_downtrend = has_downtrend(candle_data, index-2, 3);
    
    // Three White Soldiers criteria:
    // 1. Three consecutive bullish candles
    // 2. Each candle closes higher than the previous
    // 3. Each candle opens within the previous candle's body
    // 4. Each candle has relatively small upper shadows
    // 5. Market is in a downtrend before the pattern
    
    let first_bullish = is_bullish(candle_data.open[index-2], candle_data.close[index-2]);
    let second_bullish = is_bullish(candle_data.open[index-1], candle_data.close[index-1]);
    let third_bullish = is_bullish(candle_data.open[index], candle_data.close[index]);
    
    let closes_higher = candle_data.close[index-2] < candle_data.close[index-1] && 
                        candle_data.close[index-1] < candle_data.close[index];
    
    let opens_within_prev = candle_data.open[index-1] > candle_data.open[index-2] && 
                           candle_data.open[index-1] < candle_data.close[index-2] &&
                           candle_data.open[index] > candle_data.open[index-1] && 
                           candle_data.open[index] < candle_data.close[index-1];
    
    // Calculate upper shadows
    let first_upper = upper_shadow(candle_data.high[index-2], candle_data.open[index-2], candle_data.close[index-2]);
    let second_upper = upper_shadow(candle_data.high[index-1], candle_data.open[index-1], candle_data.close[index-1]);
    let third_upper = upper_shadow(candle_data.high[index], candle_data.open[index], candle_data.close[index]);
    
    let first_body = body_size(candle_data.open[index-2], candle_data.close[index-2]);
    let second_body = body_size(candle_data.open[index-1], candle_data.close[index-1]);
    let third_body = body_size(candle_data.open[index], candle_data.close[index]);
    
    let small_upper_shadows = first_upper < 0.3 * first_body && 
                             second_upper < 0.3 * second_body &&
                             third_upper < 0.3 * third_body;
    
    if has_downtrend && first_bullish && second_bullish && third_bullish && 
       closes_higher && opens_within_prev && small_upper_shadows {
        
        // Calculate pattern strength based on candle sizes and progression
        let size_increase = (second_body > first_body && third_body > second_body) as i32;
        let strength = 0.7 + (size_increase as f64) * 0.15; // Bonus for increasing size
        
        patterns["three_white_soldiers"] = json!({
            "type": "bullish",
            "strength": strength,
        });
    }
    
    Ok(())
}

// Check for Three Black Crows pattern
pub fn check_three_black_crows(
    candle_data: &CandleData,
    index: usize,
    patterns: &mut Value,
) -> Result<()> {
    if index < 2 {
        return Ok(());
    }
    
    // Check for uptrend before the pattern
    let has_uptrend = has_uptrend(candle_data, index-2, 3);
    
    // Three Black Crows criteria:
    // 1. Three consecutive bearish candles
    // 2. Each candle closes lower than the previous
    // 3. Each candle opens within the previous candle's body
    // 4. Each candle has relatively small lower shadows
    // 5. Market is in an uptrend before the pattern
    
    let first_bearish = is_bearish(candle_data.open[index-2], candle_data.close[index-2]);
    let second_bearish = is_bearish(candle_data.open[index-1], candle_data.close[index-1]);
    let third_bearish = is_bearish(candle_data.open[index], candle_data.close[index]);
    
    let closes_lower = candle_data.close[index-2] > candle_data.close[index-1] && 
                       candle_data.close[index-1] > candle_data.close[index];
    
    let opens_within_prev = candle_data.open[index-1] < candle_data.open[index-2] && 
                           candle_data.open[index-1] > candle_data.close[index-2] &&
                           candle_data.open[index] < candle_data.open[index-1] && 
                           candle_data.open[index] > candle_data.close[index-1];
    
    // Calculate lower shadows
    let first_lower = lower_shadow(candle_data.low[index-2], candle_data.open[index-2], candle_data.close[index-2]);
    let second_lower = lower_shadow(candle_data.low[index-1], candle_data.open[index-1], candle_data.close[index-1]);
    let third_lower = lower_shadow(candle_data.low[index], candle_data.open[index], candle_data.close[index]);
    
    let first_body = body_size(candle_data.open[index-2], candle_data.close[index-2]);
    let second_body = body_size(candle_data.open[index-1], candle_data.close[index-1]);
    let third_body = body_size(candle_data.open[index], candle_data.close[index]);
    
    let small_lower_shadows = first_lower < 0.3 * first_body && 
                             second_lower < 0.3 * second_body &&
                             third_lower < 0.3 * third_body;
    
    if has_uptrend && first_bearish && second_bearish && third_bearish && 
       closes_lower && opens_within_prev && small_lower_shadows {
        
        // Calculate pattern strength based on candle sizes and progression
        let size_increase = (second_body > first_body && third_body > second_body) as i32;
        let strength = 0.7 + (size_increase as f64) * 0.15; // Bonus for increasing size
        
        patterns["three_black_crows"] = json!({
            "type": "bearish",
            "strength": strength,
        });
    }
    
    Ok(())
}

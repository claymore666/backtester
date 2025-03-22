use crate::database::models::CandleData;

// Define pattern recognition strength thresholds
pub const PATTERN_STRENGTH_THRESHOLD: f64 = 0.7; // Minimum strength to consider a pattern valid

// Helper functions for pattern recognition
pub fn is_bullish(open: f64, close: f64) -> bool {
    close > open
}

pub fn is_bearish(open: f64, close: f64) -> bool {
    close < open
}

pub fn body_size(open: f64, close: f64) -> f64 {
    (open - close).abs()
}

pub fn upper_shadow(high: f64, open: f64, close: f64) -> f64 {
    if is_bullish(open, close) {
        high - close
    } else {
        high - open
    }
}

pub fn lower_shadow(low: f64, open: f64, close: f64) -> f64 {
    if is_bullish(open, close) {
        open - low
    } else {
        close - low
    }
}

pub fn has_uptrend(candle_data: &CandleData, index: usize, periods: usize) -> bool {
    if index < periods {
        return false;
    }
    
    for i in 1..periods {
        if candle_data.close[index-i] < candle_data.close[index-(i+1)] {
            return false;
        }
    }
    
    true
}

pub fn has_downtrend(candle_data: &CandleData, index: usize, periods: usize) -> bool {
    if index < periods {
        return false;
    }
    
    for i in 1..periods {
        if candle_data.close[index-i] > candle_data.close[index-(i+1)] {
            return false;
        }
    }
    
    true
}

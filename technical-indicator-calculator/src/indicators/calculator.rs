use crate::database::models::CandleData;
use crate::talib_bindings::TaLibAbstract;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use tracing::{debug, warn};

pub struct IndicatorCalculator;

// Map serde_json::Value parameter to (name, value) pairs for TA-Lib
fn extract_parameters(params: &Value) -> Vec<(String, Value)> {
    if let Value::Object(map) = params {
        return map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }
    vec![]
}

impl IndicatorCalculator {
    // Generic function to calculate any indicator using TA-Lib abstract API
    pub fn calculate_indicator(
        candle_data: &CandleData,
        indicator_name: &str,
        parameters: &Value,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        if candle_data.close.is_empty() {
            return Err(anyhow::anyhow!("No candle data available"));
        }

        // Extract parameters for TA-Lib
        let params = extract_parameters(parameters);
        
        // Get TA-Lib function name
        let func_name = TaLibAbstract::get_function_name(indicator_name);

        debug!("Calculating indicator '{}' with function '{}' and parameters: {:?}", 
               indicator_name, func_name, params);

        // Call TA-Lib function
        let results = TaLibAbstract::call_function(
            &func_name,
            Some(&candle_data.open),
            Some(&candle_data.high),
            Some(&candle_data.low),
            Some(&candle_data.close),
            Some(&candle_data.volume),
            &params,
        ).context(format!("Failed to calculate indicator {}", indicator_name))?;

        // Convert results to (DateTime, Value) pairs
        let value_results = results
            .into_iter()
            .map(|(idx, value)| {
                if idx < candle_data.open_time.len() {
                    (candle_data.open_time[idx], value)
                } else {
                    // This should not happen if TA-Lib is working correctly
                    warn!("Index out of bounds: {} >= {}", idx, candle_data.open_time.len());
                    (
                        candle_data.open_time[candle_data.open_time.len() - 1],
                        value,
                    )
                }
            })
            .collect();

        Ok(value_results)
    }

    // For specific indicator types with multiple outputs, we would need specialized functions
    // Example for MACD which returns three values (MACD, Signal, Histogram)
    pub fn calculate_macd(
        candle_data: &CandleData,
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for MACD
        let parameters = json!({
            "fast_period": fast_period,
            "slow_period": slow_period,
            "signal_period": signal_period
        });

        // Call the generic calculate_indicator function with MACD parameters
        Self::calculate_indicator(candle_data, "MACD", &parameters)
    }

    // Calculate Bollinger Bands
    #[allow(dead_code)]
    pub fn calculate_bollinger_bands(
        candle_data: &CandleData,
        period: usize,
        deviation_up: f64,
        deviation_down: f64,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for Bollinger Bands
        let parameters = json!({
            "period": period,
            "deviation_up": deviation_up,
            "deviation_down": deviation_down,
            "ma_type": 0  // SMA
        });

        // Call the generic calculate_indicator function with BBANDS parameters
        Self::calculate_indicator(candle_data, "BBANDS", &parameters)
    }

    // Calculate Stochastic
    #[allow(dead_code)]
    pub fn calculate_stochastic(
        candle_data: &CandleData,
        k_period: usize,
        slowing: usize,
        d_period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for Stochastic
        let parameters = json!({
            "k_period": k_period,
            "slowing": slowing,
            "d_period": d_period,
            "ma_type": 0  // SMA
        });

        // Call the generic calculate_indicator function with STOCH parameters
        Self::calculate_indicator(candle_data, "STOCH", &parameters)
    }

    // Calculate RSI
    #[allow(dead_code)]
    pub fn calculate_rsi(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for RSI
        let parameters = json!({
            "period": period
        });

        // Call the generic calculate_indicator function with RSI parameters
        Self::calculate_indicator(candle_data, "RSI", &parameters)
    }

    // Calculate ATR
    #[allow(dead_code)]
    pub fn calculate_atr(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for ATR
        let parameters = json!({
            "period": period
        });

        // Call the generic calculate_indicator function with ATR parameters
        Self::calculate_indicator(candle_data, "ATR", &parameters)
    }

    // Calculate OBV
    #[allow(dead_code)]
    pub fn calculate_obv(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // OBV doesn't need any parameters
        let parameters = json!({});

        // Call the generic calculate_indicator function with OBV parameters
        Self::calculate_indicator(candle_data, "OBV", &parameters)
    }

    // Calculate ADX
    #[allow(dead_code)]
    pub fn calculate_adx(
        candle_data: &CandleData,
        period: usize,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Parameters for ADX
        let parameters = json!({
            "period": period
        });

        // Call the generic calculate_indicator function with ADX parameters
        Self::calculate_indicator(candle_data, "ADX", &parameters)
    }

    // Calculate candlestick pattern (Engulfing)
    #[allow(dead_code)]
    pub fn calculate_engulfing(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // No parameters needed for candlestick patterns
        let parameters = json!({});

        // Call the generic calculate_indicator function
        Self::calculate_indicator(candle_data, "CDLENGULFING", &parameters)
    }

    // Calculate candlestick pattern (Hammer)
    #[allow(dead_code)]
    pub fn calculate_hammer(
        candle_data: &CandleData,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // No parameters needed for candlestick patterns
        let parameters = json!({});

        // Call the generic calculate_indicator function
        Self::calculate_indicator(candle_data, "CDLHAMMER", &parameters)
    }

    // Calculate candlestick pattern (Morning Star)
    #[allow(dead_code)]
    pub fn calculate_morning_star(
        candle_data: &CandleData,
        penetration: f64,
    ) -> Result<Vec<(DateTime<Utc>, Value)>> {
        // Penetration parameter for morning star pattern
        let parameters = json!({
            "penetration": penetration
        });

        // Call the generic calculate_indicator function
        Self::calculate_indicator(candle_data, "CDLMORNINGSTAR", &parameters)
    }

    // Function to map indicator types to their TA-Lib function names
    pub fn get_ta_function_name(indicator_name: &str) -> String {
        TaLibAbstract::get_function_name(indicator_name)
    }

    // Function to check if an indicator is available
    #[allow(dead_code)]
    pub fn is_indicator_available(indicator_name: &str) -> bool {
        TaLibAbstract::is_function_available(&TaLibAbstract::get_function_name(indicator_name))
    }

    // Get a list of all supported indicators
    #[allow(dead_code)]
    pub fn get_supported_indicators() -> Vec<String> {
        vec![
            "RSI".to_string(),
            "SMA".to_string(),
            "EMA".to_string(),
            "MACD".to_string(),
            "BBANDS".to_string(),
            "ATR".to_string(),
            "STOCH".to_string(),
            "ADX".to_string(),
            "OBV".to_string(),
            "CDLENGULFING".to_string(),
            "CDLHAMMER".to_string(),
            "CDLMORNINGSTAR".to_string(),
        ]
    }
}

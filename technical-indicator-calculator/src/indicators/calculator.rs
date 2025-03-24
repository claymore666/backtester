use crate::database::models::CandleData;
use crate::talib_bindings::TaLibAbstract;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

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
                    (candle_data.open_time[idx], json!(value))
                } else {
                    // This should not happen if TA-Lib is working correctly
                    (
                        candle_data.open_time[candle_data.open_time.len() - 1],
                        json!(value),
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
            "optInFastPeriod": fast_period,
            "optInSlowPeriod": slow_period,
            "optInSignalPeriod": signal_period
        });

        // For MACD we need to make 3 separate calls to get all outputs
        // This is a simplification - in a complete implementation, we would
        // modify the TaLibAbstract to handle multiple outputs
        
        // Here's a simple implementation that only gets the main MACD line
        let results = Self::calculate_indicator(candle_data, "MACD", &parameters)?;
        
        // Transform into the format expected (with MACD, signal, histogram)
        // In a real implementation, we would get all three values from TA-Lib
        let transformed_results = results
            .into_iter()
            .map(|(time, value)| {
                let macd_value = value.as_f64().unwrap_or(0.0);
                // This is a placeholder - we should get actual signal and histogram values
                let signal_value = 0.0; 
                let histogram_value = 0.0;
                
                (time, json!({
                    "macd": macd_value,
                    "signal": signal_value,
                    "histogram": histogram_value
                }))
            })
            .collect();
        
        Ok(transformed_results)
    }

    // Similar specialized functions could be implemented for other multi-output indicators
    // like Bollinger Bands, Stochastics, etc.
    
    // Function to map indicator types to their TA-Lib function names
    pub fn get_ta_function_name(indicator_name: &str) -> String {
        TaLibAbstract::get_function_name(indicator_name)
    }
}

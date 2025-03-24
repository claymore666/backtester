// Common functionality and main interface for TA-Lib
use crate::talib_bindings::ffi;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::os::raw::c_int;
use tracing::{debug, info, warn};

// Import indicator modules
use super::oscillators::OscillatorIndicators;
use super::overlaps::OverlapIndicators;
use super::patterns::PatternIndicators;
use super::volume::VolumeIndicators;
use super::volatility::VolatilityIndicators;

pub struct TaLibAbstract;

impl TaLibAbstract {
    // Initialize TA-Lib
    pub fn initialize() -> Result<()> {
        let ret_code = unsafe { ffi::TA_Initialize() };
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to initialize TA-Lib"));
        }
        Ok(())
    }

    // Check if a function is available
    pub fn is_function_available(function_name: &str) -> bool {
        match function_name.to_uppercase().as_str() {
            "RSI" | "SMA" | "EMA" | "MACD" | "BBANDS" | "ATR" | "STOCH" | 
            "ADX" | "OBV" | "CDLENGULFING" | "CDLHAMMER" | "CDLMORNINGSTAR" |
            "CCI" | "STOCHRSI" | "MOM" | "MFI" => true,
            _ => false,
        }
    }

    // Get standardized function name
    pub fn get_function_name(indicator_name: &str) -> String {
        // Map commonly used variations to standard TA-Lib function names
        match indicator_name.to_uppercase().as_str() {
            "RSI" => "RSI".to_string(),
            "SMA" => "SMA".to_string(),
            "EMA" => "EMA".to_string(),
            "MACD" => "MACD".to_string(),
            "BBANDS" => "BBANDS".to_string(),
            "ATR" => "ATR".to_string(),
            "STOCH" => "STOCH".to_string(),
            "ADX" => "ADX".to_string(),
            "OBV" => "OBV".to_string(),
            "ENGULFING" | "CDLENGULFING" => "CDLENGULFING".to_string(),
            "HAMMER" | "CDLHAMMER" => "CDLHAMMER".to_string(),
            "MORNINGSTAR" | "CDLMORNINGSTAR" => "CDLMORNINGSTAR".to_string(),
            "CCI" => "CCI".to_string(),
            "STOCHRSI" => "STOCHRSI".to_string(),
            "MOM" => "MOM".to_string(),
            "MFI" => "MFI".to_string(),
            _ => indicator_name.to_uppercase(),
        }
    }

    // Call TA-Lib function with appropriate parameters
    pub fn call_function(
        function_name: &str,
        open: Option<&[f64]>,
        high: Option<&[f64]>,
        low: Option<&[f64]>,
        close: Option<&[f64]>,
        volume: Option<&[f64]>,
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        match function_name.to_uppercase().as_str() {
            // Oscillator indicators
            "RSI" => OscillatorIndicators::calculate_rsi(close.unwrap_or(&[]), parameters),
            "MACD" => OscillatorIndicators::calculate_macd(close.unwrap_or(&[]), parameters),
            "STOCH" => OscillatorIndicators::calculate_stoch(high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            "CCI" => OscillatorIndicators::calculate_cci(high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            "STOCHRSI" => OscillatorIndicators::calculate_stoch_rsi(close.unwrap_or(&[]), parameters),
            "MOM" => OscillatorIndicators::calculate_momentum(close.unwrap_or(&[]), parameters),
            "MFI" => OscillatorIndicators::calculate_money_flow_index(high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), volume.unwrap_or(&[]), parameters),
            
            // Overlap indicators
            "SMA" => OverlapIndicators::calculate_sma(close.unwrap_or(&[]), parameters),
            "EMA" => OverlapIndicators::calculate_ema(close.unwrap_or(&[]), parameters),
            "BBANDS" => OverlapIndicators::calculate_bbands(close.unwrap_or(&[]), parameters),
            
            // Volatility indicators
            "ATR" => VolatilityIndicators::calculate_atr(high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            "ADX" => VolatilityIndicators::calculate_adx(high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            
            // Volume indicators 
            "OBV" => VolumeIndicators::calculate_obv(close.unwrap_or(&[]), volume.unwrap_or(&[]), parameters),
            
            // Pattern indicators
            "CDLENGULFING" => PatternIndicators::calculate_cdl_engulfing(open.unwrap_or(&[]), high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            "CDLHAMMER" => PatternIndicators::calculate_cdl_hammer(open.unwrap_or(&[]), high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            "CDLMORNINGSTAR" => PatternIndicators::calculate_cdl_morning_star(open.unwrap_or(&[]), high.unwrap_or(&[]), low.unwrap_or(&[]), close.unwrap_or(&[]), parameters),
            
            _ => Err(anyhow!("Unsupported function: {}", function_name)),
        }
    }

    // Helper method to get an integer parameter
    pub fn get_integer_param(
        parameters: &[(String, Value)], 
        name: &str, 
        default: c_int
    ) -> Result<c_int> {
        for (param_name, value) in parameters {
            if param_name == name {
                if let Value::Number(num) = value {
                    if let Some(i) = num.as_i64() {
                        return Ok(i as c_int);
                    }
                }
            }
        }
        debug!("Using default value {} for parameter {}", default, name);
        Ok(default)
    }
    
    // Helper method to get a float parameter
    pub fn get_float_param(
        parameters: &[(String, Value)], 
        name: &str, 
        default: f64
    ) -> Result<f64> {
        for (param_name, value) in parameters {
            if param_name == name {
                if let Value::Number(num) = value {
                    if let Some(f) = num.as_f64() {
                        return Ok(f);
                    }
                }
            }
        }
        debug!("Using default value {} for parameter {}", default, name);
        Ok(default)
    }
}

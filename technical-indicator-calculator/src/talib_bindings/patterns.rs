// Pattern indicators implementation
use crate::talib_bindings::ffi;
use crate::talib_bindings::common::TaLibAbstract;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::os::raw::c_int;

pub struct PatternIndicators;

impl PatternIndicators {
    // Calculate Engulfing Pattern
    pub fn calculate_cdl_engulfing(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        _parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if open.is_empty() || high.is_empty() || low.is_empty() || close.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = open.len();
        if high.len() != data_len || low.len() != data_len || close.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLENGULFING function
        let ret_code = unsafe {
            ffi::TA_CDLENGULFING(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLENGULFING, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = json!({
                    "pattern": "ENGULFING",
                    "type": if out_data[i] > 0 { "bullish" } else { "bearish" },
                    "strength": out_data[i].abs() as f64 / 100.0,
                });
                
                results.push((original_idx, pattern_value));
            }
        }
        
        Ok(results)
    }

    // Calculate Hammer Pattern
    pub fn calculate_cdl_hammer(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        _parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if open.is_empty() || high.is_empty() || low.is_empty() || close.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = open.len();
        if high.len() != data_len || low.len() != data_len || close.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLHAMMER function
        let ret_code = unsafe {
            ffi::TA_CDLHAMMER(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLHAMMER, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = json!({
                    "pattern": "HAMMER",
                    "type": "bullish", // Hammer is a bullish pattern
                    "strength": out_data[i].abs() as f64 / 100.0,
                });
                
                results.push((original_idx, pattern_value));
            }
        }
        
        Ok(results)
    }

    // Calculate Morning Star Pattern
    pub fn calculate_cdl_morning_star(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if open.is_empty() || high.is_empty() || low.is_empty() || close.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = open.len();
        if high.len() != data_len || low.len() != data_len || close.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }
        
        // Get the penetration parameter (usually between 0.0 and 1.0)
        let penetration = TaLibAbstract::get_float_param(parameters, "penetration", 0.3)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLMORNINGSTAR function
        let ret_code = unsafe {
            ffi::TA_CDLMORNINGSTAR(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                penetration,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLMORNINGSTAR, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = json!({
                    "pattern": "MORNINGSTAR",
                    "type": "bullish", // Morning Star is a bullish pattern
                    "strength": out_data[i].abs() as f64 / 100.0,
                });
                
                results.push((original_idx, pattern_value));
            }
        }
        
        Ok(results)
    }
}

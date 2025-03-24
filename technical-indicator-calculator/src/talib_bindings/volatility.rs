// Volatility indicators implementation
use crate::talib_bindings::ffi;
use crate::talib_bindings::common::TaLibAbstract;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::os::raw::c_int;

pub struct VolatilityIndicators;

impl VolatilityIndicators {
    // Calculate Average True Range (ATR)
    pub fn calculate_atr(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if high.is_empty() || low.is_empty() || close.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = high.len();
        if low.len() != data_len || close.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib ATR function
        let ret_code = unsafe {
            ffi::TA_ATR(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_ATR, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // Calculate Average Directional Index (ADX)
    pub fn calculate_adx(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if high.is_empty() || low.is_empty() || close.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = high.len();
        if low.len() != data_len || close.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib ADX function
        let ret_code = unsafe {
            ffi::TA_ADX(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_ADX, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }
}

// Overlap indicators implementation
use crate::talib_bindings::ffi;
use crate::talib_bindings::common::TaLibAbstract;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::os::raw::c_int;

pub struct OverlapIndicators;

impl OverlapIndicators {
    // Calculate SMA
    pub fn calculate_sma(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib SMA function
        let ret_code = unsafe {
            ffi::TA_SMA(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_SMA, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // Calculate EMA
    pub fn calculate_ema(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 9)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib EMA function
        let ret_code = unsafe {
            ffi::TA_EMA(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_EMA, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // Calculate Bollinger Bands
    pub fn calculate_bbands(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 20)?;
        let dev_up = TaLibAbstract::get_float_param(parameters, "deviation_up", 2.0)?;
        let dev_down = TaLibAbstract::get_float_param(parameters, "deviation_down", 2.0)?;
        let ma_type = TaLibAbstract::get_integer_param(parameters, "ma_type", 0)?; // 0 = SMA
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_upper = vec![0.0; close.len()];
        let mut out_middle = vec![0.0; close.len()];
        let mut out_lower = vec![0.0; close.len()];
        
        // Call TA-Lib BBANDS function
        let ret_code = unsafe {
            ffi::TA_BBANDS(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                dev_up,
                dev_down,
                ma_type,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_upper.as_mut_ptr(),
                out_middle.as_mut_ptr(),
                out_lower.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_BBANDS, error code: {}", ret_code));
        }
        
        // Create result vector with all three bands
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Calculate bandwidth
            let bandwidth = if out_middle[i] != 0.0 {
                (out_upper[i] - out_lower[i]) / out_middle[i]
            } else {
                0.0
            };
            
            let bbands_value = json!({
                "upper": out_upper[i],
                "middle": out_middle[i],
                "lower": out_lower[i],
                "width": bandwidth,
            });
            
            results.push((original_idx, bbands_value));
        }
        
        Ok(results)
    }
}

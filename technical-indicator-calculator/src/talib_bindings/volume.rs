// Volume indicators implementation
use crate::talib_bindings::ffi;
use crate::talib_bindings::common::TaLibAbstract;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::os::raw::c_int;

pub struct VolumeIndicators;

impl VolumeIndicators {
    // Calculate On Balance Volume (OBV)
    pub fn calculate_obv(
        close: &[f64],
        volume: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() || volume.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = close.len();
        if volume.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib OBV function
        let ret_code = unsafe {
            ffi::TA_OBV(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                close.as_ptr(),
                volume.as_ptr(),
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_OBV, error code: {}", ret_code));
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

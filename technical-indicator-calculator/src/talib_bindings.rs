use anyhow::{anyhow, Result};
use std::os::raw::{c_double, c_int};
use serde_json::Value;
use tracing::{debug, warn};

// Error code constants
pub const TA_SUCCESS: c_int = 0;

// Direct function bindings for basic TA-Lib functions
#[link(name = "ta-lib")]
extern "C" {
    // Initialize TA-Lib
    fn TA_Initialize() -> c_int;
    
    // RSI - Relative Strength Index
    fn TA_RSI(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // SMA - Simple Moving Average
    fn TA_SMA(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
}

pub struct TaLibAbstract;

impl TaLibAbstract {
    // Initialize TA-Lib
    pub fn initialize() -> Result<()> {
        let ret_code = unsafe { TA_Initialize() };
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to initialize TA-Lib"));
        }
        Ok(())
    }

    // Simple check if RSI is available
    pub fn is_function_available(function_name: &str) -> bool {
        function_name.to_uppercase() == "RSI"
    }

    // Just get the function name (no real work here)
    pub fn get_function_name(indicator_name: &str) -> String {
        indicator_name.to_uppercase()
    }

    // Very simplified function that only supports RSI
    pub fn call_function(
        function_name: &str,
        _open: Option<&[f64]>,
        _high: Option<&[f64]>,
        _low: Option<&[f64]>,
        close: Option<&[f64]>,
        _volume: Option<&[f64]>,
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, f64)>> {
        if let Some(close_data) = close {
            if close_data.is_empty() {
                return Ok(vec![]);
            }
            
            if function_name.to_uppercase() == "RSI" {
                let period = Self::get_integer_param(parameters, "period", 14)?;
                
                // Call RSI function
                let mut out_data = vec![0.0; close_data.len()];
                let mut out_beg_idx: c_int = 0;
                let mut out_nb_element: c_int = 0;
                
                let ret_code = unsafe {
                    TA_RSI(
                        0, // startIdx
                        (close_data.len() - 1) as c_int, // endIdx
                        close_data.as_ptr(),
                        period,
                        &mut out_beg_idx,
                        &mut out_nb_element,
                        out_data.as_mut_ptr(),
                    )
                };
                
                if ret_code != TA_SUCCESS {
                    return Err(anyhow!("Failed to call TA_RSI"));
                }
                
                let mut results = Vec::with_capacity(out_nb_element as usize);
                for i in 0..out_nb_element as usize {
                    let original_idx = out_beg_idx as usize + i;
                    results.push((original_idx, out_data[i]));
                }
                
                return Ok(results);
            } else if function_name.to_uppercase() == "SMA" {
                let period = Self::get_integer_param(parameters, "period", 14)?;
                
                // Call SMA function
                let mut out_data = vec![0.0; close_data.len()];
                let mut out_beg_idx: c_int = 0;
                let mut out_nb_element: c_int = 0;
                
                let ret_code = unsafe {
                    TA_SMA(
                        0, // startIdx
                        (close_data.len() - 1) as c_int, // endIdx
                        close_data.as_ptr(),
                        period,
                        &mut out_beg_idx,
                        &mut out_nb_element,
                        out_data.as_mut_ptr(),
                    )
                };
                
                if ret_code != TA_SUCCESS {
                    return Err(anyhow!("Failed to call TA_SMA"));
                }
                
                let mut results = Vec::with_capacity(out_nb_element as usize);
                for i in 0..out_nb_element as usize {
                    let original_idx = out_beg_idx as usize + i;
                    results.push((original_idx, out_data[i]));
                }
                
                return Ok(results);
            }
        }
        
        // Default: return empty results
        Ok(vec![])
    }
    
    // Helper method to get an integer parameter
    fn get_integer_param(
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
}

use anyhow::{anyhow, Result};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};
use serde_json::Value;

#[allow(non_camel_case_types)]
type TA_RetCode = c_int;
#[allow(non_camel_case_types)]
type TA_Integer = c_int;
#[allow(non_camel_case_types)]
type TA_Real = c_double;
#[allow(non_camel_case_types)]
type TA_FuncHandle = *const c_void;
#[allow(non_camel_case_types)]
type TA_ParamHolder = *mut c_void;

// Constants from ta_defs.h
pub const TA_SUCCESS: TA_RetCode = 0;
pub const TA_BAD_PARAM: TA_RetCode = 1;
pub const TA_BAD_OBJECT: TA_RetCode = 2;

// Define FFI bindings to TA-Lib abstract interface
#[link(name = "ta-lib")]
extern "C" {
    // Get function handle by name
    fn TA_GetFuncHandle(name: *const c_char, handle: *mut TA_FuncHandle) -> TA_RetCode;
    
    // Parameter handling
    fn TA_ParamHolderAlloc(handle: TA_FuncHandle, params: *mut TA_ParamHolder) -> TA_RetCode;
    fn TA_ParamHolderFree(params: TA_ParamHolder) -> TA_RetCode;
    
    // Input parameter setting
    fn TA_SetInputParamRealPtr(params: TA_ParamHolder, paramIndex: c_int, value: *const TA_Real) -> TA_RetCode;
    fn TA_SetInputParamPricePtr(
        params: TA_ParamHolder,
        paramIndex: c_int,
        open: *const TA_Real,
        high: *const TA_Real,
        low: *const TA_Real,
        close: *const TA_Real,
        volume: *const TA_Real,
        openInterest: *const TA_Real,
    ) -> TA_RetCode;
    
    // Optional input parameter setting
    fn TA_SetOptInputParamInteger(params: TA_ParamHolder, paramIndex: c_int, value: TA_Integer) -> TA_RetCode;
    fn TA_SetOptInputParamReal(params: TA_ParamHolder, paramIndex: c_int, value: TA_Real) -> TA_RetCode;
    
    // Output parameter setting
    fn TA_SetOutputParamRealPtr(params: TA_ParamHolder, paramIndex: c_int, out: *mut TA_Real) -> TA_RetCode;
    
    // Call function
    fn TA_CallFunc(
        params: TA_ParamHolder,
        startIdx: TA_Integer,
        endIdx: TA_Integer,
        outBegIdx: *mut TA_Integer,
        outNbElement: *mut TA_Integer,
    ) -> TA_RetCode;
}

// Safe Rust wrapper for TA-Lib abstract interface
pub struct TaLibAbstract;

impl TaLibAbstract {
    // Call TA-Lib function with safety wrapping
    pub fn call_function(
        function_name: &str,
        open: Option<&[f64]>,
        high: Option<&[f64]>,
        low: Option<&[f64]>,
        close: Option<&[f64]>,
        volume: Option<&[f64]>,
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, f64)>> {
        // Create C-string for function name
        let c_func_name = CString::new(function_name)?;
        
        // Get function handle
        let mut handle = std::ptr::null();
        let ret_code = unsafe { TA_GetFuncHandle(c_func_name.as_ptr(), &mut handle) };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to get TA function handle for {}: error {}", function_name, ret_code));
        }
        
        // Allocate parameter holder
        let mut param_holder = std::ptr::null_mut();
        let ret_code = unsafe { TA_ParamHolderAlloc(handle, &mut param_holder) };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to allocate TA parameter holder: error {}", ret_code));
        }
        
        // Set price inputs if provided
        if let (Some(open), Some(high), Some(low), Some(close)) = (open, high, low, close) {
            let length = close.len();
            if length == 0 {
                unsafe { TA_ParamHolderFree(param_holder) };
                return Err(anyhow!("Empty price data"));
            }
            
            // Set price inputs
            let ret_code = unsafe {
                TA_SetInputParamPricePtr(
                    param_holder,
                    0,  // Most TA functions use input index 0 for price
                    open.as_ptr(),
                    high.as_ptr(),
                    low.as_ptr(),
                    close.as_ptr(),
                    volume.unwrap_or(&[]).as_ptr(),
                    std::ptr::null(),  // No open interest data
                )
            };
            
            if ret_code != TA_SUCCESS {
                unsafe { TA_ParamHolderFree(param_holder) };
                return Err(anyhow!("Failed to set price inputs: error {}", ret_code));
            }
            
            // Set optional parameters
            for (i, (name, value)) in parameters.iter().enumerate() {
                // Convert from zero-based to one-based parameter index (TA-Lib convention)
                let param_index = (i + 1) as c_int;
                
                match value {
                    Value::Number(n) => {
                        if let Some(int_val) = n.as_i64() {
                            let ret_code = unsafe {
                                TA_SetOptInputParamInteger(param_holder, param_index, int_val as TA_Integer)
                            };
                            if ret_code != TA_SUCCESS {
                                unsafe { TA_ParamHolderFree(param_holder) };
                                return Err(anyhow!("Failed to set integer parameter {}: error {}", name, ret_code));
                            }
                        } else if let Some(float_val) = n.as_f64() {
                            let ret_code = unsafe {
                                TA_SetOptInputParamReal(param_holder, param_index, float_val as TA_Real)
                            };
                            if ret_code != TA_SUCCESS {
                                unsafe { TA_ParamHolderFree(param_holder) };
                                return Err(anyhow!("Failed to set real parameter {}: error {}", name, ret_code));
                            }
                        }
                    },
                    _ => {
                        // Skip non-numeric parameters
                        continue;
                    },
                }
            }
            
            // Allocate output buffer
            let mut output = vec![0.0; length];
            
            // Set output parameter
            let ret_code = unsafe {
                TA_SetOutputParamRealPtr(param_holder, 0, output.as_mut_ptr())
            };
            
            if ret_code != TA_SUCCESS {
                unsafe { TA_ParamHolderFree(param_holder) };
                return Err(anyhow!("Failed to set output parameter: error {}", ret_code));
            }
            
            // Call the function
            let mut out_begin = 0;
            let mut out_size = 0;
            
            let ret_code = unsafe {
                TA_CallFunc(
                    param_holder,
                    0,                // Start from first input
                    (length - 1) as TA_Integer, // End at last input
                    &mut out_begin,   // Index of first output
                    &mut out_size,    // Number of output elements
                )
            };
            
            // Free parameter holder regardless of call result
            unsafe { TA_ParamHolderFree(param_holder) };
            
            if ret_code != TA_SUCCESS {
                return Err(anyhow!("Failed to call TA function: error {}", ret_code));
            }
            
            // Transform output to (index, value) pairs
            let mut results = Vec::with_capacity(out_size as usize);
            
            for i in 0..out_size {
                let idx = (out_begin + i) as usize;
                let val = output[i as usize];
                
                if !val.is_nan() {
                    results.push((idx, val));
                }
            }
            
            return Ok(results);
        } else {
            unsafe { TA_ParamHolderFree(param_holder) };
            return Err(anyhow!("Required price data not provided"));
        }
    }
    
    // Method to check if a function is available
    pub fn is_function_available(function_name: &str) -> bool {
        let c_func_name = match CString::new(function_name) {
            Ok(s) => s,
            Err(_) => return false,
        };
        
        let mut handle = std::ptr::null();
        let ret_code = unsafe { TA_GetFuncHandle(c_func_name.as_ptr(), &mut handle) };
        
        ret_code == TA_SUCCESS
    }
    
    // Helper to get function name and verify it exists
    pub fn get_function_name(indicator_name: &str) -> String {
        let name = match indicator_name {
            "RSI" => "RSI",
            "SMA" => "SMA",
            "EMA" => "EMA",
            "MACD" => "MACD",
            "BBANDS" => "BBANDS",
            "ATR" => "ATR",
            "STOCH" => "STOCH",
            "MFI" => "MFI",
            "OBV" => "OBV",
            "ADX" => "ADX",
            "CCI" => "CCI",
            "ROC" => "ROC",
            "WILLR" => "WILLR",
            "STDDEV" => "STDDEV",
            "SAR" => "SAR",
            "TEMA" => "TEMA",
            "DEMA" => "DEMA",
            "KAMA" => "KAMA",
            "TRIMA" => "TRIMA",
            "WMA" => "WMA",
            // Default to same name
            _ => indicator_name,
        };
        
        name.to_string()
    }
}

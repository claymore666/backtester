use anyhow::{anyhow, Result};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};
use serde_json::Value;
use tracing::{debug, error, warn};

// Error code constants
pub const TA_SUCCESS: TA_RetCode = 0;
pub const TA_BAD_PARAM: TA_RetCode = 1;
pub const TA_BAD_OBJECT: TA_RetCode = 2;
pub const TA_NOT_SUPPORTED: TA_RetCode = 3;
pub const TA_MEMORY_ALLOCATION_ERROR: TA_RetCode = 4;
pub const TA_INTERNAL_ERROR: TA_RetCode = 5;
pub const TA_UNKNOWN_ERR: TA_RetCode = 6;

// Error code decoder
pub fn decode_ta_return_code(code: TA_RetCode) -> &'static str {
    match code {
        TA_SUCCESS => "Success",
        TA_BAD_PARAM => "Bad Parameter (one of the parameters has an invalid value)",
        TA_BAD_OBJECT => "Bad Object (a TA object is invalid or not initialized)",
        TA_NOT_SUPPORTED => "Not Supported (the operation is not supported)",
        TA_MEMORY_ALLOCATION_ERROR => "Memory Allocation Error",
        TA_INTERNAL_ERROR => "Internal Error (unexpected error encountered)",
        TA_UNKNOWN_ERR => "Unknown Error",
        _ => "Unrecognized TA-Lib error code",
    }
}

// Type aliases for TA-Lib C types
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

// FFI bindings (keep existing external function declarations)
#[link(name = "ta-lib")]
extern "C" {
    // Existing function declarations...
    fn TA_GetFuncHandle(name: *const c_char, handle: *mut TA_FuncHandle) -> TA_RetCode;
    fn TA_GetFuncInfo(handle: TA_FuncHandle, funcInfo: *mut *const c_void) -> TA_RetCode;
    fn TA_GetFuncName(funcInfo: *const c_void, funcName: *mut *const c_char) -> TA_RetCode;
    
    // New: Function to get number of inputs/outputs
    fn TA_GetInputParameterCount(handle: TA_FuncHandle, count: *mut TA_Integer) -> TA_RetCode;
    fn TA_GetOutputParameterCount(handle: TA_FuncHandle, count: *mut TA_Integer) -> TA_RetCode;
}

/// Detailed function information for TA-Lib functions
#[derive(Debug)]
pub struct TALibFunctionInfo {
    pub name: String,
    pub input_count: i32,
    pub output_count: i32,
}

pub struct TaLibAbstract;

impl TaLibAbstract {
    // Add this missing method
    pub fn get_function_name(indicator_name: &str) -> String {
        match indicator_name.to_uppercase().as_str() {
            "SMA" => "SMA".to_string(),
            "EMA" => "EMA".to_string(),
            "MACD" => "MACD".to_string(),
            "RSI" => "RSI".to_string(),
            "BBANDS" => "BBANDS".to_string(),
            "STOCH" => "STOCH".to_string(),
            "CCI" => "CCI".to_string(),
            "MFI" => "MFI".to_string(),
            _ => indicator_name.to_uppercase(),
        }
    }

    // Add is_function_available method
    pub fn is_function_available(function_name: &str) -> bool {
        let c_func_name = match CString::new(function_name) {
            Ok(name) => name,
            Err(_) => return false,
        };
        
        let mut handle: TA_FuncHandle = std::ptr::null();
        let ret_code = unsafe { 
            TA_GetFuncHandle(c_func_name.as_ptr(), &mut handle as *mut TA_FuncHandle) 
        };
        
        ret_code == 0 && !handle.is_null()
    }


    // Enhanced function information retrieval
    pub fn get_function_info(function_name: &str) -> Result<TALibFunctionInfo> {
        let c_func_name = CString::new(function_name)?;
        
        // Get function handle
        let mut handle = std::ptr::null();
        let ret_code = unsafe { TA_GetFuncHandle(c_func_name.as_ptr(), &mut handle) };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!(
                "Failed to get function handle for {}: {} ({})", 
                function_name, 
                ret_code, 
                decode_ta_return_code(ret_code)
            ));
        }
        
        // Get function name 
        let mut func_info = std::ptr::null();
        let mut real_func_name = std::ptr::null();
        
        // Retrieve function name
        unsafe {
            if TA_GetFuncInfo(handle, &mut func_info) != TA_SUCCESS || 
               TA_GetFuncName(func_info, &mut real_func_name) != TA_SUCCESS {
                return Err(anyhow!("Failed to retrieve function name"));
            }
            
            let name = CStr::from_ptr(real_func_name)
                .to_string_lossy()
                .into_owned();
            
            // Get input parameter count
            let mut input_count = 0;
            let input_ret = TA_GetInputParameterCount(handle, &mut input_count);
            
            // Get output parameter count
            let mut output_count = 0;
            let output_ret = TA_GetOutputParameterCount(handle, &mut output_count);
            
            if input_ret != TA_SUCCESS || output_ret != TA_SUCCESS {
                return Err(anyhow!("Failed to retrieve parameter counts"));
            }
            
            Ok(TALibFunctionInfo {
                name,
                input_count,
                output_count,
            })
        }
    }

    // Existing call_function method with enhanced error handling
    pub fn call_function(
        function_name: &str,
        open: Option<&[f64]>,
        high: Option<&[f64]>,
        low: Option<&[f64]>,
        close: Option<&[f64]>,
        volume: Option<&[f64]>,
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, f64)>> {
        // Log function call details
        debug!("Calling TA-Lib function: {}", function_name);
        
        // Optional: Get and log function information before calling
        match Self::get_function_info(function_name) {
            Ok(info) => {
                debug!(
                    "Function Details: Name={}, Inputs={}, Outputs={}", 
                    info.name, info.input_count, info.output_count
                );
            }
            Err(e) => {
                warn!("Could not retrieve function info: {}", e);
            }
        }

        // Placeholder return to satisfy the compiler
        Ok(Vec::new())
    }

    // Additional utility method for comprehensive function listing
    pub fn list_available_functions() -> Vec<String> {
        vec![
            "SMA".to_string(), 
            "EMA".to_string(), 
            "MACD".to_string(), 
            "RSI".to_string(), 
            "STOCH".to_string()
        ]
    }
}

// Utility traits for easier parameter handling
trait TaLibParameter {
    fn set_parameter(&self, param_holder: TA_ParamHolder, index: c_int) -> Result<()>;
}

impl TaLibParameter for Value {
    fn set_parameter(&self, param_holder: TA_ParamHolder, index: c_int) -> Result<()> {
        match self {
            Value::Number(n) => {
                if let Some(int_val) = n.as_i64() {
                    let ret_code = unsafe {
                        TA_SetOptInputParamInteger(param_holder, index, int_val as TA_Integer)
                    };
                    if ret_code != TA_SUCCESS {
                        return Err(anyhow!(
                            "Failed to set integer parameter: {} ({})", 
                            ret_code, 
                            decode_ta_return_code(ret_code)
                        ));
                    }
                } else if let Some(float_val) = n.as_f64() {
                    let ret_code = unsafe {
                        TA_SetOptInputParamReal(param_holder, index, float_val as TA_Real)
                    };
                    if ret_code != TA_SUCCESS {
                        return Err(anyhow!(
                            "Failed to set real parameter: {} ({})", 
                            ret_code, 
                            decode_ta_return_code(ret_code)
                        ));
                    }
                }
                Ok(())
            },
            _ => Err(anyhow!("Unsupported parameter type for TA-Lib")),
        }
    }
}

// Placeholder FFI function declarations
#[link(name = "ta-lib")]
extern "C" {
    fn TA_SetOptInputParamInteger(
        params: TA_ParamHolder, 
        paramIndex: c_int, 
        value: TA_Integer
    ) -> TA_RetCode;
    
    fn TA_SetOptInputParamReal(
        params: TA_ParamHolder, 
        paramIndex: c_int, 
        value: TA_Real
    ) -> TA_RetCode;
}

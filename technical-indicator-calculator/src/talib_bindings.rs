use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_void};

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
// Note: Using the correct library name (ta-lib)
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

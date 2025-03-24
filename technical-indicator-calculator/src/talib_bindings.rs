use anyhow::{anyhow, Result};
use std::os::raw::{c_char, c_double, c_int};
use std::ffi::CString;
use serde_json::{json, Value};
use tracing::{debug, info, warn};

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
    
    // EMA - Exponential Moving Average
    fn TA_EMA(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // MACD - Moving Average Convergence/Divergence
    fn TA_MACD(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInFastPeriod: c_int,
        optInSlowPeriod: c_int,
        optInSignalPeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outMACD: *mut c_double,
        outMACDSignal: *mut c_double,
        outMACDHist: *mut c_double,
    ) -> c_int;
    
    // BBANDS - Bollinger Bands
    fn TA_BBANDS(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        optInNbDevUp: c_double,
        optInNbDevDn: c_double,
        optInMAType: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outRealUpperBand: *mut c_double,
        outRealMiddleBand: *mut c_double,
        outRealLowerBand: *mut c_double,
    ) -> c_int;
    
    // ATR - Average True Range
    fn TA_ATR(
        startIdx: c_int,
        endIdx: c_int,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // STOCH - Stochastic
    fn TA_STOCH(
        startIdx: c_int,
        endIdx: c_int,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        optInFastK_Period: c_int,
        optInSlowK_Period: c_int,
        optInSlowK_MAType: c_int,
        optInSlowD_Period: c_int,
        optInSlowD_MAType: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outSlowK: *mut c_double,
        outSlowD: *mut c_double,
    ) -> c_int;
    
    // ADX - Average Directional Movement Index
    fn TA_ADX(
        startIdx: c_int,
        endIdx: c_int,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // OBV - On Balance Volume
    fn TA_OBV(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        inVolume: *const c_double,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // Patterns - Engulfing (example of a pattern function)
    fn TA_CDLENGULFING(
        startIdx: c_int,
        endIdx: c_int,
        inOpen: *const c_double,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outInteger: *mut c_int,
    ) -> c_int;
    
    // Patterns - Hammer
    fn TA_CDLHAMMER(
        startIdx: c_int,
        endIdx: c_int,
        inOpen: *const c_double,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outInteger: *mut c_int,
    ) -> c_int;
    
    // Patterns - Morning Star
    fn TA_CDLMORNINGSTAR(
        startIdx: c_int,
        endIdx: c_int,
        inOpen: *const c_double,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        optInPenetration: c_double,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outInteger: *mut c_int,
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

    // Check if a function is available
    pub fn is_function_available(function_name: &str) -> bool {
        match function_name.to_uppercase().as_str() {
            "RSI" | "SMA" | "EMA" | "MACD" | "BBANDS" | "ATR" | "STOCH" | 
            "ADX" | "OBV" | "CDLENGULFING" | "CDLHAMMER" | "CDLMORNINGSTAR" => true,
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
            "RSI" => Self::calculate_rsi(close.unwrap_or(&[]), parameters),
            "SMA" => Self::calculate_sma(close.unwrap_or(&[]), parameters),
            "EMA" => Self::calculate_ema(close.unwrap_or(&[]), parameters),
            "MACD" => Self::calculate_macd(close.unwrap_or(&[]), parameters),
            "BBANDS" => Self::calculate_bbands(close.unwrap_or(&[]), parameters),
            "ATR" => Self::calculate_atr(
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            "STOCH" => Self::calculate_stoch(
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            "ADX" => Self::calculate_adx(
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            "OBV" => Self::calculate_obv(
                close.unwrap_or(&[]),
                volume.unwrap_or(&[]),
                parameters,
            ),
            "CDLENGULFING" => Self::calculate_cdl_engulfing(
                open.unwrap_or(&[]),
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            "CDLHAMMER" => Self::calculate_cdl_hammer(
                open.unwrap_or(&[]),
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            "CDLMORNINGSTAR" => Self::calculate_cdl_morning_star(
                open.unwrap_or(&[]),
                high.unwrap_or(&[]),
                low.unwrap_or(&[]),
                close.unwrap_or(&[]),
                parameters,
            ),
            _ => Err(anyhow!("Unsupported function: {}", function_name)),
        }
    }

    // Calculate RSI
    fn calculate_rsi(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = Self::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib RSI function
        let ret_code = unsafe {
            TA_RSI(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_RSI, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // Calculate SMA
    fn calculate_sma(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = Self::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib SMA function
        let ret_code = unsafe {
            TA_SMA(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
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
    fn calculate_ema(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = Self::get_integer_param(parameters, "period", 9)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib EMA function
        let ret_code = unsafe {
            TA_EMA(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
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

    // Calculate MACD
    fn calculate_macd(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let fast_period = Self::get_integer_param(parameters, "fast_period", 12)?;
        let slow_period = Self::get_integer_param(parameters, "slow_period", 26)?;
        let signal_period = Self::get_integer_param(parameters, "signal_period", 9)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_macd = vec![0.0; close.len()];
        let mut out_signal = vec![0.0; close.len()];
        let mut out_hist = vec![0.0; close.len()];
        
        // Call TA-Lib MACD function
        let ret_code = unsafe {
            TA_MACD(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                fast_period,
                slow_period,
                signal_period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_macd.as_mut_ptr(),
                out_signal.as_mut_ptr(),
                out_hist.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_MACD, error code: {}", ret_code));
        }
        
        // Create result vector with all three output values
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            let macd_value = serde_json::json!({
                "macd": out_macd[i],
                "signal": out_signal[i],
                "histogram": out_hist[i],
            });
            
            results.push((original_idx, macd_value));
        }
        
        Ok(results)
    }

    // Calculate Bollinger Bands
    fn calculate_bbands(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = Self::get_integer_param(parameters, "period", 20)?;
        let dev_up = Self::get_float_param(parameters, "deviation_up", 2.0)?;
        let dev_down = Self::get_float_param(parameters, "deviation_down", 2.0)?;
        let ma_type = Self::get_integer_param(parameters, "ma_type", 0)?; // 0 = SMA
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_upper = vec![0.0; close.len()];
        let mut out_middle = vec![0.0; close.len()];
        let mut out_lower = vec![0.0; close.len()];
        
        // Call TA-Lib BBANDS function
        let ret_code = unsafe {
            TA_BBANDS(
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
        
        if ret_code != TA_SUCCESS {
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
            
            let bbands_value = serde_json::json!({
                "upper": out_upper[i],
                "middle": out_middle[i],
                "lower": out_lower[i],
                "width": bandwidth,
            });
            
            results.push((original_idx, bbands_value));
        }
        
        Ok(results)
    }

    // Calculate Average True Range (ATR)
    fn calculate_atr(
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

        let period = Self::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib ATR function
        let ret_code = unsafe {
            TA_ATR(
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
        
        if ret_code != TA_SUCCESS {
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

    // Calculate Stochastic
    fn calculate_stoch(
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

        let k_period = Self::get_integer_param(parameters, "k_period", 14)?;
        let k_slowing = Self::get_integer_param(parameters, "slowing", 3)?;
        let d_period = Self::get_integer_param(parameters, "d_period", 3)?;
        let ma_type = Self::get_integer_param(parameters, "ma_type", 0)?; // 0 = SMA
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_k = vec![0.0; data_len];
        let mut out_d = vec![0.0; data_len];
        
        // Call TA-Lib STOCH function
        let ret_code = unsafe {
            TA_STOCH(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                k_period,
                k_slowing,
                ma_type,
                d_period,
                ma_type,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_k.as_mut_ptr(),
                out_d.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_STOCH, error code: {}", ret_code));
        }
        
        // Create result vector with both K and D
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            let stoch_value = serde_json::json!({
                "k": out_k[i],
                "d": out_d[i],
            });
            
            results.push((original_idx, stoch_value));
        }
        
        Ok(results)
    }

    // Calculate Average Directional Index (ADX)
    fn calculate_adx(
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

        let period = Self::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib ADX function
        let ret_code = unsafe {
            TA_ADX(
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
        
        if ret_code != TA_SUCCESS {
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

    // Calculate On Balance Volume (OBV)
    fn calculate_obv(
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
            TA_OBV(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                close.as_ptr(),
                volume.as_ptr(),
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != TA_SUCCESS {
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

    // Calculate Engulfing Pattern
    fn calculate_cdl_engulfing(
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
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLENGULFING function
        let ret_code = unsafe {
            TA_CDLENGULFING(
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
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLENGULFING, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = serde_json::json!({
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
    fn calculate_cdl_hammer(
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
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLHAMMER function
        let ret_code = unsafe {
            TA_CDLHAMMER(
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
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLHAMMER, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = serde_json::json!({
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
    fn calculate_cdl_morning_star(
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
        let penetration = Self::get_float_param(parameters, "penetration", 0.3)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0; data_len];
        
        // Call TA-Lib CDLMORNINGSTAR function
        let ret_code = unsafe {
            TA_CDLMORNINGSTAR(
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
        
        if ret_code != TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_CDLMORNINGSTAR, error code: {}", ret_code));
        }
        
        // Create result vector - pattern recognition returns integers
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            // Non-zero values indicate pattern detected
            // Convert to a meaningful JSON structure
            if out_data[i] != 0 {
                let pattern_value = serde_json::json!({
                    "pattern": "MORNINGSTAR",
                    "type": "bullish", // Morning Star is a bullish pattern
                    "strength": out_data[i].abs() as f64 / 100.0,
                });
                
                results.push((original_idx, pattern_value));
            }
        }
        
        Ok(results)
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
    
    // Helper method to get a float parameter
    fn get_float_param(
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

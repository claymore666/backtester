// Oscillator indicators implementation
use crate::talib_bindings::ffi;
use crate::talib_bindings::common::TaLibAbstract;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::os::raw::c_int;

pub struct OscillatorIndicators;

impl OscillatorIndicators {
    // Calculate RSI
    pub fn calculate_rsi(
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
        
        // Call TA-Lib RSI function
        let ret_code = unsafe {
            ffi::TA_RSI(
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

    // Calculate MACD
    pub fn calculate_macd(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let fast_period = TaLibAbstract::get_integer_param(parameters, "fast_period", 12)?;
        let slow_period = TaLibAbstract::get_integer_param(parameters, "slow_period", 26)?;
        let signal_period = TaLibAbstract::get_integer_param(parameters, "signal_period", 9)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_macd = vec![0.0; close.len()];
        let mut out_signal = vec![0.0; close.len()];
        let mut out_hist = vec![0.0; close.len()];
        
        // Call TA-Lib MACD function
        let ret_code = unsafe {
            ffi::TA_MACD(
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
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_MACD, error code: {}", ret_code));
        }
        
        // Create result vector with all three output values
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            let macd_value = json!({
                "macd": out_macd[i],
                "signal": out_signal[i],
                "histogram": out_hist[i],
            });
            
            results.push((original_idx, macd_value));
        }
        
        Ok(results)
    }

    // Calculate Stochastic
    pub fn calculate_stoch(
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

        let k_period = TaLibAbstract::get_integer_param(parameters, "k_period", 14)?;
        let k_slowing = TaLibAbstract::get_integer_param(parameters, "slowing", 3)?;
        let d_period = TaLibAbstract::get_integer_param(parameters, "d_period", 3)?;
        let ma_type = TaLibAbstract::get_integer_param(parameters, "ma_type", 0)?; // 0 = SMA
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_k = vec![0.0; data_len];
        let mut out_d = vec![0.0; data_len];
        
        // Call TA-Lib STOCH function
        let ret_code = unsafe {
            ffi::TA_STOCH(
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
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_STOCH, error code: {}", ret_code));
        }
        
        // Create result vector with both K and D
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            let stoch_value = json!({
                "k": out_k[i],
                "d": out_d[i],
            });
            
            results.push((original_idx, stoch_value));
        }
        
        Ok(results)
    }

    // CCI - Commodity Channel Index
    pub fn calculate_cci(
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
        
        // Call TA-Lib CCI function
        let ret_code = unsafe {
            ffi::TA_CCI(
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
            return Err(anyhow!("Failed to call TA_CCI, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // STOCHRSI - Stochastic RSI
    pub fn calculate_stoch_rsi(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 14)?;
        let k_period = TaLibAbstract::get_integer_param(parameters, "k_period", 5)?;
        let d_period = TaLibAbstract::get_integer_param(parameters, "d_period", 3)?;
        let ma_type = TaLibAbstract::get_integer_param(parameters, "ma_type", 0)?; // 0 = SMA
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_k = vec![0.0; close.len()];
        let mut out_d = vec![0.0; close.len()];
        
        // Call TA-Lib STOCHRSI function
        let ret_code = unsafe {
            ffi::TA_STOCHRSI(
                0, // startIdx
                (close.len() - 1) as c_int, // endIdx
                close.as_ptr(),
                period,
                k_period,
                d_period,
                ma_type,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_k.as_mut_ptr(),
                out_d.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_STOCHRSI, error code: {}", ret_code));
        }
        
        // Create result vector with both K and D
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            
            let stochrsi_value = json!({
                "k": out_k[i],
                "d": out_d[i],
            });
            
            results.push((original_idx, stochrsi_value));
        }
        
        Ok(results)
    }

    // MOM - Momentum
    pub fn calculate_momentum(
        close: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if close.is_empty() {
            return Ok(vec![]);
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 10)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; close.len()];
        
        // Call TA-Lib MOM function
        let ret_code = unsafe {
            ffi::TA_MOM(
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
            return Err(anyhow!("Failed to call TA_MOM, error code: {}", ret_code));
        }
        
        // Create result vector
        let mut results = Vec::with_capacity(out_nb_element as usize);
        for i in 0..out_nb_element as usize {
            let original_idx = out_beg_idx as usize + i;
            results.push((original_idx, Value::from(out_data[i])));
        }
        
        Ok(results)
    }

    // MFI - Money Flow Index
    pub fn calculate_money_flow_index(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        parameters: &[(String, Value)],
    ) -> Result<Vec<(usize, Value)>> {
        if high.is_empty() || low.is_empty() || close.is_empty() || volume.is_empty() {
            return Ok(vec![]);
        }

        // Validate input lengths
        let data_len = high.len();
        if low.len() != data_len || close.len() != data_len || volume.len() != data_len {
            return Err(anyhow!("Input arrays must have the same length"));
        }

        let period = TaLibAbstract::get_integer_param(parameters, "period", 14)?;
        
        // Prepare output arrays
        let mut out_beg_idx: c_int = 0;
        let mut out_nb_element: c_int = 0;
        let mut out_data = vec![0.0; data_len];
        
        // Call TA-Lib MFI function
        let ret_code = unsafe {
            ffi::TA_MFI(
                0, // startIdx
                (data_len - 1) as c_int, // endIdx
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                volume.as_ptr(),
                period,
                &mut out_beg_idx,
                &mut out_nb_element,
                out_data.as_mut_ptr(),
            )
        };
        
        if ret_code != ffi::TA_SUCCESS {
            return Err(anyhow!("Failed to call TA_MFI, error code: {}", ret_code));
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

// FFI bindings for direct TA-Lib functions
use std::os::raw::{c_double, c_int};

// Error code constants
pub const TA_SUCCESS: c_int = 0;

// Direct function bindings for basic TA-Lib functions
#[link(name = "ta-lib")]
extern "C" {
    // Initialize TA-Lib
    pub fn TA_Initialize() -> c_int;
    
    // RSI - Relative Strength Index
    pub fn TA_RSI(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // SMA - Simple Moving Average
    pub fn TA_SMA(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // EMA - Exponential Moving Average
    pub fn TA_EMA(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // MACD - Moving Average Convergence/Divergence
    pub fn TA_MACD(
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
    pub fn TA_BBANDS(
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
    pub fn TA_ATR(
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
    pub fn TA_STOCH(
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
    pub fn TA_ADX(
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
    pub fn TA_OBV(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        inVolume: *const c_double,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // Patterns - Engulfing (example of a pattern function)
    pub fn TA_CDLENGULFING(
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
    pub fn TA_CDLHAMMER(
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
    pub fn TA_CDLMORNINGSTAR(
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
    
    // CCI - Commodity Channel Index
    pub fn TA_CCI(
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
    
    // STOCHRSI - Stochastic RSI
    pub fn TA_STOCHRSI(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        optInFastK_Period: c_int,
        optInFastD_Period: c_int,
        optInFastD_MAType: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outFastK: *mut c_double,
        outFastD: *mut c_double,
    ) -> c_int;
    
    // MOM - Momentum
    pub fn TA_MOM(
        startIdx: c_int,
        endIdx: c_int,
        inReal: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
    
    // MFI - Money Flow Index
    pub fn TA_MFI(
        startIdx: c_int,
        endIdx: c_int,
        inHigh: *const c_double,
        inLow: *const c_double,
        inClose: *const c_double,
        inVolume: *const c_double,
        optInTimePeriod: c_int,
        outBegIdx: *mut c_int,
        outNbElement: *mut c_int,
        outReal: *mut c_double,
    ) -> c_int;
}

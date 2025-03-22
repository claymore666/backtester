use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndicatorType {
    Oscillator,
    Overlap,
    Volume,
    Volatility,
    Pattern,
}

impl fmt::Display for IndicatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndicatorType::Oscillator => write!(f, "oscillator"),
            IndicatorType::Overlap => write!(f, "overlap"),
            IndicatorType::Volume => write!(f, "volume"),
            IndicatorType::Volatility => write!(f, "volatility"),
            IndicatorType::Pattern => write!(f, "pattern"),
        }
    }
}

impl From<&str> for IndicatorType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "oscillator" => IndicatorType::Oscillator,
            "overlap" => IndicatorType::Overlap,
            "volume" => IndicatorType::Volume,
            "volatility" => IndicatorType::Volatility,
            "pattern" => IndicatorType::Pattern,
            _ => IndicatorType::Oscillator, // Default
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationJob {
    pub symbol: String,
    pub interval: String,
    pub indicator_type: IndicatorType,
    pub indicator_name: String,
    pub parameters: serde_json::Value,
}

impl CalculationJob {
    pub fn new(
        symbol: String,
        interval: String,
        indicator_type: IndicatorType,
        indicator_name: String,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            symbol,
            interval,
            indicator_type,
            indicator_name,
            parameters,
        }
    }

    pub fn cache_key(&self) -> String {
        format!(
            "job:{}:{}:{}:{}:{}",
            self.symbol,
            self.interval,
            self.indicator_type,
            self.indicator_name,
            self.parameters.to_string(),
        )
    }
}

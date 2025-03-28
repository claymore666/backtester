// src/strategy/validator.rs
use crate::strategy::schema::{
    Strategy, StrategyIndicator, RiskManagement, CompositeCondition, Condition, ValueSource
};
use anyhow::{Result, anyhow};
use std::collections::HashSet;

/// Validate a complete strategy to ensure it meets all requirements
pub fn validate_strategy(strategy: &Strategy) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    
    // Validate basic fields
    validate_basic_fields(strategy, &mut result);
    
    // Validate indicators
    validate_indicators(strategy, &mut result);
    
    // Validate rules
    validate_rules(strategy, &mut result);
    
    // Validate risk management
    validate_risk_management(&strategy.risk_management, &mut result);
    
    // Return validation result
    Ok(result)
}

/// Validate basic fields of a strategy
fn validate_basic_fields(strategy: &Strategy, result: &mut ValidationResult) {
    // Check ID
    if strategy.id.is_empty() {
        result.add_error("Strategy ID is empty");
    } else {
        // Validate UUID format
        if uuid::Uuid::parse_str(&strategy.id).is_err() {
            result.add_error("Strategy ID is not a valid UUID");
        }
    }
    
    // Check name
    if strategy.name.is_empty() {
        result.add_error("Strategy name is empty");
    }
    
    // Check version
    if strategy.version.is_empty() {
        result.add_error("Strategy version is empty");
    }
    
    // Check assets
    if strategy.assets.is_empty() {
        result.add_error("Strategy has no assets defined");
    }
    
    // Check timeframes
    if strategy.timeframes.is_empty() {
        result.add_error("Strategy has no timeframes defined");
    }
    
    // Check if there are indicators
    if strategy.indicators.is_empty() {
        result.add_warning("Strategy has no indicators defined");
    }
    
    // Check if there are rules
    if strategy.rules.is_empty() {
        result.add_warning("Strategy has no rules defined");
    }
}

/// Validate indicators in a strategy
fn validate_indicators(strategy: &Strategy, result: &mut ValidationResult) {
    let mut indicator_ids = HashSet::new();
    
    for indicator in &strategy.indicators {
        // Check ID
        if indicator.id.is_empty() {
            result.add_error("Indicator ID is empty");
            continue;
        }
        
        // Check for duplicate IDs
        if !indicator_ids.insert(&indicator.id) {
            result.add_error(format!("Duplicate indicator ID: {}", indicator.id));
        }
        
        // Check type
        if indicator.indicator_type.is_empty() {
            result.add_error(format!("Indicator {} has empty type", indicator.id));
        } else {
            // Validate type against known types
            match indicator.indicator_type.as_str() {
                "oscillator" | "overlap" | "volume" | "volatility" | "pattern" => {},
                _ => result.add_warning(format!(
                    "Indicator {} has unknown type: {}", 
                    indicator.id, 
                    indicator.indicator_type
                )),
            }
        }
        
        // Check name
        if indicator.indicator_name.is_empty() {
            result.add_error(format!("Indicator {} has empty name", indicator.id));
        }
        
        // Check parameters based on indicator_name
        validate_indicator_parameters(indicator, result);
    }
}

/// Validate indicator parameters based on indicator type and name
fn validate_indicator_parameters(indicator: &StrategyIndicator, result: &mut ValidationResult) {
    match indicator.indicator_name.as_str() {
        "RSI" => {
            // Check for required parameters
            if !indicator.parameters.get("period").is_some() {
                result.add_warning(format!(
                    "RSI indicator {} missing 'period' parameter", 
                    indicator.id
                ));
            }
        },
        "MACD" => {
            // Check for required parameters
            let has_fast = indicator.parameters.get("fast_period").is_some();
            let has_slow = indicator.parameters.get("slow_period").is_some();
            let has_signal = indicator.parameters.get("signal_period").is_some();
            
            if !has_fast || !has_slow || !has_signal {
                result.add_warning(format!(
                    "MACD indicator {} missing required parameters (fast_period, slow_period, signal_period)",
                    indicator.id
                ));
            }
        },
        "BBANDS" => {
            // Check for required parameters
            let has_period = indicator.parameters.get("period").is_some();
            let has_dev_up = indicator.parameters.get("deviation_up").is_some();
            let has_dev_down = indicator.parameters.get("deviation_down").is_some();
            
            if !has_period || !has_dev_up || !has_dev_down {
                result.add_warning(format!(
                    "BBANDS indicator {} missing required parameters (period, deviation_up, deviation_down)",
                    indicator.id
                ));
            }
        },
        "SMA" | "EMA" | "WMA" | "TEMA" => {
            // Check for required parameters
            if !indicator.parameters.get("period").is_some() {
                result.add_warning(format!(
                    "{} indicator {} missing 'period' parameter", 
                    indicator.indicator_name,
                    indicator.id
                ));
            }
        },
        "ATR" | "NATR" => {
            // Check for required parameters
            if !indicator.parameters.get("period").is_some() {
                result.add_warning(format!(
                    "{} indicator {} missing 'period' parameter", 
                    indicator.indicator_name,
                    indicator.id
                ));
            }
        },
        "STOCH" => {
            // Check for required parameters
            let has_k = indicator.parameters.get("k_period").is_some();
            let has_d = indicator.parameters.get("d_period").is_some();
            let has_slowing = indicator.parameters.get("slowing").is_some();
            
            if !has_k || !has_d || !has_slowing {
                result.add_warning(format!(
                    "STOCH indicator {} missing required parameters (k_period, d_period, slowing)",
                    indicator.id
                ));
            }
        },
        // Add more indicators as needed
        _ => {}
    }
}

/// Validate rules in a strategy
fn validate_rules(strategy: &Strategy, result: &mut ValidationResult) {
    let mut rule_ids = HashSet::new();
    let indicator_ids: HashSet<&String> = strategy.indicators.iter()
        .map(|i| &i.id)
        .collect();
    
    for rule in &strategy.rules {
        // Check ID
        if rule.id.is_empty() {
            result.add_error("Rule ID is empty");
            continue;
        }
        
        // Check for duplicate IDs
        if !rule_ids.insert(&rule.id) {
            result.add_error(format!("Duplicate rule ID: {}", rule.id));
        }
        
        // Check name
        if rule.name.is_empty() {
            result.add_error(format!("Rule {} has empty name", rule.id));
        }
        
        // Validate condition
        validate_condition(&rule.condition, &indicator_ids, result);
    }
}

/// Validate a composite condition
fn validate_condition(
    condition: &CompositeCondition, 
    indicator_ids: &HashSet<&String>, 
    result: &mut ValidationResult
) {
    match condition {
        CompositeCondition::Simple { condition } => {
            validate_simple_condition(condition, indicator_ids, result);
        },
        CompositeCondition::Compound { operator: _, conditions } => {
            for cond in conditions {
                validate_condition(cond, indicator_ids, result);
            }
        }
    }
}

/// Validate a simple condition
fn validate_simple_condition(
    condition: &Condition, 
    indicator_ids: &HashSet<&String>, 
    result: &mut ValidationResult
) {
    // Validate left value source
    validate_value_source(&condition.left, indicator_ids, result);
    
    // Validate right value source
    validate_value_source(&condition.right, indicator_ids, result);
}

/// Validate a value source
fn validate_value_source(
    source: &ValueSource, 
    indicator_ids: &HashSet<&String>, 
    result: &mut ValidationResult
) {
    match source {
        ValueSource::Indicator { indicator_id, property: _, offset: _ } => {
            // Check if the indicator ID exists
            if !indicator_ids.contains(indicator_id) {
                result.add_error(format!(
                    "Rule references unknown indicator: {}", 
                    indicator_id
                ));
            }
        },
        ValueSource::Price { property, offset: _ } => {
            // Check if the price property is valid
            match property.as_str() {
                "open" | "high" | "low" | "close" | "volume" => {},
                _ => result.add_warning(format!(
                    "Unknown price property: {}", 
                    property
                )),
            }
        },
        ValueSource::Parameter { parameter_id: _ } => {
            // Parameter validation would require checking against strategy.parameters
            // This is left as a future enhancement
        },
        ValueSource::Constant { value: _ } => {
            // Constants don't need validation
        }
    }
}

/// Validate risk management settings
fn validate_risk_management(risk_management: &RiskManagement, result: &mut ValidationResult) {
    // Check max risk per trade
    if risk_management.max_risk_per_trade <= 0.0 || risk_management.max_risk_per_trade > 100.0 {
        result.add_warning(format!(
            "Invalid max_risk_per_trade: {}%. Should be between 0 and 100", 
            risk_management.max_risk_per_trade
        ));
    }
    
    // Check max total risk
    if risk_management.max_total_risk <= 0.0 || risk_management.max_total_risk > 100.0 {
        result.add_warning(format!(
            "Invalid max_total_risk: {}%. Should be between 0 and 100", 
            risk_management.max_total_risk
        ));
    }
    
    // Check default position size
    if risk_management.default_position_size <= 0.0 || risk_management.default_position_size > 100.0 {
        result.add_warning(format!(
            "Invalid default_position_size: {}%. Should be between 0 and 100", 
            risk_management.default_position_size
        ));
    }
    
    // Check stop loss and take profit
    if let Some(sl) = risk_management.default_stop_loss {
        if sl <= 0.0 || sl > 100.0 {
            result.add_warning(format!(
                "Invalid default_stop_loss: {}%. Should be between 0 and 100", 
                sl
            ));
        }
    }
    
    if let Some(tp) = risk_management.default_take_profit {
        if tp <= 0.0 || tp > 100.0 {
            result.add_warning(format!(
                "Invalid default_take_profit: {}%. Should be between 0 and 100", 
                tp
            ));
        }
    }
    
    // Check trailing stop settings
    if risk_management.use_trailing_stop {
        if risk_management.trailing_stop_activation.is_none() {
            result.add_warning("Trailing stop is enabled but activation percentage is not set");
        }
        
        if risk_management.trailing_stop_percent.is_none() {
            result.add_warning("Trailing stop is enabled but trailing percentage is not set");
        }
    }
}

/// Result of strategy validation containing errors and warnings
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// Add an error message
    pub fn add_error<S: Into<String>>(&mut self, message: S) {
        self.errors.push(message.into());
    }
    
    /// Add a warning message
    pub fn add_warning<S: Into<String>>(&mut self, message: S) {
        self.warnings.push(message.into());
    }
    
    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    /// Convert validation result to anyhow::Result
    pub fn to_result(self) -> Result<()> {
        if self.has_errors() {
            Err(anyhow!("Strategy validation failed: {}", self.errors.join(", ")))
        } else {
            Ok(())
        }
    }
    
    /// Get a summary of validation issues
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        
        if self.has_errors() {
            summary.push_str(&format!("Errors ({}):\n", self.errors.len()));
            for (i, error) in self.errors.iter().enumerate() {
                summary.push_str(&format!("  {}. {}\n", i + 1, error));
            }
        }
        
        if self.has_warnings() {
            if !summary.is_empty() {
                summary.push('\n');
            }
            summary.push_str(&format!("Warnings ({}):\n", self.warnings.len()));
            for (i, warning) in self.warnings.iter().enumerate() {
                summary.push_str(&format!("  {}. {}\n", i + 1, warning));
            }
        }
        
        if summary.is_empty() {
            summary.push_str("Strategy validation passed without issues.");
        }
        
        summary
    }
}

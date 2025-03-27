// src/strategy/schema.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete trading strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Unique identifier for the strategy
    pub id: String,
    /// Human-readable name of the strategy
    pub name: String,
    /// Description of the strategy
    pub description: String,
    /// Version of this strategy definition
    pub version: String,
    /// Author of the strategy
    pub author: String,
    /// When the strategy was created
    pub created_at: DateTime<Utc>,
    /// When the strategy was last updated
    pub updated_at: DateTime<Utc>,
    /// Whether the strategy is enabled
    pub enabled: bool,
    /// Assets this strategy is designed for
    pub assets: Vec<String>,
    /// Timeframes this strategy is designed for
    pub timeframes: Vec<String>,
    /// List of indicator configurations used by this strategy
    pub indicators: Vec<StrategyIndicator>,
    /// Rules that define when to enter or exit positions
    pub rules: Vec<StrategyRule>,
    /// Strategy parameters that can be tuned
    pub parameters: HashMap<String, StrategyParameter>,
    /// Risk management settings
    pub risk_management: RiskManagement,
    /// Performance metrics during backtesting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<StrategyPerformance>,
    /// Custom metadata for the strategy
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Represents an indicator used within a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyIndicator {
    /// Unique identifier for this indicator within the strategy
    pub id: String,
    /// Type of indicator (must match a valid indicator_type)
    pub indicator_type: String,
    /// Name of the indicator (must match a valid indicator_name)
    pub indicator_name: String,
    /// Parameters for the indicator
    pub parameters: serde_json::Value,
    /// Human-readable description of how this indicator is used
    pub description: String,
}

/// Types of operations for comparing values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    #[serde(rename = "=")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = "<=")]
    LessThanOrEqual,
    #[serde(rename = "crosses_above")]
    CrossesAbove,
    #[serde(rename = "crosses_below")]
    CrossesBelow,
}

/// Types of logical operators for combining conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

/// Represents a value source for a condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ValueSource {
    #[serde(rename = "indicator")]
    Indicator {
        indicator_id: String,
        property: Option<String>,
        offset: Option<i32>,
    },
    #[serde(rename = "price")]
    Price {
        property: String, // "open", "high", "low", "close", "volume"
        offset: Option<i32>,
    },
    #[serde(rename = "parameter")]
    Parameter {
        parameter_id: String,
    },
    #[serde(rename = "constant")]
    Constant {
        value: serde_json::Value,
    },
}

/// Represents a condition in a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub left: ValueSource,
    pub operator: ComparisonOperator,
    pub right: ValueSource,
}

/// Represents a composite condition with logical operators
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompositeCondition {
    #[serde(rename = "simple")]
    Simple { condition: Condition },
    #[serde(rename = "composite")]
    Compound {
        operator: LogicalOperator,
        conditions: Vec<CompositeCondition>,
    },
}

/// Possible actions for a strategy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleAction {
    #[serde(rename = "enter_long")]
    EnterLong {
        #[serde(default)]
        size_percent: Option<f64>,
    },
    #[serde(rename = "enter_short")]
    EnterShort {
        #[serde(default)]
        size_percent: Option<f64>,
    },
    #[serde(rename = "exit_long")]
    ExitLong {
        #[serde(default)]
        size_percent: Option<f64>,
    },
    #[serde(rename = "exit_short")]
    ExitShort {
        #[serde(default)]
        size_percent: Option<f64>,
    },
    #[serde(rename = "set_stop_loss")]
    SetStopLoss {
        #[serde(default)]
        percent: Option<f64>,
        #[serde(default)]
        price: Option<f64>,
    },
    #[serde(rename = "set_take_profit")]
    SetTakeProfit {
        #[serde(default)]
        percent: Option<f64>,
        #[serde(default)]
        price: Option<f64>,
    },
}

/// Represents a single rule within a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRule {
    /// Unique identifier for this rule
    pub id: String,
    /// Name of the rule
    pub name: String,
    /// Condition for when this rule should trigger
    pub condition: CompositeCondition,
    /// Action to take when the condition is met
    pub action: RuleAction,
    /// Priority of this rule (lower numbers have higher priority)
    #[serde(default)]
    pub priority: i32,
    /// Optional description of the rule
    #[serde(default)]
    pub description: String,
}

/// Represents a parameter that can be tuned in the strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StrategyParameter {
    #[serde(rename = "integer")]
    Integer {
        value: i64,
        min: i64,
        max: i64,
        description: String,
    },
    #[serde(rename = "float")]
    Float {
        value: f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        description: String,
    },
    #[serde(rename = "boolean")]
    Boolean {
        value: bool,
        description: String,
    },
    #[serde(rename = "string")]
    String {
        value: String,
        options: Option<Vec<String>>,
        description: String,
    },
}

/// Risk management settings for the strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagement {
    /// Maximum percentage of account to risk per trade
    pub max_risk_per_trade: f64,
    /// Maximum percentage of account to use at once
    pub max_total_risk: f64,
    /// Default position size as a percentage of available capital
    pub default_position_size: f64,
    /// Default stop loss percentage
    pub default_stop_loss: Option<f64>,
    /// Default take profit percentage
    pub default_take_profit: Option<f64>,
    /// Whether to use trailing stops
    pub use_trailing_stop: bool,
    /// Trailing stop activation percentage
    pub trailing_stop_activation: Option<f64>,
    /// Trailing stop percentage
    pub trailing_stop_percent: Option<f64>,
}

/// Performance metrics for a strategy during backtesting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    /// Total number of completed trades
    pub total_trades: i32,
    /// Number of winning trades
    pub winning_trades: i32,
    /// Number of losing trades
    pub losing_trades: i32,
    /// Win rate (percentage)
    pub win_rate: f64,
    /// Maximum drawdown (percentage)
    pub max_drawdown: f64,
    /// Profit factor (gross profits / gross losses)
    pub profit_factor: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Total return (percentage)
    pub total_return: f64,
    /// Annualized return (percentage)
    pub annualized_return: f64,
    /// Maximum consecutive wins
    pub max_consecutive_wins: i32,
    /// Maximum consecutive losses
    pub max_consecutive_losses: i32,
    /// Average profit per winning trade (percentage)
    pub avg_profit_per_win: f64,
    /// Average loss per losing trade (percentage)
    pub avg_loss_per_loss: f64,
    /// Average holding period for winning trades (in hours)
    pub avg_win_holding_period: f64,
    /// Average holding period for losing trades (in hours)
    pub avg_loss_holding_period: f64,
    /// Expectancy (average profit/loss per trade)
    pub expectancy: f64,
}

/// Create a new strategy with default values
impl Default for Strategy {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Strategy".to_string(),
            description: "".to_string(),
            version: "1.0.0".to_string(),
            author: "".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            enabled: true,
            assets: Vec::new(),
            timeframes: Vec::new(),
            indicators: Vec::new(),
            rules: Vec::new(),
            parameters: HashMap::new(),
            risk_management: RiskManagement::default(),
            performance: None,
            metadata: HashMap::new(),
        }
    }
}

impl Default for RiskManagement {
    fn default() -> Self {
        Self {
            max_risk_per_trade: 2.0,
            max_total_risk: 10.0,
            default_position_size: 5.0,
            default_stop_loss: Some(2.0),
            default_take_profit: Some(6.0),
            use_trailing_stop: false,
            trailing_stop_activation: None,
            trailing_stop_percent: None,
        }
    }
}

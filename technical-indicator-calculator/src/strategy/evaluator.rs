// src/strategy/evaluator.rs
use crate::database::models::CandleData;
use crate::database::postgres::PostgresManager;
use crate::indicators::calculator::IndicatorCalculator;
use crate::strategy::schema::{Strategy, StrategyIndicator, StrategyPerformance, ValueSource, CompositeCondition, Condition, ComparisonOperator, LogicalOperator, RuleAction, StrategyParameter};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Represents a position in the market
#[derive(Debug, Clone)]
struct Position {
    /// Whether the position is long or short
    is_long: bool,
    /// Entry price
    entry_price: f64,
    /// Size of the position as percentage of capital
    size_percent: f64,
    /// Entry time
    entry_time: DateTime<Utc>,
    /// Stop loss price, if any
    stop_loss: Option<f64>,
    /// Take profit price, if any
    take_profit: Option<f64>,
}

/// Result of a completed trade
#[derive(Debug, Clone)]
struct TradeResult {
    /// Whether the trade was long or short
    is_long: bool,
    /// Entry price
    entry_price: f64,
    /// Exit price
    exit_price: f64,
    /// Size of the position as percentage of capital
    size_percent: f64,
    /// Entry time
    entry_time: DateTime<Utc>,
    /// Exit time
    exit_time: DateTime<Utc>,
    /// Reason for exiting the trade
    exit_reason: String,
    /// Profit/loss as percentage
    pl_percent: f64,
}

/// Evaluator for backtesting strategies
pub struct StrategyEvaluator {
    pg: Arc<PostgresManager>,
    initial_capital: f64,
}

impl StrategyEvaluator {
    /// Create a new strategy evaluator
    pub fn new(pg: Arc<PostgresManager>, initial_capital: f64) -> Self {
        Self {
            pg,
            initial_capital,
        }
    }

    /// Backtest a strategy on a symbol and interval
    pub async fn backtest(&self, strategy: &Strategy, symbol: &str, interval: &str, 
                         start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> Result<StrategyPerformance> {
        info!("Starting backtest for strategy {} on {}:{}", strategy.name, symbol, interval);
        
        // For now we'll just return a placeholder result
        info!("(Note: This is a placeholder implementation)");
        
        let performance = StrategyPerformance {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            max_drawdown: 0.0,
            profit_factor: 0.0,
            sharpe_ratio: 0.0,
            total_return: 0.0,
            annualized_return: 0.0,
            max_consecutive_wins: 0,
            max_consecutive_losses: 0,
            avg_profit_per_win: 0.0,
            avg_loss_per_loss: 0.0,
            avg_win_holding_period: 0.0,
            avg_loss_holding_period: 0.0,
            expectancy: 0.0,
        };
        
        Ok(performance)
    }
    
    /// Filter candle data based on date range
    fn filter_candle_data(&self, candle_data: &CandleData, start_date: Option<DateTime<Utc>>, 
                          end_date: Option<DateTime<Utc>>) -> Result<(CandleData, usize, usize)> {
        let mut filtered = CandleData::new(candle_data.symbol.clone(), candle_data.interval.clone());
        
        let start_idx = match start_date {
            Some(date) => candle_data.open_time.iter()
                .position(|t| t >= &date)
                .unwrap_or(0),
            None => 0,
        };
        
        let end_idx = match end_date {
            Some(date) => candle_data.open_time.iter()
                .position(|t| t > &date)
                .unwrap_or(candle_data.open_time.len()),
            None => candle_data.open_time.len(),
        };
        
        if start_idx >= end_idx {
            return Err(anyhow::anyhow!("Invalid date range: start_date must be before end_date"));
        }
        
        // Copy the data within the range
        filtered.open_time = candle_data.open_time[start_idx..end_idx].to_vec();
        filtered.open = candle_data.open[start_idx..end_idx].to_vec();
        filtered.high = candle_data.high[start_idx..end_idx].to_vec();
        filtered.low = candle_data.low[start_idx..end_idx].to_vec();
        filtered.close = candle_data.close[start_idx..end_idx].to_vec();
        filtered.volume = candle_data.volume[start_idx..end_idx].to_vec();
        filtered.close_time = candle_data.close_time[start_idx..end_idx].to_vec();
        
        Ok((filtered, start_idx, end_idx))
    }
    
    /// Calculate all indicators for the strategy
    async fn calculate_indicators(&self, strategy: &Strategy, candle_data: &CandleData) 
        -> Result<HashMap<String, Vec<Value>>> {
        let mut indicators_map = HashMap::new();
        
        for indicator in &strategy.indicators {
            let indicator_values = self.calculate_indicator(indicator, candle_data).await?;
            indicators_map.insert(indicator.id.clone(), indicator_values);
        }
        
        Ok(indicators_map)
    }
    
    /// Calculate a single indicator
    async fn calculate_indicator(&self, indicator: &StrategyIndicator, candle_data: &CandleData) 
        -> Result<Vec<Value>> {
        let results = IndicatorCalculator::calculate_indicator(
            candle_data,
            &indicator.indicator_name,
            &indicator.parameters,
        ).context(format!("Failed to calculate indicator {}", indicator.indicator_name))?;
        
        // Convert from (DateTime, Value) to just Value
        let values = results.into_iter()
            .map(|(_, value)| value)
            .collect();
        
        Ok(values)
    }
    
    /// Simulate trading based on strategy rules
    fn simulate_trades(&self, strategy: &Strategy, candle_data: &CandleData, 
                     indicators_map: &HashMap<String, Vec<Value>>, _start_idx: usize, _end_idx: usize) 
        -> Result<Vec<TradeResult>> {
        let mut trade_results = Vec::new();
        let mut current_position: Option<Position> = None;
        let mut equity = self.initial_capital;
        
        // We need a warmup period to have all indicators ready
        // Find the longest indicator length to determine warmup
        let warmup = indicators_map.values()
            .map(|values| candle_data.close.len().saturating_sub(values.len()))
            .max()
            .unwrap_or(0);
        
        let first_tradeable_idx = warmup;
        
        // Track max equity for drawdown calculation
        let mut max_equity = equity;
        
        // Simulate candle by candle
        for i in first_tradeable_idx..candle_data.close.len() {
            let candle_time = candle_data.open_time[i];
            let _open_price = candle_data.open[i];
            let high_price = candle_data.high[i];
            let low_price = candle_data.low[i];
            let close_price = candle_data.close[i];
            let _volume = candle_data.volume[i];
            
            // Check if we need to close position due to stop loss or take profit
            if let Some(position) = &current_position {
                let mut exit_reason = None;
                
                // Check stop loss
                if let Some(stop_loss) = position.stop_loss {
                    // For long positions, stop loss is triggered if price goes below stop level
                    if position.is_long && low_price <= stop_loss {
                        exit_reason = Some("Stop Loss".to_string());
                    } 
                    // For short positions, stop loss is triggered if price goes above stop level
                    else if !position.is_long && high_price >= stop_loss {
                        exit_reason = Some("Stop Loss".to_string());
                    }
                }
                
                // Check take profit
                if exit_reason.is_none() {
                    if let Some(take_profit) = position.take_profit {
                        // For long positions, take profit is triggered if price goes above take profit level
                        if position.is_long && high_price >= take_profit {
                            exit_reason = Some("Take Profit".to_string());
                        } 
                        // For short positions, take profit is triggered if price goes below take profit level
                        else if !position.is_long && low_price <= take_profit {
                            exit_reason = Some("Take Profit".to_string());
                        }
                    }
                }
                
                // Exit position if needed
                if let Some(reason) = exit_reason {
                    // Calculate P/L
                    let exit_price = if reason == "Stop Loss" {
                        position.stop_loss.unwrap_or(close_price)
                    } else if reason == "Take Profit" {
                        position.take_profit.unwrap_or(close_price)
                    } else {
                        close_price
                    };
                    
                    let pl_percent = if position.is_long {
                        (exit_price - position.entry_price) / position.entry_price * 100.0
                    } else {
                        (position.entry_price - exit_price) / position.entry_price * 100.0
                    };
                    
                    // Record the trade
                    trade_results.push(TradeResult {
                        is_long: position.is_long,
                        entry_price: position.entry_price,
                        exit_price,
                        size_percent: position.size_percent,
                        entry_time: position.entry_time,
                        exit_time: candle_time,
                        exit_reason: reason,
                        pl_percent,
                    });
                    
                    // Update equity
                    let position_value = equity * (position.size_percent / 100.0);
                    let profit_loss = position_value * (pl_percent / 100.0);
                    equity += profit_loss;
                    
                    // Update max equity
                    max_equity = max_equity.max(equity);
                    
                    // Clear position
                    current_position = None;
                }
            }
            
            // Create a context for evaluating rules
            let context = self.create_rule_evaluation_context(
                i, candle_data, indicators_map, &strategy.parameters,
            )?;
            
            // Evaluate rules in priority order
            let mut rules = strategy.rules.clone();
            rules.sort_by_key(|rule| rule.priority);
            
            for rule in &rules {
                // Skip rules that don't apply to our current position state
                match &rule.action {
                    RuleAction::EnterLong { .. } | RuleAction::EnterShort { .. } 
                        if current_position.is_some() => continue,
                    RuleAction::ExitLong { .. } 
                        if current_position.is_none() || !current_position.as_ref().unwrap().is_long => continue,
                    RuleAction::ExitShort { .. } 
                        if current_position.is_none() || current_position.as_ref().unwrap().is_long => continue,
                    _ => {}
                }
                
                // Evaluate the rule condition
                if self.evaluate_condition(&rule.condition, &context)? {
                    // Execute the rule action
                    match &rule.action {
                        RuleAction::EnterLong { size_percent } => {
                            let size = size_percent.unwrap_or(strategy.risk_management.default_position_size);
                            
                            // Calculate stop loss and take profit if configured
                            let stop_loss = if let Some(sl_pct) = strategy.risk_management.default_stop_loss {
                                Some(close_price * (1.0 - sl_pct / 100.0))
                            } else {
                                None
                            };
                            
                            let take_profit = if let Some(tp_pct) = strategy.risk_management.default_take_profit {
                                Some(close_price * (1.0 + tp_pct / 100.0))
                            } else {
                                None
                            };
                            
                            current_position = Some(Position {
                                is_long: true,
                                entry_price: close_price,
                                size_percent: size,
                                entry_time: candle_time,
                                stop_loss,
                                take_profit,
                            });
                        },
                        RuleAction::EnterShort { size_percent } => {
                            let size = size_percent.unwrap_or(strategy.risk_management.default_position_size);
                            
                            // Calculate stop loss and take profit if configured
                            let stop_loss = if let Some(sl_pct) = strategy.risk_management.default_stop_loss {
                                Some(close_price * (1.0 + sl_pct / 100.0))
                            } else {
                                None
                            };
                            
                            let take_profit = if let Some(tp_pct) = strategy.risk_management.default_take_profit {
                                Some(close_price * (1.0 - tp_pct / 100.0))
                            } else {
                                None
                            };
                            
                            current_position = Some(Position {
                                is_long: false,
                                entry_price: close_price,
                                size_percent: size,
                                entry_time: candle_time,
                                stop_loss,
                                take_profit,
                            });
                        },
                        RuleAction::ExitLong { size_percent } | RuleAction::ExitShort { size_percent } => {
                            if let Some(position) = &current_position {
                                let size = size_percent.unwrap_or(100.0);
                                
                                if size >= 100.0 {
                                    // Full exit
                                    let pl_percent = if position.is_long {
                                        (close_price - position.entry_price) / position.entry_price * 100.0
                                    } else {
                                        (position.entry_price - close_price) / position.entry_price * 100.0
                                    };
                                    
                                    trade_results.push(TradeResult {
                                        is_long: position.is_long,
                                        entry_price: position.entry_price,
                                        exit_price: close_price,
                                        size_percent: position.size_percent,
                                        entry_time: position.entry_time,
                                        exit_time: candle_time,
                                        exit_reason: "Rule Based Exit".to_string(),
                                        pl_percent,
                                    });
                                    
                                    // Update equity
                                    let position_value = equity * (position.size_percent / 100.0);
                                    let profit_loss = position_value * (pl_percent / 100.0);
                                    equity += profit_loss;
                                    
                                    // Update max equity
                                    max_equity = max_equity.max(equity);
                                    
                                    current_position = None;
                                } else {
                                    // Partial exit - not implemented yet
                                    // Would need to track multiple positions
                                    warn!("Partial exits not supported yet");
                                }
                            }
                        },
                        RuleAction::SetStopLoss { percent, price } => {
                            if let Some(position) = &mut current_position {
                                if let Some(pct) = percent {
                                    position.stop_loss = if position.is_long {
                                        Some(position.entry_price * (1.0 - pct / 100.0))
                                    } else {
                                        Some(position.entry_price * (1.0 + pct / 100.0))
                                    };
                                } else if let Some(p) = price {
                                    position.stop_loss = Some(*p);
                                }
                            }
                        },
                        RuleAction::SetTakeProfit { percent, price } => {
                            if let Some(position) = &mut current_position {
                                if let Some(pct) = percent {
                                    position.take_profit = if position.is_long {
                                        Some(position.entry_price * (1.0 + pct / 100.0))
                                    } else {
                                        Some(position.entry_price * (1.0 - pct / 100.0))
                                    };
                                } else if let Some(p) = price {
                                    position.take_profit = Some(*p);
                                }
                            }
                        },
                    }
                }
            }
        }
        
        // Close any open positions at the end of the simulation
        if let Some(position) = &current_position {
            let last_idx = candle_data.close.len() - 1;
            let close_price = candle_data.close[last_idx];
            let candle_time = candle_data.open_time[last_idx];
            
            let pl_percent = if position.is_long {
                (close_price - position.entry_price) / position.entry_price * 100.0
            } else {
                (position.entry_price - close_price) / position.entry_price * 100.0
            };
            
            trade_results.push(TradeResult {
                is_long: position.is_long,
                entry_price: position.entry_price,
                exit_price: close_price,
                size_percent: position.size_percent,
                entry_time: position.entry_time,
                exit_time: candle_time,
                exit_reason: "End of Simulation".to_string(),
                pl_percent,
            });
        }
        
        Ok(trade_results)
    }
    
    /// Create a context for evaluating rule conditions
    fn create_rule_evaluation_context(&self, 
                                     idx: usize, 
                                     candle_data: &CandleData,
                                     indicators_map: &HashMap<String, Vec<Value>>,
                                     parameters: &HashMap<String, StrategyParameter>) 
        -> Result<HashMap<String, Value>> {
        let mut context = HashMap::new();
        
        // Add price data
        context.insert("open".to_string(), json!(candle_data.open[idx]));
        context.insert("high".to_string(), json!(candle_data.high[idx]));
        context.insert("low".to_string(), json!(candle_data.low[idx]));
        context.insert("close".to_string(), json!(candle_data.close[idx]));
        context.insert("volume".to_string(), json!(candle_data.volume[idx]));
        
        // Add indicator values
        for (indicator_id, values) in indicators_map {
            // Calculate the offset within the indicator values
            let indicator_idx = values.len().saturating_sub(candle_data.close.len() - idx);
            
            // Only add the indicator if we have a valid index
            if indicator_idx < values.len() {
                context.insert(format!("indicator_{}", indicator_id), values[indicator_idx].clone());
            }
        }
        
        // Add strategy parameters
        for (param_id, param) in parameters {
            // Extract the value based on parameter type
            let value = match param {
                StrategyParameter::Integer { value, .. } => json!(*value),
                StrategyParameter::Float { value, .. } => json!(*value),
                StrategyParameter::Boolean { value, .. } => json!(*value),
                StrategyParameter::String { value, .. } => json!(value),
            };
            
            context.insert(format!("param_{}", param_id), value);
        }
        
        Ok(context)
    }
    
    /// Evaluate a condition based on the evaluation context
    fn evaluate_condition(&self, condition: &CompositeCondition, context: &HashMap<String, Value>) -> Result<bool> {
        match condition {
            CompositeCondition::Simple { condition } => {
                self.evaluate_simple_condition(condition, context)
            },
            CompositeCondition::Compound { operator, conditions } => {
                let results = conditions.iter()
                    .map(|cond| self.evaluate_condition(cond, context))
                    .collect::<Result<Vec<bool>>>()?;
                
                match operator {
                    LogicalOperator::And => Ok(results.iter().all(|&result| result)),
                    LogicalOperator::Or => Ok(results.iter().any(|&result| result)),
                }
            },
        }
    }
    
    /// Evaluate a simple condition
    fn evaluate_simple_condition(&self, condition: &Condition, context: &HashMap<String, Value>) -> Result<bool> {
        let left_value = self.resolve_value(&condition.left, context)?;
        let right_value = self.resolve_value(&condition.right, context)?;
        
        match condition.operator {
            ComparisonOperator::Equal => Ok(left_value == right_value),
            ComparisonOperator::NotEqual => Ok(left_value != right_value),
            ComparisonOperator::GreaterThan => {
                // Convert to f64 for numerical comparison
                let left_num = self.to_f64(&left_value)?;
                let right_num = self.to_f64(&right_value)?;
                Ok(left_num > right_num)
            },
            ComparisonOperator::GreaterThanOrEqual => {
                let left_num = self.to_f64(&left_value)?;
                let right_num = self.to_f64(&right_value)?;
                Ok(left_num >= right_num)
            },
            ComparisonOperator::LessThan => {
                let left_num = self.to_f64(&left_value)?;
                let right_num = self.to_f64(&right_value)?;
                Ok(left_num < right_num)
            },
            ComparisonOperator::LessThanOrEqual => {
                let left_num = self.to_f64(&left_value)?;
                let right_num = self.to_f64(&right_value)?;
                Ok(left_num <= right_num)
            },
            ComparisonOperator::CrossesAbove => {
                // Not implemented yet - would need historical values
                warn!("CrossesAbove not fully implemented yet");
                Ok(false)
            },
            ComparisonOperator::CrossesBelow => {
                // Not implemented yet - would need historical values
                warn!("CrossesBelow not fully implemented yet");
                Ok(false)
            },
        }
    }
    
    /// Resolve a value source to an actual value
    fn resolve_value(&self, source: &ValueSource, context: &HashMap<String, Value>) -> Result<Value> {
        match source {
            ValueSource::Indicator { indicator_id, property, offset: _ } => {
                let indicator_key = format!("indicator_{}", indicator_id);
                let indicator_value = context.get(&indicator_key)
                    .ok_or_else(|| anyhow::anyhow!("Indicator not found: {}", indicator_id))?;
                
                if let Some(prop) = property {
                    // If indicator returns a complex object, extract the property
                    if let Value::Object(obj) = indicator_value {
                        obj.get(prop)
                            .ok_or_else(|| anyhow::anyhow!("Property not found in indicator: {}", prop))
                            .map(|v| v.clone())
                    } else {
                        return Err(anyhow::anyhow!("Indicator value is not an object but property was requested"));
                    }
                } else {
                    // Return the whole value
                    Ok(indicator_value.clone())
                }
            },
            ValueSource::Price { property, offset: _ } => {
                // Get price data
                context.get(property)
                    .ok_or_else(|| anyhow::anyhow!("Price property not found: {}", property))
                    .map(|v| v.clone())
            },
            ValueSource::Parameter { parameter_id } => {
                let param_key = format!("param_{}", parameter_id);
                context.get(&param_key)
                    .ok_or_else(|| anyhow::anyhow!("Parameter not found: {}", parameter_id))
                    .map(|v| v.clone())
            },
            ValueSource::Constant { value } => {
                Ok(value.clone())
            },
        }
    }
    
    /// Convert a JSON value to f64 for numerical comparison
    fn to_f64(&self, value: &Value) -> Result<f64> {
        match value {
            Value::Number(n) => n.as_f64().ok_or_else(|| anyhow::anyhow!("Failed to convert number to f64")),
            Value::String(s) => s.parse::<f64>().map_err(|_| anyhow::anyhow!("Failed to parse string as f64")),
            _ => Err(anyhow::anyhow!("Value cannot be converted to f64")),
        }
    }
    
    /// Calculate performance metrics from trade results
    fn calculate_performance(&self, trade_results: Vec<TradeResult>, _candle_data: &CandleData) -> Result<StrategyPerformance> {
        if trade_results.is_empty() {
            return Ok(StrategyPerformance {
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                win_rate: 0.0,
                max_drawdown: 0.0,
                profit_factor: 0.0,
                sharpe_ratio: 0.0,
                total_return: 0.0,
                annualized_return: 0.0,
                max_consecutive_wins: 0,
                max_consecutive_losses: 0,
                avg_profit_per_win: 0.0,
                avg_loss_per_loss: 0.0,
                avg_win_holding_period: 0.0,
                avg_loss_holding_period: 0.0,
                expectancy: 0.0,
            });
        }
        
        // Calculate basic metrics
        let total_trades = trade_results.len() as i32;
        
        let winning_trades = trade_results.iter()
            .filter(|t| t.pl_percent > 0.0)
            .count() as i32;
            
        let losing_trades = trade_results.iter()
            .filter(|t| t.pl_percent <= 0.0)
            .count() as i32;
            
        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };
        
        // Calculate P/L metrics
        let gross_profit: f64 = trade_results.iter()
            .filter(|t| t.pl_percent > 0.0)
            .map(|t| t.pl_percent)
            .sum();
            
        let gross_loss: f64 = trade_results.iter()
            .filter(|t| t.pl_percent <= 0.0)
            .map(|t| t.pl_percent.abs())
            .sum();
            
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        // Calculate consecutive win/loss streaks
        let mut current_streak = 0;
        let mut max_win_streak = 0;
        let mut max_loss_streak = 0;
        let mut is_winning = false;
        
        for trade in &trade_results {
            if current_streak == 0 {
                current_streak = 1;
                is_winning = trade.pl_percent > 0.0;
            } else if (trade.pl_percent > 0.0) == is_winning {
                current_streak += 1;
            } else {
                if is_winning {
                    max_win_streak = max_win_streak.max(current_streak);
                } else {
                    max_loss_streak = max_loss_streak.max(current_streak);
                }
                current_streak = 1;
                is_winning = trade.pl_percent > 0.0;
            }
        }
        
        // Don't forget the last streak
        if is_winning {
            max_win_streak = max_win_streak.max(current_streak);
        } else {
            max_loss_streak = max_loss_streak.max(current_streak);
        }
        
        // Calculate average profit/loss
        let avg_profit_per_win = if winning_trades > 0 {
            gross_profit / winning_trades as f64
        } else {
            0.0
        };
        
        let avg_loss_per_loss = if losing_trades > 0 {
            gross_loss / losing_trades as f64
        } else {
            0.0
        };
        
        // Calculate holding periods
        let avg_win_holding_period = trade_results.iter()
            .filter(|t| t.pl_percent > 0.0)
            .map(|t| (t.exit_time - t.entry_time).num_hours() as f64)
            .sum::<f64>() / winning_trades.max(1) as f64;
            
        let avg_loss_holding_period = trade_results.iter()
            .filter(|t| t.pl_percent <= 0.0)
            .map(|t| (t.exit_time - t.entry_time).num_hours() as f64)
            .sum::<f64>() / losing_trades.max(1) as f64;
        
        // Calculate expectancy
        let expectancy = win_rate / 100.0 * avg_profit_per_win - (1.0 - win_rate / 100.0) * avg_loss_per_loss;
        
        // Calculate total return
        let mut equity_curve = vec![self.initial_capital];
        let mut max_equity = self.initial_capital;
        let mut current_equity = self.initial_capital;
        let mut max_drawdown: f64 = 0.0;
        
        for trade in &trade_results {
            let position_size = current_equity * (trade.size_percent / 100.0);
            let profit_loss = position_size * (trade.pl_percent / 100.0);
            current_equity += profit_loss;
            equity_curve.push(current_equity);
            
            if current_equity > max_equity {
                max_equity = current_equity;
            } else {
                let drawdown = (max_equity - current_equity) / max_equity * 100.0;
                max_drawdown = max_drawdown.max(drawdown);
            }
        }
        
        let total_return = (current_equity - self.initial_capital) / self.initial_capital * 100.0;
        
        // Calculate annualized return
        let first_trade_time = trade_results.first().map(|t| t.entry_time).unwrap_or_else(Utc::now);
        let last_trade_time = trade_results.last().map(|t| t.exit_time).unwrap_or_else(Utc::now);
        let trading_days = (last_trade_time - first_trade_time).num_days() as f64 / 365.0;
        
        let annualized_return = if trading_days > 0.0 {
            ((1.0 + total_return / 100.0).powf(1.0 / trading_days) - 1.0) * 100.0
        } else {
            0.0
        };
        
        // Calculate Sharpe Ratio (simplified)
        // Using a risk-free rate of 0% for simplicity
        let daily_returns = equity_curve.windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect::<Vec<f64>>();
            
        let mean_return = daily_returns.iter().sum::<f64>() / daily_returns.len().max(1) as f64;
        
        let variance = daily_returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / daily_returns.len().max(1) as f64;
            
        let std_dev = variance.sqrt();
        
        let sharpe_ratio = if std_dev > 0.0 {
            mean_return / std_dev * (252.0_f64).sqrt()  // Annualize with sqrt(252) trading days
        } else if mean_return > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        Ok(StrategyPerformance {
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            max_drawdown,
            profit_factor,
            sharpe_ratio,
            total_return,
            annualized_return,
            max_consecutive_wins: max_win_streak,
            max_consecutive_losses: max_loss_streak,
            avg_profit_per_win,
            avg_loss_per_loss,
            avg_win_holding_period,
            avg_loss_holding_period,
            expectancy,
        })
    }
}

// src/strategy/repository.rs
use crate::database::postgres::PostgresManager;
use crate::strategy::schema::{Strategy, StrategyPerformance};
use crate::strategy::repository_helpers::{
    load_strategy_indicators, load_strategy_rules, save_strategy_indicators, 
    save_strategy_rules, parse_strategy_json
};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use sqlx::Row; // Add this import

/// Repository for storing and retrieving strategies from the database
pub struct StrategyRepository {
    pg: Arc<PostgresManager>,
}

impl StrategyRepository {
    /// Create a new repository
    pub fn new(pg: Arc<PostgresManager>) -> Self {
        Self { pg }
    }
    
    /// Get the database connection
    pub fn get_db_connection(&self) -> Arc<PostgresManager> {
        self.pg.clone()
    }
    
    /// List all strategies
    pub async fn list_strategies(&self, enabled_only: bool) -> Result<Vec<Strategy>> {
        info!("Listing strategies (enabled_only: {})", enabled_only);
        
        let rows = if enabled_only {
            self.pg.execute_query(
                "SELECT id, name, description, version, author, created_at, updated_at, 
                        enabled, assets, timeframes, parameters, risk_management, metadata
                 FROM strategies
                 WHERE enabled = true
                 ORDER BY name"
            ).await?
        } else {
            self.pg.execute_query(
                "SELECT id, name, description, version, author, created_at, updated_at, 
                        enabled, assets, timeframes, parameters, risk_management, metadata
                 FROM strategies
                 ORDER BY name"
            ).await?
        };
        
        let mut strategies = Vec::with_capacity(rows.len());
        
        for row in rows {
            let id_str: String = row.get("id");
            let strategy = self.get_strategy(&id_str).await?;
            strategies.push(strategy);
        }
        
        Ok(strategies)
    }
    
    /// Get a strategy by ID
    pub async fn get_strategy(&self, id: &str) -> Result<Strategy> {
        info!("Getting strategy with ID: {}", id);
        
        // First, get the base strategy data
        let strategy_row = self.pg.query_opt_by_string(
            "SELECT id, name, description, version, author, created_at, updated_at, 
                    enabled, assets, timeframes, parameters, risk_management, metadata
             FROM strategies
             WHERE id = $1",
            id
        ).await?;
        
        let strategy_row = match strategy_row {
            Some(row) => row,
            None => return Err(anyhow::anyhow!("Strategy not found with ID: {}", id)),
        };
        
        // Parse the strategy data from the database row
        let mut strategy = parse_strategy_json(strategy_row)?;
        
        // Get indicators for this strategy
        strategy.indicators = load_strategy_indicators(&self.pg, id).await?;
        
        // Get rules for this strategy
        strategy.rules = load_strategy_rules(&self.pg, id).await?;
        
        Ok(strategy)
    }
    
    /// Save a strategy to the database
    pub async fn save_strategy(&self, strategy: &Strategy) -> Result<()> {
        info!("Saving strategy: {} ({})", strategy.name, strategy.id);
        
        // Start a transaction
        let mut tx = self.pg.begin_transaction().await?;
        
        // Convert UUID string to Uuid object
        let id = Uuid::parse_str(&strategy.id)
            .context("Invalid UUID format for strategy ID")?;
            
        // Convert Uuid to String for database query
        let id_str = id.to_string();
        
        // Serialize JSON fields
        let assets_json = serde_json::to_value(&strategy.assets)?;
        let timeframes_json = serde_json::to_value(&strategy.timeframes)?;
        let parameters_json = serde_json::to_value(&strategy.parameters)?;
        let risk_management_json = serde_json::to_value(&strategy.risk_management)?;
        let metadata_json = serde_json::to_value(&strategy.metadata)?;
        
        // Check if strategy exists
        let exists = self.pg.query_opt_by_string(
            "SELECT 1 FROM strategies WHERE id = $1::uuid",
            &id_str
        ).await?.is_some();
        
        if exists {
            // Update existing strategy
            self.pg.execute_tx_insert_strategy(
                &mut tx,
                "UPDATE strategies 
                 SET name = $2, description = $3, version = $4, author = $5, 
                     updated_at = $7, enabled = $8, assets = $9, timeframes = $10, 
                     parameters = $11, risk_management = $12, metadata = $13
                 WHERE id = $1",
                &id_str,
                &strategy.name,
                &strategy.description,
                &strategy.version,
                &strategy.author,
                strategy.created_at,
                Utc::now(),
                strategy.enabled,
                assets_json,
                timeframes_json,
                parameters_json,
                risk_management_json,
                metadata_json
            ).await?;
            
            // Delete existing indicators and rules
            self.pg.execute_tx_command_by_string(
                &mut tx,
                "DELETE FROM strategy_indicators WHERE strategy_id = $1",
                &id_str
            ).await?;
                
            self.pg.execute_tx_command_by_string(
                &mut tx,
                "DELETE FROM strategy_rules WHERE strategy_id = $1",
                &id_str
            ).await?;
        } else {
            // Insert new strategy
            self.pg.execute_tx_insert_strategy(
                &mut tx,
                "INSERT INTO strategies 
                 (id, name, description, version, author, created_at, updated_at, 
                  enabled, assets, timeframes, parameters, risk_management, metadata)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                &id_str,
                &strategy.name,
                &strategy.description,
                &strategy.version,
                &strategy.author,
                strategy.created_at,
                Utc::now(),
                strategy.enabled,
                assets_json,
                timeframes_json,
                parameters_json,
                risk_management_json,
                metadata_json
            ).await?;
        }
        
        // Save indicators and rules
        save_strategy_indicators(&mut tx, &self.pg, id, &strategy.indicators).await?;
        save_strategy_rules(&mut tx, &self.pg, id, &strategy.rules).await?;
        
        // Commit the transaction
        self.pg.commit_transaction(tx).await?;
        
        info!("Strategy saved successfully");
        Ok(())
    }
    
    /// Save backtest results
    pub async fn save_backtest_result(
        &self, 
        strategy_id: &str, 
        symbol: &str, 
        interval: &str,
        start_date: Option<DateTime<Utc>>, 
        end_date: Option<DateTime<Utc>>,
        initial_capital: f64, 
        performance: &StrategyPerformance
    ) -> Result<i32> {
        info!("Saving backtest result for strategy {} on {}:{}", strategy_id, symbol, interval);
        
        // Calculate final capital
        let final_capital = initial_capital * (1.0 + performance.total_return / 100.0);
        
        // Create parameter snapshot JSON
        let parameters_json = serde_json::to_value(performance)?;
        
        // Use the actual start/end dates or default to 30 days ago/now
        let start = start_date.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| Utc::now());
        
        // Insert the backtest result
        let backtest_id = self.pg.execute_save_backtest_result(
            "INSERT INTO strategy_backtest_results
             (strategy_id, symbol, interval, start_date, end_date, initial_capital, 
              final_capital, total_trades, winning_trades, losing_trades, win_rate,
              max_drawdown, profit_factor, sharpe_ratio, total_return, annualized_return,
              max_consecutive_wins, max_consecutive_losses, avg_profit_per_win, 
              avg_loss_per_loss, avg_win_holding_period, avg_loss_holding_period,
              expectancy, parameters_snapshot, created_at)
             VALUES
             ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 
              $18, $19, $20, $21, $22, $23, $24, $25)
             RETURNING id",
            strategy_id,
            symbol,
            interval,
            start,
            end,
            initial_capital,
            final_capital,
            performance.total_trades,
            performance.winning_trades,
            performance.losing_trades,
            performance.win_rate as f32,
            performance.max_drawdown as f32,
            performance.profit_factor as f32,
            performance.sharpe_ratio as f32,
            performance.total_return as f32,
            performance.annualized_return as f32,
            performance.max_consecutive_wins,
            performance.max_consecutive_losses,
            performance.avg_profit_per_win as f32,
            performance.avg_loss_per_loss as f32,
            performance.avg_win_holding_period as f32,
            performance.avg_loss_holding_period as f32,
            performance.expectancy as f32,
            parameters_json,
            Utc::now()
        ).await?;
        
        info!("Backtest result saved with ID: {}", backtest_id);
        
        Ok(backtest_id)
    }
    
    /// Get recent backtest results for a strategy
    pub async fn get_recent_backtest_results(
        &self, 
        strategy_id: &str, 
        limit: i64
    ) -> Result<Vec<(i32, String, String, StrategyPerformance)>> {
        info!("Getting recent backtest results for strategy: {}", strategy_id);
        
        // Query recent backtest results
        let rows = self.pg.query_by_string_and_i64(
            "SELECT id, symbol, interval, 
                    total_trades, winning_trades, losing_trades, win_rate,
                    max_drawdown, profit_factor, sharpe_ratio, total_return, 
                    annualized_return, max_consecutive_wins, max_consecutive_losses, 
                    avg_profit_per_win, avg_loss_per_loss, avg_win_holding_period, 
                    avg_loss_holding_period, expectancy
             FROM strategy_backtest_results
             WHERE strategy_id = $1
             ORDER BY created_at DESC
             LIMIT $2",
            strategy_id,
            limit
        ).await?;
        
        let mut results = Vec::with_capacity(rows.len());
        
        for row in rows {
            let id: i32 = row.get("id");
            let symbol: String = row.get("symbol");
            let interval: String = row.get("interval");
            
            // Create StrategyPerformance from row data
            let performance = StrategyPerformance {
                total_trades: row.get("total_trades"),
                winning_trades: row.get("winning_trades"),
                losing_trades: row.get("losing_trades"),
                win_rate: row.get::<f32, _>("win_rate") as f64,
                max_drawdown: row.get::<f32, _>("max_drawdown") as f64,
                profit_factor: row.get::<f32, _>("profit_factor") as f64,
                sharpe_ratio: row.get::<f32, _>("sharpe_ratio") as f64,
                total_return: row.get::<f32, _>("total_return") as f64,
                annualized_return: row.get::<f32, _>("annualized_return") as f64,
                max_consecutive_wins: row.get("max_consecutive_wins"),
                max_consecutive_losses: row.get("max_consecutive_losses"),
                avg_profit_per_win: row.get::<f32, _>("avg_profit_per_win") as f64,
                avg_loss_per_loss: row.get::<f32, _>("avg_loss_per_loss") as f64,
                avg_win_holding_period: row.get::<f32, _>("avg_win_holding_period") as f64,
                avg_loss_holding_period: row.get::<f32, _>("avg_loss_holding_period") as f64,
                expectancy: row.get::<f32, _>("expectancy") as f64,
            };
            
            results.push((id, symbol, interval, performance));
        }
        
        Ok(results)
    }
}

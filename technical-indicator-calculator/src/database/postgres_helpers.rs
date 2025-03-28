use crate::database::postgres::PostgresManager;
use anyhow::Result;
use sqlx::{postgres::PgRow, Row, Postgres, query};
use chrono::{DateTime, Utc};

/// Query helper methods for the PostgresManager
impl PostgresManager {
    /// Execute a simple query without parameters and return all rows
    pub async fn execute_query(&self, query_str: &str) -> Result<Vec<PgRow>> {
        let rows = query(query_str)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query with a single string parameter
    pub async fn query_by_string(&self, query_str: &str, param: &str) -> Result<Vec<PgRow>> {
        let rows = query(query_str)
            .bind(param)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query with two string parameters
    pub async fn query_by_two_strings(&self, query_str: &str, param1: &str, param2: &str) -> Result<Vec<PgRow>> {
        let rows = query(query_str)
            .bind(param1)
            .bind(param2)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query with string and boolean parameters
    pub async fn query_by_string_and_bool(&self, query_str: &str, string_param: &str, bool_param: bool) -> Result<Vec<PgRow>> {
        let rows = query(query_str)
            .bind(string_param)
            .bind(bool_param)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query with string and i64 parameters
    pub async fn query_by_string_and_i64(&self, query_str: &str, string_param: &str, i64_param: i64) -> Result<Vec<PgRow>> {
        let rows = query(query_str)
            .bind(string_param)
            .bind(i64_param)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
    
    /// Execute a query that returns an optional single row
    pub async fn query_opt_by_string(&self, query_str: &str, param: &str) -> Result<Option<PgRow>> {
        let row = query(query_str)
            .bind(param)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }
    
    /// Execute a query that returns a single row
    pub async fn query_one_by_string(&self, query_str: &str, param: &str) -> Result<PgRow> {
        let row = query(query_str)
            .bind(param)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }
    
    /// Execute a command that doesn't return rows but affects rows
    pub async fn execute_command_by_string(&self, query_str: &str, param: &str) -> Result<u64> {
        let result = query(query_str)
            .bind(param)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
    
    /// Execute a command with string and boolean parameters
    pub async fn execute_command_by_string_and_bool(&self, query_str: &str, string_param: &str, bool_param: bool) -> Result<u64> {
        let result = query(query_str)
            .bind(string_param)
            .bind(bool_param)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
    
    /// Begin a transaction
    pub async fn begin_transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        let tx = self.pool.begin().await?;
        Ok(tx)
    }
    
    /// Execute transaction with a string parameter and returning rows affected
    pub async fn execute_tx_command_by_string<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Postgres>,
        query_str: &str,
        param: &str
    ) -> Result<u64> {
        let result = query(query_str)
            .bind(param)
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected())
    }
    
    /// Execute transaction with multiple string parameters
    pub async fn execute_tx_command_multi_string<'a>(
        &self, 
        tx: &mut sqlx::Transaction<'a, Postgres>,
        query_str: &str,
        params: &[&str]
    ) -> Result<u64> {
        let mut q = query(query_str);
        
        for param in params {
            q = q.bind(*param);
        }
        
        let result = q.execute(&mut **tx).await?;
        Ok(result.rows_affected())
    }
    
    /// Execute a transaction with a variety of parameter types (for strategy saving)
    pub async fn execute_tx_insert_strategy<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Postgres>,
        query_str: &str,
        id: &str,
        name: &str,
        description: &str,
        version: &str,
        author: &str,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        enabled: bool,
        assets_json: serde_json::Value,
        timeframes_json: serde_json::Value,
        parameters_json: serde_json::Value,
        risk_management_json: serde_json::Value,
        metadata_json: serde_json::Value
    ) -> Result<u64> {
        let result = query(query_str)
            .bind(id)
            .bind(name)
            .bind(description)
            .bind(version)
            .bind(author)
            .bind(created_at)
            .bind(updated_at)
            .bind(enabled)
            .bind(assets_json)
            .bind(timeframes_json)
            .bind(parameters_json)
            .bind(risk_management_json)
            .bind(metadata_json)
            .execute(&mut **tx)
            .await?;
            
        Ok(result.rows_affected())
    }
    
    /// Execute transaction to insert indicator
    pub async fn execute_tx_insert_indicator<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Postgres>,
        query_str: &str,
        strategy_id: &str,
        indicator_id: &str,
        indicator_type: &str,
        indicator_name: &str,
        parameters_json: serde_json::Value,
        description: &str,
        created_at: DateTime<Utc>
    ) -> Result<u64> {
        let result = query(query_str)
            .bind(strategy_id)
            .bind(indicator_id)
            .bind(indicator_type)
            .bind(indicator_name)
            .bind(parameters_json)
            .bind(description)
            .bind(created_at)
            .execute(&mut **tx)
            .await?;
            
        Ok(result.rows_affected())
    }
    
    /// Execute transaction to insert rule
    pub async fn execute_tx_insert_rule<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Postgres>,
        query_str: &str,
        strategy_id: &str,
        rule_id: &str,
        name: &str,
        condition_json: serde_json::Value,
        action_json: serde_json::Value,
        priority: i32,
        description: &str,
        created_at: DateTime<Utc>
    ) -> Result<u64> {
        let result = query(query_str)
            .bind(strategy_id)
            .bind(rule_id)
            .bind(name)
            .bind(condition_json)
            .bind(action_json)
            .bind(priority)
            .bind(description)
            .bind(created_at)
            .execute(&mut **tx)
            .await?;
            
        Ok(result.rows_affected())
    }
    
    /// Execute transaction for saving backtest results
    pub async fn execute_save_backtest_result(
        &self,
        query_str: &str,
        strategy_id: &str,
        symbol: &str,
        interval: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        initial_capital: f64,
        final_capital: f64,
        total_trades: i32,
        winning_trades: i32,
        losing_trades: i32,
        win_rate: f32,
        max_drawdown: f32,
        profit_factor: f32,
        sharpe_ratio: f32,
        total_return: f32,
        annualized_return: f32,
        max_consecutive_wins: i32,
        max_consecutive_losses: i32,
        avg_profit_per_win: f32,
        avg_loss_per_loss: f32,
        avg_win_holding_period: f32,
        avg_loss_holding_period: f32,
        expectancy: f32,
        parameters_snapshot: serde_json::Value,
        created_at: DateTime<Utc>
    ) -> Result<i32> {
        let row = query(query_str)
            .bind(strategy_id)
            .bind(symbol)
            .bind(interval)
            .bind(start_date)
            .bind(end_date)
            .bind(initial_capital as f32)
            .bind(final_capital as f32)
            .bind(total_trades)
            .bind(winning_trades)
            .bind(losing_trades)
            .bind(win_rate)
            .bind(max_drawdown)
            .bind(profit_factor)
            .bind(sharpe_ratio)
            .bind(total_return)
            .bind(annualized_return)
            .bind(max_consecutive_wins)
            .bind(max_consecutive_losses)
            .bind(avg_profit_per_win)
            .bind(avg_loss_per_loss)
            .bind(avg_win_holding_period)
            .bind(avg_loss_holding_period)
            .bind(expectancy)
            .bind(parameters_snapshot)
            .bind(created_at)
            .fetch_one(&self.pool)
            .await?;
            
        let id: i32 = row.get("id");
        Ok(id)
    }
    
    /// Commit a transaction
    pub async fn commit_transaction(
        &self, 
        tx: sqlx::Transaction<'_, Postgres>
    ) -> Result<()> {
        tx.commit().await?;
        Ok(())
    }
}

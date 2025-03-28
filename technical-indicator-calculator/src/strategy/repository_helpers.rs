// src/strategy/repository_helpers.rs
use crate::database::postgres::PostgresManager;
use crate::strategy::schema::{
    Strategy, StrategyIndicator, StrategyRule, StrategyParameter,
    RiskManagement, CompositeCondition, RuleAction
};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use sqlx::postgres::PgRow;
use sqlx::Row; // Add this import
use uuid::Uuid;

/// Parse a strategy from a database row
pub fn parse_strategy_json(row: PgRow) -> Result<Strategy> {
    // Parse the strategy data
    let uuid_str: String = row.get("id");
    let uuid = Uuid::parse_str(&uuid_str)?;
    
    let name: String = row.get("name");
    let description: String = row.get("description");
    let version: String = row.get("version");
    let author: String = row.get("author");
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");
    let enabled: bool = row.get("enabled");
    
    // Parse JSON fields
    let assets_json: serde_json::Value = row.get("assets");
    let timeframes_json: serde_json::Value = row.get("timeframes");
    let parameters_json: serde_json::Value = row.get("parameters");
    let risk_management_json: serde_json::Value = row.get("risk_management");
    let metadata_json: Option<serde_json::Value> = row.try_get("metadata").unwrap_or(None);
    
    // Convert JSON fields to proper types
    let assets: Vec<String> = serde_json::from_value(assets_json)?;
    let timeframes: Vec<String> = serde_json::from_value(timeframes_json)?;
    let parameters: HashMap<String, StrategyParameter> = serde_json::from_value(parameters_json)?;
    let risk_management: RiskManagement = serde_json::from_value(risk_management_json)?;
    let metadata: HashMap<String, serde_json::Value> = match metadata_json {
        Some(json) => serde_json::from_value(json)?,
        None => HashMap::new(),
    };
    
    // Create the strategy object (without indicators and rules, which will be loaded separately)
    let strategy = Strategy {
        id: uuid.to_string(),
        name,
        description,
        version,
        author,
        created_at,
        updated_at,
        enabled,
        assets,
        timeframes,
        indicators: Vec::new(), // Will be filled separately
        rules: Vec::new(),      // Will be filled separately
        parameters,
        risk_management,
        performance: None,      // Will be filled separately if needed
        metadata,
    };
    
    Ok(strategy)
}

/// Load indicators for a strategy
pub async fn load_strategy_indicators(pg: &PostgresManager, strategy_id: &str) -> Result<Vec<StrategyIndicator>> {
    let strategy_uuid = Uuid::parse_str(strategy_id)
        .context("Invalid UUID format for strategy ID")?;
    
    // Convert Uuid to String for database query
    let strategy_uuid_str = strategy_uuid.to_string();
        
    let rows = pg.query_by_string(
        "SELECT indicator_id, indicator_type, indicator_name, parameters, description
         FROM strategy_indicators
         WHERE strategy_id = $1
         ORDER BY indicator_id",
        &strategy_uuid_str
    ).await?;
    
    let mut indicators = Vec::with_capacity(rows.len());
    
    for row in rows {
        let indicator_id: String = row.get("indicator_id");
        let indicator_type: String = row.get("indicator_type");
        let indicator_name: String = row.get("indicator_name");
        let parameters_json: serde_json::Value = row.get("parameters");
        let description: String = row.get("description");
        
        indicators.push(StrategyIndicator {
            id: indicator_id,
            indicator_type,
            indicator_name,
            parameters: parameters_json,
            description,
        });
    }
    
    Ok(indicators)
}

/// Load rules for a strategy
pub async fn load_strategy_rules(pg: &PostgresManager, strategy_id: &str) -> Result<Vec<StrategyRule>> {
    let strategy_uuid = Uuid::parse_str(strategy_id)
        .context("Invalid UUID format for strategy ID")?;
    
    // Convert Uuid to String for database query
    let strategy_uuid_str = strategy_uuid.to_string();
        
    let rows = pg.query_by_string(
        "SELECT rule_id, name, condition, action, priority, description
         FROM strategy_rules
         WHERE strategy_id = $1
         ORDER BY priority",
        &strategy_uuid_str
    ).await?;
    
    let mut rules = Vec::with_capacity(rows.len());
    
    for row in rows {
        let rule_id: String = row.get("rule_id");
        let name: String = row.get("name");
        let condition_json: serde_json::Value = row.get("condition");
        let action_json: serde_json::Value = row.get("action");
        let priority: i32 = row.get("priority");
        let description: String = row.get("description");
        
        // Parse JSON fields
        let condition: CompositeCondition = serde_json::from_value(condition_json)?;
        let action: RuleAction = serde_json::from_value(action_json)?;
        
        rules.push(StrategyRule {
            id: rule_id,
            name,
            condition,
            action,
            priority,
            description,
        });
    }
    
    Ok(rules)
}

/// Save indicators for a strategy
pub async fn save_strategy_indicators<'a>(
    tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    pg: &PostgresManager,
    strategy_id: Uuid,
    indicators: &[StrategyIndicator]
) -> Result<()> {
    // Convert Uuid to String for database query
    let strategy_id_str = strategy_id.to_string();
    
    for indicator in indicators {
        // Serialize parameters
        let parameters_json = serde_json::to_value(&indicator.parameters)?;
        
        pg.execute_tx_insert_indicator(
            tx,
            "INSERT INTO strategy_indicators
             (strategy_id, indicator_id, indicator_type, indicator_name, parameters, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &strategy_id_str,
            &indicator.id,
            &indicator.indicator_type,
            &indicator.indicator_name,
            parameters_json,
            &indicator.description,
            Utc::now()
        ).await?;
    }
    
    Ok(())
}

/// Save rules for a strategy
pub async fn save_strategy_rules<'a>(
    tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    pg: &PostgresManager,
    strategy_id: Uuid,
    rules: &[StrategyRule]
) -> Result<()> {
    // Convert Uuid to String for database query
    let strategy_id_str = strategy_id.to_string();
    
    for rule in rules {
        // Serialize condition and action
        let condition_json = serde_json::to_value(&rule.condition)?;
        let action_json = serde_json::to_value(&rule.action)?;
        
        pg.execute_tx_insert_rule(
            tx,
            "INSERT INTO strategy_rules
             (strategy_id, rule_id, name, condition, action, priority, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &strategy_id_str,
            &rule.id,
            &rule.name,
            condition_json,
            action_json,
            rule.priority,
            &rule.description,
            Utc::now()
        ).await?;
    }
    
    Ok(())
}

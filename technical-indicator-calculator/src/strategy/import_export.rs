// src/strategy/import_export.rs
use crate::strategy::schema::Strategy;
use crate::strategy::repository::StrategyRepository;
use anyhow::{Result, Context};
use chrono::Utc;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use tracing::info;
use uuid::Uuid;

/// Import a strategy from a JSON file
pub async fn import_strategy_from_file(
    repository: &StrategyRepository, 
    file_path: &Path
) -> Result<Strategy> {
    info!("Importing strategy from file: {}", file_path.display());
    
    // Open and read the file
    let file = File::open(file_path)
        .context(format!("Failed to open file: {}", file_path.display()))?;
    
    let mut reader = BufReader::new(file);
    let mut json_str = String::new();
    reader.read_to_string(&mut json_str)
        .context(format!("Failed to read file: {}", file_path.display()))?;
    
    // Parse the JSON into a Strategy object
    let mut strategy: Strategy = serde_json::from_str(&json_str)
        .context("Failed to parse strategy JSON")?;
    
    // Validate the strategy
    validate_strategy(&mut strategy)?;
    
    // Save the strategy to the database
    repository.save_strategy(&strategy).await?;
    
    info!("Strategy imported successfully: {} ({})", strategy.name, strategy.id);
    Ok(strategy)
}

/// Export a strategy to a JSON file
pub async fn export_strategy_to_file(
    repository: &StrategyRepository, 
    strategy_id: &str,
    file_path: &Path
) -> Result<()> {
    info!("Exporting strategy {} to file: {}", strategy_id, file_path.display());
    
    // Load the strategy
    let strategy = repository.get_strategy(strategy_id).await?;
    
    // Convert to JSON
    let json = serde_json::to_string_pretty(&strategy)
        .context("Failed to serialize strategy to JSON")?;
    
    // Write to file
    let file = File::create(file_path)
        .context(format!("Failed to create file: {}", file_path.display()))?;
    
    let mut writer = BufWriter::new(file);
    writer.write_all(json.as_bytes())
        .context(format!("Failed to write to file: {}", file_path.display()))?;
    
    info!("Strategy exported successfully to: {}", file_path.display());
    Ok(())
}

/// Validate a strategy and ensure it has a valid UUID
fn validate_strategy(strategy: &mut Strategy) -> Result<()> {
    // Check if the strategy has a valid UUID
    if strategy.id.is_empty() || strategy.id == "00000000-0000-0000-0000-000000000000" {
        // Generate a new UUID
        strategy.id = Uuid::new_v4().to_string();
        info!("Generated new UUID for strategy: {}", strategy.id);
    } else {
        // Validate the existing UUID
        let _: Uuid = Uuid::parse_str(&strategy.id)
            .context("Invalid UUID format in strategy")?;
    }
    
    // Ensure created_at and updated_at are set
    if strategy.created_at.timestamp() == 0 {
        strategy.created_at = Utc::now();
    }
    strategy.updated_at = Utc::now();
    
    // Validate indicators
    for indicator in &strategy.indicators {
        if indicator.id.is_empty() {
            return Err(anyhow::anyhow!("Indicator ID cannot be empty"));
        }
    }
    
    // Validate rules
    for rule in &strategy.rules {
        if rule.id.is_empty() {
            return Err(anyhow::anyhow!("Rule ID cannot be empty"));
        }
    }
    
    Ok(())
}

/// Create a new empty strategy
pub fn create_new_strategy(name: &str, description: &str) -> Strategy {
    Strategy {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: description.to_string(),
        version: "1.0.0".to_string(),
        author: "Strategy Creator".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        enabled: true,
        assets: vec!["BTCUSDT".to_string()],
        timeframes: vec!["1h".to_string()],
        indicators: Vec::new(),
        rules: Vec::new(),
        parameters: std::collections::HashMap::new(),
        risk_management: crate::strategy::schema::RiskManagement::default(),
        performance: None,
        metadata: std::collections::HashMap::new(),
    }
}

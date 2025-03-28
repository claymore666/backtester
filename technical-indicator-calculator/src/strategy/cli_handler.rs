// src/cli_handler.rs
use crate::cli::Commands;
use crate::database::postgres::PostgresManager;
use crate::strategy::evaluator::StrategyEvaluator;
use crate::strategy::repository::StrategyRepository;
use crate::strategy::import_export::{import_strategy_from_file, export_strategy_to_file};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::process::Command;
use tracing::{info, warn, error};

/// Create a database connection and repository
pub async fn create_repository() -> Result<StrategyRepository> {
    // Get database configuration from environment
    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse::<u16>()?;
    let db_user = env::var("DB_USER").unwrap_or_else(|_| "binanceuser".to_string());
    let db_password = env::var("DB_PASSWORD").unwrap_or_else(|_| "binancepass".to_string());
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| "binancedb".to_string());
    
    // Create PostgreSQL connection
    let pg = PostgresManager::new(
        &db_host,
        db_port,
        &db_user,
        &db_password,
        &db_name,
        10, // Max connections
    )
    .await?;
    
    // Create the repository
    let repository = StrategyRepository::new(Arc::new(pg));
    
    Ok(repository)
}

/// Parse an ISO date string to DateTime<Utc>
pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date_str)
        .map(|dt| dt.with_timezone(&Utc))
        .context("Failed to parse date string. Use ISO 8601 format (e.g., 2025-03-25T12:00:00Z)")
}

/// Format a strategy for display
fn format_strategy_for_display(strategy_id: &str, name: &str, version: &str, enabled: bool) -> String {
    let status = if enabled { "Enabled" } else { "Disabled" };
    let id_short = if strategy_id.len() > 8 {
        &strategy_id[0..8]
    } else {
        strategy_id
    };
    
    format!("{:8} | {:30} | {:10} | {}", id_short, name, version, status)
}

/// Execute a command from the CLI
pub async fn execute_command(command: Commands) -> Result<()> {
    match command {
        Commands::Start { .. } | Commands::Stop | Commands::Status => {
            // These commands are handled in main.rs
            unreachable!("Start/Stop/Status commands should be handled in main.rs");
        },
        
        Commands::List { enabled_only } => {
            // Create repository
            let repository = create_repository().await?;
            
            let strategies = repository.list_strategies(enabled_only).await?;
            
            println!("Found {} strategies:", strategies.len());
            println!("{:<8} | {:<30} | {:<10} | {:<10}", "ID", "Name", "Version", "Status");
            println!("{:-<8}-+-{:-<30}-+-{:-<10}-+-{:-<10}", "", "", "", "");
            
            for strategy in strategies {
                println!("{}", format_strategy_for_display(
                    &strategy.id, 
                    &strategy.name,
                    &strategy.version,
                    strategy.enabled
                ));
            }
        },
        
        Commands::View { id, export } => {
            // Create repository
            let repository = create_repository().await?;
            
            // Try to get the strategy
            let strategy = repository.get_strategy(&id).await?;
            
            // If export is specified, export the strategy to file
            if let Some(export_path) = export {
                export_strategy_to_file(&repository, &id, &export_path).await?;
                println!("Strategy exported to: {}", export_path.display());
                return Ok(());
            }
            
            // Otherwise, display the strategy details
            println!("\n=== STRATEGY DETAILS ===");
            println!("ID: {}", strategy.id);
            println!("Name: {}", strategy.name);
            println!("Description: {}", strategy.description);
            println!("Version: {}", strategy.version);
            println!("Author: {}", strategy.author);
            println!("Created: {}", strategy.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("Updated: {}", strategy.updated_at.format("%Y-%m-%d %H:%M:%S"));
            println!("Status: {}", if strategy.enabled { "Enabled" } else { "Disabled" });
            println!("Assets: {}", strategy.assets.join(", "));
            println!("Timeframes: {}", strategy.timeframes.join(", "));
            
            println!("\n=== INDICATORS ({}) ===", strategy.indicators.len());
            for indicator in &strategy.indicators {
                println!("- {} ({})", indicator.id, indicator.indicator_name);
                println!("  Type: {}", indicator.indicator_type);
                println!("  Parameters: {}", serde_json::to_string_pretty(&indicator.parameters)?);
                println!("  Description: {}", indicator.description);
                println!();
            }
            
            println!("\n=== RULES ({}) ===", strategy.rules.len());
            for rule in &strategy.rules {
                println!("- {} (Priority: {})", rule.name, rule.priority);
                println!("  ID: {}", rule.id);
                println!("  Action: {}", serde_json::to_string(&rule.action)?);
                println!("  Description: {}", rule.description);
                println!();
            }
            
            println!("\n=== PARAMETERS ({}) ===", strategy.parameters.len());
            for (name, param) in &strategy.parameters {
                println!("- {}: {}", name, serde_json::to_string(param)?);
            }
            
            println!("\n=== RISK MANAGEMENT ===");
            println!("Max Risk Per Trade: {}%", strategy.risk_management.max_risk_per_trade);
            println!("Max Total Risk: {}%", strategy.risk_management.max_total_risk);
            println!("Default Position Size: {}%", strategy.risk_management.default_position_size);
            println!("Default Stop Loss: {:?}%", strategy.risk_management.default_stop_loss);
            println!("Default Take Profit: {:?}%", strategy.risk_management.default_take_profit);
            println!("Use Trailing Stop: {}", strategy.risk_management.use_trailing_stop);
            if strategy.risk_management.use_trailing_stop {
                println!("  Trailing Stop Activation: {:?}%", strategy.risk_management.trailing_stop_activation);
                println!("  Trailing Stop Percent: {:?}%", strategy.risk_management.trailing_stop_percent);
            }
            
            if !strategy.metadata.is_empty() {
                println!("\n=== METADATA ===");
                for (key, value) in &strategy.metadata {
                    println!("{}: {}", key, value);
                }
            }
            
            // Get recent backtest results if available
            let backtest_results = repository.get_recent_backtest_results(&id, 5).await?;
            
            if !backtest_results.is_empty() {
                println!("\n=== RECENT BACKTEST RESULTS ===");
                println!("{:<5} | {:<10} | {:<10} | {:<8} | {:<8} | {:<12}", 
                         "ID", "Symbol", "Interval", "Win Rate", "Return", "Profit Factor");
                println!("{:-<5}-+-{:-<10}-+-{:-<10}-+-{:-<8}-+-{:-<8}-+-{:-<12}", 
                         "", "", "", "", "", "");
                
                for (result_id, symbol, interval, performance) in backtest_results {
                    println!("{:<5} | {:<10} | {:<10} | {:<8.2}% | {:<8.2}% | {:<12.2}", 
                             result_id, symbol, interval, 
                             performance.win_rate,
                             performance.total_return,
                             performance.profit_factor);
                }
            }
        },
        
        Commands::Import { file } => {
            // Create repository
            let repository = create_repository().await?;
            
            // Import the strategy
            let strategy = import_strategy_from_file(&repository, &file).await?;
            
            println!("Strategy imported successfully:");
            println!("ID: {}", strategy.id);
            println!("Name: {}", strategy.name);
            println!("Version: {}", strategy.version);
            println!("Assets: {}", strategy.assets.join(", "));
            println!("Timeframes: {}", strategy.timeframes.join(", "));
            println!("Indicators: {}", strategy.indicators.len());
            println!("Rules: {}", strategy.rules.len());
        },
        
        Commands::Backtest { 
            strategy_id, 
            symbol, 
            interval, 
            start_date, 
            end_date, 
            initial_capital, 
            export 
        } => {
            // Create repository
            let repository = create_repository().await?;
            
            // Get the strategy
            let strategy = repository.get_strategy(&strategy_id).await?;
            
            // Parse dates if provided
            let start_date = start_date.map(|d| parse_date(&d)).transpose()?;
            let end_date = end_date.map(|d| parse_date(&d)).transpose()?;
            
            // Create evaluator
            let evaluator = StrategyEvaluator::new(repository.get_db_connection(), initial_capital);
            
            // Run backtest
            println!("Running backtest for strategy {} on {}:{}", strategy.name, symbol, interval);
            let performance = evaluator.backtest(&strategy, &symbol, &interval, start_date, end_date).await?;
            
            // Save results to database
            let backtest_id = repository.save_backtest_result(
                &strategy_id, 
                &symbol, 
                &interval, 
                start_date, 
                end_date, 
                initial_capital, 
                &performance
            ).await?;
            
            // Display results
            println!("\nBacktest Results (ID: {}):", backtest_id);
            println!("Total Trades: {}", performance.total_trades);
            println!("Win Rate: {:.2}%", performance.win_rate);
            println!("Total Return: {:.2}%", performance.total_return);
            println!("Max Drawdown: {:.2}%", performance.max_drawdown);
            println!("Sharpe Ratio: {:.2}", performance.sharpe_ratio);
            println!("Profit Factor: {:.2}", performance.profit_factor);
            println!("Expectancy: {:.2}", performance.expectancy);
            
            // More detailed statistics
            println!("\nDetailed Statistics:");
            println!("Winning Trades: {} (Avg profit: {:.2}%)", 
                     performance.winning_trades, performance.avg_profit_per_win);
            println!("Losing Trades: {} (Avg loss: {:.2}%)", 
                     performance.losing_trades, performance.avg_loss_per_loss);
            println!("Max Consecutive Wins: {}", performance.max_consecutive_wins);
            println!("Max Consecutive Losses: {}", performance.max_consecutive_losses);
            println!("Avg Win Holding Period: {:.2} hours", performance.avg_win_holding_period);
            println!("Avg Loss Holding Period: {:.2} hours", performance.avg_loss_holding_period);
            println!("Annualized Return: {:.2}%", performance.annualized_return);
            
            // Export if requested
            if let Some(export_path) = export {
                let json = serde_json::to_string_pretty(&performance)?;
                std::fs::write(&export_path, json)?;
                println!("\nResults exported to: {}", export_path.display());
            }
        },
        
        Commands::Optimize { 
            strategy_id, 
            symbol, 
            interval, 
            start_date, 
            end_date, 
            initial_capital, 
            max_iterations, 
            output 
        } => {
            // Since the optimizer is in Python, call it as a separate process
            let mut command = Command::new("python3");
            
            command.arg("llm_optimizer.py")
                   .arg("--strategy_id").arg(strategy_id)
                   .arg("--symbol").arg(symbol)
                   .arg("--interval").arg(interval)
                   .arg("--initial_capital").arg(initial_capital.to_string())
                   .arg("--max_iterations").arg(max_iterations.to_string())
                   .arg("--output").arg(output.to_string_lossy().to_string());
                   
            if let Some(start) = start_date {
                command.arg("--start_date").arg(start);
            }
            
            if let Some(end) = end_date {
                command.arg("--end_date").arg(end);
            }
            
            println!("Starting LLM optimization process...");
            
            let output = command.output().context("Failed to run LLM optimizer")?;
            
            // Print any stdout/stderr from the process
            if !output.stdout.is_empty() {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            
            if !output.stderr.is_empty() {
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            
            // Check exit status
            if output.status.success() {
                println!("Optimization completed successfully.");
            } else {
                eprintln!("Optimization failed with exit code: {:?}", output.status.code());
                return Err(anyhow::anyhow!("Optimization failed"));
            }
        },
    }
    
    Ok(())
}

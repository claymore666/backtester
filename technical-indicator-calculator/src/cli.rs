// src/cli.rs
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use crate::database::postgres::PostgresManager;
use crate::strategy::evaluator::StrategyEvaluator;
use crate::strategy::repository::StrategyRepository;
use crate::strategy::schema::Strategy;
use std::sync::Arc;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, Context};
use serde_json;

#[derive(Parser)]
#[command(name = "strategy-cli")]
#[command(about = "Trading strategy CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all available strategies
    List {
        /// Show only enabled strategies
        #[arg(short, long)]
        enabled_only: bool,
    },
    
    /// View details of a strategy
    View {
        /// Strategy ID
        #[arg(short, long)]
        id: String,
        
        /// Export to JSON file
        #[arg(short, long)]
        export: Option<PathBuf>,
    },
    
    /// Import a strategy from JSON file
    Import {
        /// Input file
        #[arg(short, long)]
        file: PathBuf,
    },
    
    /// Run a backtest for a strategy
    Backtest {
        /// Strategy ID
        #[arg(short, long)]
        strategy_id: String,
        
        /// Symbol (e.g., "BTCUSDT")
        #[arg(short, long)]
        symbol: String,
        
        /// Interval (e.g., "1h", "4h", "1d")
        #[arg(short, long)]
        interval: String,
        
        /// Start date for backtest (ISO format)
        #[arg(long)]
        start_date: Option<String>,
        
        /// End date for backtest (ISO format)
        #[arg(long)]
        end_date: Option<String>,
        
        /// Initial capital
        #[arg(long, default_value = "10000.0")]
        initial_capital: f64,
        
        /// Export results to JSON file
        #[arg(long)]
        export: Option<PathBuf>,
    },
    
    /// Optimize a strategy using the LLM
    Optimize {
        /// Strategy ID
        #[arg(short, long)]
        strategy_id: String,
        
        /// Symbol (e.g., "BTCUSDT")
        #[arg(short, long)]
        symbol: String,
        
        /// Interval (e.g., "1h", "4h", "1d")
        #[arg(short, long)]
        interval: String,
        
        /// Start date for backtest (ISO format)
        #[arg(long)]
        start_date: Option<String>,
        
        /// End date for backtest (ISO format)
        #[arg(long)]
        end_date: Option<String>,
        
        /// Initial capital
        #[arg(long, default_value = "10000.0")]
        initial_capital: f64,
        
        /// Maximum iterations
        #[arg(long, default_value = "10")]
        max_iterations: usize,
        
        /// Output file for report
        #[arg(long, default_value = "optimization_report.md")]
        output: PathBuf,
    },
}

/// Connect to the database and create a repository
pub async fn create_repository() -> Result<StrategyRepository> {
    // Get database configuration from environment
    let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse::<u16>()?;
    let db_user = std::env::var("DB_USER").unwrap_or_else(|_| "binanceuser".to_string());
    let db_password = std::env::var("DB_PASSWORD").unwrap_or_else(|_| "binancepass".to_string());
    let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "binancedb".to_string());
    
    // Create PostgreSQL connection
    let pg: Arc<PostgresManager> = Arc::new(
        PostgresManager::new(
            &db_host,
            db_port,
            &db_user,
            &db_password,
            &db_name,
            10, // Max connections
        )
        .await?
    );
    
    // Create the repository
    let repository = StrategyRepository::new(pg);
    
    Ok(repository)
}

/// Parse an ISO date string to DateTime<Utc>
pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(date_str)?.with_timezone(&Utc))
}

/// Execute a command from the CLI
pub async fn execute_command(command: Commands) -> Result<()> {
    // Create repository
    let repository = create_repository().await?;
    
    // Execute command
    match command {
        Commands::List { enabled_only } => {
            let strategies = repository.list_strategies(enabled_only).await?;
            
            println!("Found {} strategies:", strategies.len());
            println!("{:<36} | {:<30} | {:<15} | {:<10}", "ID", "Name", "Version", "Enabled");
            println!("{:-<36}-+-{:-<30}-+-{:-<15}-+-{:-<10}", "", "", "", "");
            
            for strategy in strategies {
                println!("{:<36} | {:<30} | {:<15} | {:<10}", 
                        strategy.id, 
                        strategy.name, 
                        strategy.version,
                        if strategy.enabled { "Yes" } else { "No" });
            }
        },
        
        Commands::View { id, export } => {
            let strategy = repository.get_strategy(&id).await?;
            
            if let Some(export_path) = export {
                let json = serde_json::to_string_pretty(&strategy)?;
                std::fs::write(export_path, json)?;
                println!("Strategy exported successfully.");
            } else {
                // Print strategy details
                println!("Strategy: {} ({})", strategy.name, strategy.id);
                println!("Description: {}", strategy.description);
                println!("Version: {}", strategy.version);
                println!("Author: {}", strategy.author);
                println!("Created: {}", strategy.created_at);
                println!("Updated: {}", strategy.updated_at);
                println!("Enabled: {}", strategy.enabled);
                println!("Assets: {:?}", strategy.assets);
                println!("Timeframes: {:?}", strategy.timeframes);
                
                println!("\nIndicators:");
                for indicator in &strategy.indicators {
                    println!("  {} ({}): {} - {:?}", 
                             indicator.id, 
                             indicator.indicator_type,
                             indicator.indicator_name,
                             indicator.parameters);
                }
                
                println!("\nRules:");
                for rule in &strategy.rules {
                    println!("  {} (Priority {}): {}", rule.id, rule.priority, rule.name);
                    println!("    Action: {:?}", rule.action);
                }
                
                println!("\nParameters:");
                for (param_id, param) in &strategy.parameters {
                    println!("  {}: {:?}", param_id, param);
                }
                
                println!("\nRisk Management:");
                println!("  Max Risk Per Trade: {}%", strategy.risk_management.max_risk_per_trade);
                println!("  Max Total Risk: {}%", strategy.risk_management.max_total_risk);
                println!("  Default Position Size: {}%", strategy.risk_management.default_position_size);
                println!("  Default Stop Loss: {:?}%", strategy.risk_management.default_stop_loss);
                println!("  Default Take Profit: {:?}%", strategy.risk_management.default_take_profit);
            }
        },
        
        Commands::Import { file } => {
            let json = std::fs::read_to_string(file)?;
            let strategy: Strategy = serde_json::from_str(&json)?;
            
            repository.save_strategy(&strategy).await?;
            println!("Strategy imported successfully with ID: {}", strategy.id);
        },
        
        Commands::Backtest { strategy_id, symbol, interval, start_date, end_date, initial_capital, export } => {
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
            repository.save_backtest_result(&strategy_id, &symbol, &interval, start_date, end_date, 
                                           initial_capital, &performance).await?;
            
            // Display results
            println!("\nBacktest Results:");
            println!("Total Trades: {}", performance.total_trades);
            println!("Win Rate: {:.2}%", performance.win_rate);
            println!("Total Return: {:.2}%", performance.total_return);
            println!("Max Drawdown: {:.2}%", performance.max_drawdown);
            println!("Sharpe Ratio: {:.2}", performance.sharpe_ratio);
            println!("Profit Factor: {:.2}", performance.profit_factor);
            println!("Expectancy: {:.2}", performance.expectancy);
            
            // Export if requested
            if let Some(export_path) = export {
                let json = serde_json::to_string_pretty(&performance)?;
                std::fs::write(export_path, json)?;
                println!("\nResults exported successfully.");
            }
        },
        
        Commands::Optimize { strategy_id, symbol, interval, start_date, end_date, initial_capital, max_iterations, output } => {
            // Since the optimizer is in Python, call it as a separate process
            let mut command = Command::new("python3");
            
            command.arg("llm_optimizer.py")
                   .arg("--strategy_id").arg(strategy_id)
                   .arg("--symbol").arg(symbol)
                   .arg("--interval").arg(interval)
                   .arg("--initial_capital").arg(initial_capital.to_string())
                   .arg("--max_iterations").arg(max_iterations.to_string())
                   .arg("--output").arg(output);
                   
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

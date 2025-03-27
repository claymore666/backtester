// src/main.rs
use technical_indicator_calculator::cli::{Cli, Commands, execute_command};
use clap::Parser;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment
    dotenv::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Execute command
    execute_command(cli.command).await?;
    
    Ok(())
}

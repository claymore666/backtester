// src/main.rs
use technical_indicator_calculator::cli::{Cli, Commands};
use technical_indicator_calculator::strategy::cli_handler::execute_command;
use technical_indicator_calculator::daemon::{start_daemon, stop_daemon, check_daemon_status};
use technical_indicator_calculator::worker::start_worker;
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
    match cli.command {
        Commands::Start { concurrency, detached } => {
            if detached {
                start_daemon(concurrency).await?;
            } else {
                start_worker(concurrency).await?;
            }
        },
        Commands::Stop => {
            stop_daemon().await?;
        },
        Commands::Status => {
            check_daemon_status().await?;
        },
        _ => {
            execute_command(cli.command).await?;
        }
    }
    
    Ok(())
}

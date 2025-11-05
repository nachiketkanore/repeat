use anyhow::{Context, Result};
use clap::Parser;
use std::pin::Pin;
use std::time::Duration;
use tokio::select;
use tokio::time::timeout;

mod analyzer;
mod config;
mod execution;
mod runner;

use crate::execution::{Execution, TimedExecution};
use analyzer::AnalysisTracker;
use config::CliConfig;
use runner::Runner;

#[tokio::main]
async fn main() -> Result<()> {
    let config = CliConfig::parse();
    if config.verbose {
        println!("arguments: {:#?}", config);
    }

    let global_timeout = Duration::from_secs(config.total_run_timeout_sec);

    let execution_fn = move || -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move { Ok(run_execution_loop(&config, Runner::new(&config)).await) })
    };

    let timed_executor = TimedExecution {
        timeout: global_timeout,
        executor: execution_fn, // Pass the closure
    };

    let _ = timed_executor.execute().await;

    Ok(())
}

async fn run_execution_loop(config: &CliConfig, runner: Runner<'_>) {
    // 2. Global Signal Handling for Graceful Exit (Essential for stability)
    let ctrl_c_signal = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c_signal);

    for _iteration in 0..config.iterations {
        // Use tokio::select! to watch for both command execution and Ctrl+C signal concurrently.
        select! {
            // Check for Ctrl+C signal
            _ = &mut ctrl_c_signal => {
                eprintln!("\nCtrl+C signal received. Preparing for graceful shutdown...");
                break;
            }

            // Execute the command
            run_result = runner.execute_command() => {
                let record = match run_result {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("\n[ERROR] Command execution failed: {:#?}", e);
                        // If execution fails immediately (e.g., bad command path), break the loop
                        break;
                    }
                };

                // 4. Exit Control Check
                if let Some(exit_code) = config.exit_code {
                    if record.exit_code == Some(exit_code) {
                        eprintln!("\nTarget exit code {} matched. Exiting loop.", exit_code);
                        break;
                    }
                }
            }
        }
    }
}

use std::time::Duration;
use anyhow::{Context, Result};
use clap::Parser;
use tokio::select;
use tokio::time::timeout;

mod analyzer;
mod config;
mod runner;

use analyzer::AnalysisTracker;
use config::CliConfig;
use runner::Runner;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup and Argument Parsing
    let config = CliConfig::parse();
    let runner = Runner::new(&config);
    let mut tracker = AnalysisTracker::new();

    // run this only until config.total_run_timeout_sec is passed
    let global_timeout = Duration::from_secs(config.total_run_timeout_sec);
    let result = timeout(global_timeout, run_execution_loop(&config, runner, &mut tracker)).await;
    
    match result {
        Ok(()) => {
            println!("Execution loop finished normally before the timeout.");
        }
        Err(_elapsed) => {
            println!(
                "Global timeout of {} seconds reached – stopping the loop.",
                config.total_run_timeout_sec
            );
            // TODO: perform any clean-up here (e.g. signal the runner to stop)
        }
    }

    if config.verbose {
        // print input information
        println!("arguments: {:#?}", config);
    }

    // TODO: design patterns -> execution hooks trait
    // for any custom condition that needs to be fulfilled


    // 3. Main Infinite Loop

    // 5. Post-Run Analysis
    tracker.report();

    Ok(())
}

async fn run_execution_loop(config: &CliConfig, runner: Runner<'_>, tracker: &mut AnalysisTracker) {

    // 2. Global Signal Handling for Graceful Exit (Essential for stability)
    let ctrl_c_signal = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c_signal);

    for iteration in  0..config.iterations {
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

                // Record the run and print verbose output if enabled
                tracker.record(&record);

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
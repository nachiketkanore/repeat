use anyhow::{Context, Result};
use clap::Parser;
use tokio::select;

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

    // 2. Global Signal Handling for Graceful Exit (Essential for stability)
    let ctrl_c_signal = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c_signal);

    eprintln!(
        "RLR: Starting loop for command: {}",
        config.command.join(" ")
    );
    if let Some(code) = config.exit_code {
        eprintln!("RLR: Loop will exit on command exit code: {}", code);
    }
    if let Some(secs) = config.run_timeout_sec {
        eprintln!("RLR: Per-run timeout set to {} seconds.", secs);
    }

    // 3. Main Infinite Loop
    for iteration in  0..config.iterations {
        // Use tokio::select! to watch for both command execution and Ctrl+C signal concurrently.
        select! {
            // Check for Ctrl+C signal
            _ = &mut ctrl_c_signal => {
                eprintln!("\nRLR: Ctrl+C signal received. Preparing for graceful shutdown...");
                break;
            }

            // Execute the command
            run_result = runner.execute_command() => {
                let record = match run_result {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("\n[RLR ERROR] Command execution failed: {:#?}", e);
                        // If execution fails immediately (e.g., bad command path), break the loop
                        break;
                    }
                };

                // Record the run and print verbose output if enabled
                tracker.record(&record);

                // 4. Exit Control Check
                if let Some(exit_code) = config.exit_code {
                    if record.exit_code == Some(exit_code) {
                        eprintln!("\nRLR: Target exit code {} matched. Exiting loop.", exit_code);
                        break;
                    }
                }
            }
        }
    }

    // 5. Post-Run Analysis
    tracker.report();

    Ok(())
}

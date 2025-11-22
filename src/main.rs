use anyhow::Result;
use clap::Parser;

use std::time::Duration;
use tokio::select;
use tokio::time::{Instant, sleep};

mod analyzer;
mod config;
mod execution;
mod runner;
mod utils;

use crate::analyzer::AnalysisTracker;
use crate::execution::{Execution, TimedFunctionExecution};
use config::CliConfig;
use runner::Runner;

#[tokio::main]
async fn main() -> Result<()> {
    let config = CliConfig::parse();
    let mut tracker = AnalysisTracker::new(config.verbose);
    if config.verbose {
        // TODO: decide on this
        // println!("arguments: {:#?}", config);
    }

    let global_timeout = Duration::from_secs(config.total_run_timeout_sec);
    let runner = Runner::new(&config);

    let start_instant = Instant::now();

    let _result = TimedFunctionExecution {
        timeout: global_timeout,
        executor: run_execution_loop(&config, runner, &mut tracker),
    }
    .execute()
    .await;

    tracker.report(start_instant);

    Ok(())
}

async fn run_execution_loop(config: &CliConfig, runner: Runner<'_>, tracker: &mut AnalysisTracker) {
    // 2. Global Signal Handling for Graceful Exit (Essential for stability)
    let ctrl_c_signal = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c_signal);

    sleep(Duration::from_secs(config.initial_delay.get_value())).await;

    for itr in 1..=config.iterations {
        select! {
            _ = &mut ctrl_c_signal => {
                eprintln!("\nCtrl+C signal received. Preparing for graceful shutdown...");
                break;
            }

            run_result = runner.execute_command(tracker) => {
                let record = match run_result {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("\n[ERROR] Command execution failed: {:#?}", e);
                        break;
                    }
                };

                if let Some(ref match_output) = config.match_output {
                    if record.stdout == *match_output {
                        eprintln!("\nCommand output matching required output");
                        break;
                    }
                }

                if let Some(exit_code) = config.exit_code {
                    if record.exit_code == Some(exit_code) {
                        eprintln!("\nTarget exit code {} matched. Exiting loop.", exit_code);
                        break;
                    }
                }
            }
        }

        if itr != config.iterations {
            sleep(Duration::from_secs(config.in_between_delay.get_value())).await;
        }
    }
}

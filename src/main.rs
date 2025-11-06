use anyhow::{Context, Result};
use clap::Parser;
use std::pin::Pin;
use std::time::Duration;
use tokio::select;
use tokio::time::{Instant, sleep};

mod analyzer;
mod config;
mod execution;
mod runner;
mod utils;

use crate::analyzer::AnalysisTracker;
use crate::execution::{Execution, TimedCommandExecution, TimedFunctionExecution};
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

    sleep(Duration::from_secs(config.initial_delay)).await;

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

                if let Some(exit_code) = config.exit_code {
                    if record.exit_code == Some(exit_code) {
                        eprintln!("\nTarget exit code {} matched. Exiting loop.", exit_code);
                        break;
                    }
                }
            }
        }

        if itr != config.iterations {
            sleep(Duration::from_secs(config.in_between_delay)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    #[derive(Debug, PartialEq, Eq, Clone)]
    struct CommandRecord {
        pub exit_code: Option<i32>,
        pub stdout: String,
        pub stderr: String,
    }
    // --- Test Mocks for Runner ---

    // Define a mock Runner that uses an mpsc channel to receive pre-determined results
    struct MockRunner {
        // Sender for a channel that feeds results back to the caller of execute_command
        result_rx: Mutex<mpsc::Receiver<Result<CommandRecord>>>,
        // Atomic counter to track how many times execute_command was called
        call_count: Arc<AtomicUsize>,
    }

    impl MockRunner {
        fn new(rx: mpsc::Receiver<Result<CommandRecord>>) -> Self {
            MockRunner {
                result_rx: Mutex::new(rx),
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        async fn execute_command(&self) -> Result<CommandRecord> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            // This is safe because only one task owns and consumes the MockRunner
            // in the test, ensuring no concurrent access to the receiver.
            self.result_rx
                .lock()
                .unwrap()
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("Mock result channel closed unexpectedly"))
                .flatten()
        }
    }

    // Helper to create the test config with minimal fields
    fn create_config(iterations: u64, exit_code: Option<i32>) -> CliConfig {
        CliConfig {
            iterations,
            exit_code,
            ..Default::default()
        }
    }

    // Helper to run run_execution_loop with the mock runner
    async fn run_mocked_loop(config: &CliConfig, mock_runner: MockRunner) -> usize {
        let call_count = mock_runner.call_count.clone();

        // This is the function under test. We manually call its logic but with our mock.
        // We use an internal function to bypass the Runner struct definition and call the mock method directly.
        async fn internal_run_loop(config: &CliConfig, runner: MockRunner) {
            let (mock_ctrl_c_tx, mut mock_ctrl_c_rx) = tokio::sync::mpsc::channel::<()>(1);

            for _iteration in 0..config.iterations {
                select! {
                    _ = mock_ctrl_c_rx.recv() => { // Use channel to simulate Ctrl+C
                        eprintln!("\nCtrl+C signal received. Preparing for graceful shutdown (Mocked).");
                        break;
                    }

                    run_result = runner.execute_command() => {
                        let record = match run_result {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("\n[ERROR] Command execution failed: {:#?}", e);
                                break;
                            }
                        };

                        if let Some(exit_code) = config.exit_code {
                            if record.exit_code == Some(exit_code) {
                                eprintln!("\nTarget exit code {} matched. Exiting loop (Mocked).", exit_code);
                                break;
                            }
                        }
                    }
                }
            }
        }

        internal_run_loop(config, mock_runner).await;

        call_count.load(Ordering::SeqCst)
    }

    // --- Tests ---

    #[tokio::test]
    async fn test_run_execution_loop_completes_all_iterations() {
        let iterations = 5;
        let config = create_config(iterations, None);
        let (tx, rx) = mpsc::channel(iterations as usize);

        for _ in 0..iterations {
            tx.send(Ok(CommandRecord {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }))
            .await
            .unwrap();
        }

        // Drop the sender so the channel closes after all results are consumed.
        drop(tx);

        let mock_runner = MockRunner::new(rx);
        let calls = run_mocked_loop(&config, mock_runner).await;

        // The loop should run exactly 5 times and exit gracefully.
        assert_eq!(
            calls, iterations as usize,
            "The loop should execute the configured number of iterations."
        );
    }

    #[tokio::test]
    async fn test_run_execution_loop_exits_on_target_exit_code() {
        let iterations = 10;
        let target_exit_code = 42;
        let config = create_config(iterations, Some(target_exit_code));
        let (tx, rx) = mpsc::channel(iterations as usize);

        // 1. Send 2 successful runs (non-matching code)
        tx.send(Ok(CommandRecord {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();
        tx.send(Ok(CommandRecord {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();

        // 2. Send the matching exit code on the 3rd iteration
        tx.send(Ok(CommandRecord {
            exit_code: Some(target_exit_code),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();

        // 3. Send a few more non-matching results that should be ignored
        tx.send(Ok(CommandRecord {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();

        drop(tx);

        let mock_runner = MockRunner::new(rx);
        let calls = run_mocked_loop(&config, mock_runner).await;

        // The loop should break immediately after the 3rd execution
        assert_eq!(
            calls, 3,
            "The loop should break after the target exit code is matched."
        );
    }

    #[tokio::test]
    async fn test_run_execution_loop_exits_on_command_error() {
        let iterations = 10;
        let config = create_config(iterations, None);
        let (tx, rx) = mpsc::channel(iterations as usize);

        // 1. Send a successful run
        tx.send(Ok(CommandRecord {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();

        // 2. Send an execution error (e.g., process failed to spawn)
        tx.send(Err(anyhow::anyhow!("Spawn error"))).await.unwrap();

        // 3. Send more runs that should be ignored
        tx.send(Ok(CommandRecord {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        }))
        .await
        .unwrap();

        drop(tx);

        let mock_runner = MockRunner::new(rx);
        let calls = run_mocked_loop(&config, mock_runner).await;

        // The loop should break immediately after the 2nd execution due to the error.
        assert_eq!(
            calls, 2,
            "The loop should break immediately upon receiving a command execution error."
        );
    }
}

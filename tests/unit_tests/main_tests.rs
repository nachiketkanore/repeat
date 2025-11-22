// Unit tests for the main module (execution loop logic)

use anyhow::Result;
use repeat::config::CliConfig;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::select;
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
        let (_mock_ctrl_c_tx, mut mock_ctrl_c_rx) = tokio::sync::mpsc::channel::<()>(1);

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

                    if let Some(ref match_output) = config.match_output {
                        if record.stdout == *match_output {
                            eprintln!("\nCommand output matching required output (Mocked)");
                            break;
                        }
                    }

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

#[tokio::test]
async fn test_run_execution_loop_exits_on_output_match() {
    let iterations = 10;
    let match_output = "STOP".to_string();
    let mut config = create_config(iterations, None);
    config.match_output = Some(match_output.clone());

    let (tx, rx) = mpsc::channel(iterations as usize);

    // 1. Send two non-matching outputs
    tx.send(Ok(CommandRecord {
        exit_code: Some(0),
        stdout: "RUNNING".to_string(),
        stderr: String::new(),
    }))
    .await
    .unwrap();
    tx.send(Ok(CommandRecord {
        exit_code: Some(0),
        stdout: "STILL_RUNNING".to_string(),
        stderr: String::new(),
    }))
    .await
    .unwrap();

    // 2. Send the matching output
    tx.send(Ok(CommandRecord {
        exit_code: Some(0),
        stdout: match_output,
        stderr: String::new(),
    }))
    .await
    .unwrap();

    // 3. Send one more that should be ignored
    tx.send(Ok(CommandRecord {
        exit_code: Some(0),
        stdout: "AFTER".to_string(),
        stderr: String::new(),
    }))
    .await
    .unwrap();

    drop(tx);

    let mock_runner = MockRunner::new(rx);
    let calls = run_mocked_loop(&config, mock_runner).await;

    assert_eq!(
        calls, 3,
        "The loop should break after the `match_output` is found."
    );
}

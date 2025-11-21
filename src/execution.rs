use crate::analyzer::{RunRecord, RunStatus};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

pub trait Execution<R> {
    async fn execute(self) -> R;
}

#[derive(Debug, PartialEq)]
pub enum ExecutionResult {
    Success,
    Timeout,
    Failure,
}

pub struct TimedFunctionExecution<F>
where
    F: IntoFuture,
{
    pub(crate) timeout: Duration,
    pub(crate) executor: F,
}

impl<F> Execution<Result<(), Elapsed>> for TimedFunctionExecution<F>
where
    F: IntoFuture,
{
    async fn execute(self) -> Result<(), Elapsed> {
        match timeout(self.timeout, self.executor).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub struct TimedCommandExecution {
    pub(crate) timeout: Duration,
    pub(crate) command: Command,
}

impl Execution<RunRecord> for TimedCommandExecution {
    async fn execute(mut self) -> RunRecord {
        let start_time = tokio::time::Instant::now();

        // 1. Spawn the command
        let child = match self
            .command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                // If spawning fails (e.g., command not found)
                eprintln!("FAILURE: Failed to spawn command. Error: {}", e);
                return RunRecord {
                    status: RunStatus::Completed,
                    exit_code: None,
                    duration: tokio::time::Instant::now().duration_since(start_time),
                    stdout: String::new(),
                    stderr: format!("Failed to spawn command: {}", e),
                };
            }
        };

        // 2. Set up the waiting future
        // Wait for the child process to complete.
        let wait_future = child.wait_with_output();

        let result = match timeout(self.timeout, wait_future).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

                RunRecord {
                    status: RunStatus::Completed,
                    exit_code: output.status.code(),
                    duration: tokio::time::Instant::now().duration_since(start_time),
                    stdout,
                    stderr,
                }
            }
            Ok(Err(e)) => {
                // An IO error occurred while waiting for the output
                RunRecord {
                    status: RunStatus::Completed,
                    exit_code: None,
                    duration: tokio::time::Instant::now().duration_since(start_time),
                    stdout: String::new(),
                    stderr: format!("IO error: {}", e),
                }
            }
            Err(_) => {
                // The outer Result is Err(Elapsed), meaning the process *timed out*.

                // TODO: Kill the child process upon timeout
                // We attempt to kill the process and ignore potential errors (like it being gone already).
                // let _ = child.kill().await;

                RunRecord {
                    status: RunStatus::Timeout,
                    exit_code: None, // Cannot reliably get exit code after a forced kill
                    duration: tokio::time::Instant::now().duration_since(start_time),
                    stdout: String::new(), // Output might be partial or unavailable
                    stderr: "Process terminated due to timeout.".to_string(),
                }
            }
        };

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::process::Command;

    #[tokio::test]
    async fn test_command_execution_output_match_success() {
        let mut command = Command::new("echo");
        command.arg("hello");

        let execution = TimedCommandExecution {
            timeout: Duration::from_secs(1),
            command,
        };

        let record = execution.execute().await;
        assert_eq!(record.status, RunStatus::Completed);
        assert_eq!(record.exit_code, Some(0));
        assert_eq!(record.stdout, "hello\n");
    }
}

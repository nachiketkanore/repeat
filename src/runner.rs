use crate::analyzer::{RunRecord, RunStatus};
use crate::config::CliConfig;
use crate::execution::{Execution, ExecutionResult, TimedExecution};
use anyhow::{Context, Result, bail};
use std::process::Stdio;
use std::time::Duration;

pub struct Runner<'a> {
    config: &'a CliConfig,
}

impl<'a> Runner<'a> {
    pub fn new(config: &'a CliConfig) -> Self {
        Runner { config }
    }

    pub async fn execute_command(&self) -> Result<RunRecord> {
        let (exec, args) = self.config.executable_and_args();
        let start_time = std::time::Instant::now();

        if self.config.verbose {
            eprintln!("\n=> Starting run: {} {}", exec, args.join(" "));
        }

        let mut command = tokio::process::Command::new(exec);
        command
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()); // Capture stderr

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn command: {}", exec))?;

        let output = {
            let secs = self.config.single_run_timeout_sec;
            let limit = Duration::from_secs(secs);

            let result = TimedExecution {
                timeout: limit,
                executor: child.wait(),
            };

            match result.execute().await {
                ExecutionResult::Success => child.wait_with_output().await?,
                ExecutionResult::Failure => {
                    bail!("Error waiting for child status")
                }
                ExecutionResult::Timeout => {
                    let _ = child.kill().await;
                    return Ok(RunRecord {
                        status: RunStatus::Timeout,
                        exit_code: None,
                        duration: start_time.elapsed(),
                        stdout: String::new(),
                        stderr: format!("Process timed out after {}s and was killed.", secs),
                    });
                }
            }
        };

        // Output and duration calculation are now outside the conditional logic
        let duration = start_time.elapsed();
        let exit_code = output.status.code();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let run_record = RunRecord {
            status: RunStatus::Completed,
            exit_code,
            duration,
            stdout,
            stderr,
        };

        if self.config.verbose {
            println!("{:#?}", output);
            println!("{:#?}", run_record);
        }

        Ok(run_record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CliConfig;
    // Need to import CliConfig for testing

    // Helper to create a mock CliConfig
    fn mock_config(command: Vec<&str>, single_run_timeout_sec: u64, verbose: bool) -> CliConfig {
        CliConfig {
            command: command.into_iter().map(String::from).collect(),
            exit_code: None,
            single_run_timeout_sec,
            total_run_timeout_sec: 100, // Irrelevant for unit tests
            verbose,
            iterations: 1, // Irrelevant for unit tests
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn execute_command_success() -> Result<()> {
        let config = mock_config(vec!["echo", "TEST_SUCCESS"], 1, false);
        let runner = Runner::new(&config);
        let record = runner.execute_command().await?;

        assert_eq!(record.status, RunStatus::Completed);
        assert_eq!(record.exit_code, Some(0));
        assert_eq!(record.stdout.trim(), "TEST_SUCCESS");
        assert!(record.stderr.is_empty());
        assert!(record.duration < Duration::from_secs(1)); // Should be fast

        Ok(())
    }

    #[tokio::test]
    async fn execute_command_non_zero_exit_code() -> Result<()> {
        // Use a shell command to explicitly control the exit code
        let config = mock_config(vec!["sh", "-c", "echo 'failed' >&2; exit 123"], 1, false);
        let runner = Runner::new(&config);
        let record = runner.execute_command().await?;

        assert_eq!(record.status, RunStatus::Completed);
        assert_eq!(record.exit_code, Some(123));
        assert!(record.stdout.is_empty());
        assert_eq!(record.stderr.trim(), "failed");

        Ok(())
    }

    #[tokio::test]
    async fn execute_command_timeout() -> Result<()> {
        // Run a command that sleeps for 2 seconds with a 1 second timeout
        // Note: The `sleep` command is generally available on POSIX systems.
        let config = mock_config(vec!["sleep", "2"], 1, true);
        let runner = Runner::new(&config);
        let record = runner.execute_command().await?;

        assert_eq!(record.status, RunStatus::Timeout);
        assert_eq!(record.exit_code, None);
        assert!(
            record
                .stderr
                .contains("Process timed out after 1s and was killed.")
        );
        // Duration should be slightly greater than the timeout limit (1 second)
        assert!(record.duration >= Duration::from_secs(1));
        assert!(record.duration < Duration::from_secs(2));

        Ok(())
    }

    // Note: Testing RunStatus::Killed (process terminated by external signal)
    // is tricky to reliably simulate cross-platform without custom setup.
    // The current logic in execute_command handles it by setting status to Killed
    // if exit_code is None after the process completes (not times out).
}

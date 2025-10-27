use crate::config::CliConfig;
use crate::analyzer::{RunRecord, RunStatus};
use anyhow::{Result, Context, bail};
use std::process::Stdio;
use std::time::Duration;
use tokio::time::timeout;

/// Core logic for executing the command and handling its lifecycle.
pub struct Runner<'a> {
    config: &'a CliConfig,
}

impl<'a> Runner<'a> {
    pub fn new(config: &'a CliConfig) -> Self {
        Runner { config }
    }

    /// Executes the configured command, respecting the per-run timeout.
    pub async fn execute_command(&self) -> Result<RunRecord> {
        let (exec, args) = self.config.executable_and_args();
        let start_time = std::time::Instant::now();

        if self.config.verbose {
            eprintln!("\n=> Starting run: {} {}", exec, args.join(" "));
        }

        // 1. Build the command
        let mut command = tokio::process::Command::new(exec);
        command
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()); // Capture stderr

        // 2. Spawn the process
        // We use 'let mut' so we can call child.kill() later if needed.
        let mut child = command.spawn()
            .with_context(|| format!("Failed to spawn command: {}", exec))?;

        let output = if let Some(secs) = self.config.run_timeout_sec {
            let limit = Duration::from_secs(secs);

            // 3. Apply timeout logic
            // Use child.wait() which takes &mut self, preventing the move that caused the E0382 error.
            match timeout(limit, child.wait()).await {
                Ok(Ok(_status)) => {
                    // Process completed normally. Now safely call wait_with_output
                    // to collect the buffers and consume the child handle. This call is instantaneous.
                    child.wait_with_output().await?
                },
                Ok(Err(e)) => bail!("Error waiting for child status: {}", e),
                Err(_) => {
                    // Timeout occurred: 'child' is available because child.wait() only borrowed it.
                    // Attempt to kill the child process
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
        } else {
            // No timeout configured, wait indefinitely and consume the child handle in one step.
            child.wait_with_output().await?
        };

        // Output and duration calculation are now outside the conditional logic
        let duration = start_time.elapsed();
        let exit_code = output.status.code();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if self.config.verbose {
            eprintln!("<= Run finished. Code: {:?}, Duration: {:.3}ms", exit_code, duration.as_secs_f64() * 1000.0);
            if !stdout.is_empty() {
                eprintln!("   --- STDOUT ---\n{}\n   --- END STDOUT ---", stdout.trim());
            }
            if !stderr.is_empty() {
                eprintln!("   --- STDERR ---\n{}\n   --- END STDERR ---", stderr.trim());
            }
        }

        Ok(RunRecord {
            status: RunStatus::Completed,
            exit_code,
            duration,
            stdout,
            stderr,
        })
    }
}

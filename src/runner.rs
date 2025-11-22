use crate::analyzer::{AnalysisTracker, RunRecord};
use crate::config::CliConfig;
use crate::execution::{Execution, TimedCommandExecution};
use anyhow::Result;
use std::process::Stdio;
use std::time::Duration;

pub struct Runner<'a> {
    config: &'a CliConfig,
}

impl<'a> Runner<'a> {
    pub fn new(config: &'a CliConfig) -> Self {
        Runner { config }
    }

    pub async fn execute_command(&self, tracker: &mut AnalysisTracker) -> Result<RunRecord> {
        let (exec, args) = self.config.executable_and_args();
        let mut command = tokio::process::Command::new(exec);
        command
            .args(args)
            .stdout(Stdio::piped()) // Capture stdout
            .stderr(Stdio::piped()); // Capture stderr

        // Apply custom environment variables
        for env_var in &self.config.env {
            if let Some((key, value)) = env_var.split_once('=') {
                command.env(key, value);
            } else {
                eprintln!(
                    "Warning: Invalid environment variable format '{}'. Expected KEY=VALUE",
                    env_var
                );
            }
        }

        let secs = self.config.single_run_timeout_sec;
        let limit = Duration::from_secs(secs);

        let result = TimedCommandExecution {
            command,
            timeout: limit,
        };

        let run_record = result.execute().await;

        if self.config.verbose {
            // TODO: decide on this
            // utils::print_struct_as_json(&run_record);
        }

        tracker.record(&run_record);

        Ok(run_record)
    }
}

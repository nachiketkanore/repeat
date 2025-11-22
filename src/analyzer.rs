use crate::utils;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::Instant;

/// Status indicating the result of a single command execution.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub enum RunStatus {
    Completed,
    Timeout,
    Killed,
}

/// Stores the detailed results of a single command execution.
#[derive(Serialize)]
pub struct RunRecord {
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    // TODO: this should have the actual command executed
    // pub command: String,
}

/// Aggregates performance statistics across all runs.
#[derive(Serialize, Debug, Deserialize)]
pub struct AnalysisTracker {
    pub verbose: bool,
    pub total_runs: u64,
    pub timeout_runs: u64,
    pub total_duration: Duration,
    pub total_completed_duration: Duration,
    pub min_run_duration: Duration,
    pub max_run_duration: Duration,
    pub exit_code_counts: HashMap<i32, u64>,
}

impl AnalysisTracker {
    /// Creates a new analysis tracker, setting the overall start time.
    pub fn new(verbose: bool) -> Self {
        AnalysisTracker {
            verbose,
            total_runs: 0,
            timeout_runs: 0,
            total_duration: Duration::ZERO,
            total_completed_duration: Duration::ZERO,
            min_run_duration: Duration::MAX,
            max_run_duration: Duration::ZERO,
            exit_code_counts: HashMap::new(),
        }
    }

    /// Records the results of a single command execution and updates metrics.
    pub fn record(&mut self, record: &RunRecord) {
        self.total_runs += 1;

        // Update timing metrics
        self.total_completed_duration += record.duration;
        self.min_run_duration = self.min_run_duration.min(record.duration);
        self.max_run_duration = self.max_run_duration.max(record.duration);

        // Update status and exit code counts
        match record.status {
            RunStatus::Timeout => self.timeout_runs += 1,
            RunStatus::Completed | RunStatus::Killed => {
                if let Some(code) = record.exit_code {
                    *self.exit_code_counts.entry(code).or_insert(0) += 1;
                }
            }
        }
    }

    /// Generates and prints the final analysis report.
    pub fn report(&mut self, start_instant: Instant) {
        self.total_duration = start_instant.elapsed();
        utils::print_struct_as_json(&self);
    }
}

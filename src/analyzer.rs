use crate::utils;
use chrono::{DateTime, Utc};
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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a basic RunRecord
    fn create_record(status: RunStatus, code: Option<i32>, ms: u64) -> RunRecord {
        RunRecord {
            status,
            exit_code: code,
            duration: Duration::from_millis(ms),
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    #[test]
    fn new_tracker_is_empty() {
        let tracker = AnalysisTracker::new(false);
        assert_eq!(tracker.total_runs, 0);
        assert_eq!(tracker.timeout_runs, 0);
        assert_eq!(tracker.total_duration, Duration::ZERO);
        assert_eq!(tracker.max_run_duration, Duration::ZERO);
        assert!(tracker.min_run_duration > Duration::ZERO); // Should be MAX, which is greater than ZERO
        assert!(tracker.exit_code_counts.is_empty());
    }

    #[test]
    fn record_successful_run() {
        let mut tracker = AnalysisTracker::new(false);
        let record = create_record(RunStatus::Completed, Some(0), 100);
        tracker.record(&record);

        assert_eq!(tracker.total_runs, 1);
        assert_eq!(tracker.timeout_runs, 0);
        assert_eq!(tracker.total_completed_duration, Duration::from_millis(100));
        assert_eq!(tracker.min_run_duration, Duration::from_millis(100));
        assert_eq!(tracker.max_run_duration, Duration::from_millis(100));
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 1);
    }

    #[test]
    fn record_failing_and_killed_runs() {
        let mut tracker = AnalysisTracker::new(false);
        let fail_record = create_record(RunStatus::Completed, Some(1), 50);
        let killed_record = create_record(RunStatus::Killed, Some(137), 200);

        tracker.record(&fail_record);
        tracker.record(&killed_record);

        assert_eq!(tracker.total_runs, 2);
        assert_eq!(tracker.timeout_runs, 0);
        assert_eq!(*tracker.exit_code_counts.get(&1).unwrap_or(&0), 1);
        assert_eq!(*tracker.exit_code_counts.get(&137).unwrap_or(&0), 1);
        assert_eq!(tracker.total_completed_duration, Duration::from_millis(250));
        assert_eq!(tracker.min_run_duration, Duration::from_millis(50));
        assert_eq!(tracker.max_run_duration, Duration::from_millis(200));
    }

    #[test]
    fn record_timeout_run() {
        let mut tracker = AnalysisTracker::new(false);
        let record = create_record(RunStatus::Timeout, None, 500); // Timeout doesn't have an exit code
        tracker.record(&record);

        assert_eq!(tracker.total_runs, 1);
        assert_eq!(tracker.timeout_runs, 1);
        assert!(tracker.exit_code_counts.is_empty());
    }

    #[test]
    fn record_mixed_runs_and_check_metrics() {
        let mut tracker = AnalysisTracker::new(false);

        tracker.record(&create_record(RunStatus::Completed, Some(0), 200));
        tracker.record(&create_record(RunStatus::Completed, Some(1), 300));
        tracker.record(&create_record(RunStatus::Timeout, None, 50));
        tracker.record(&create_record(RunStatus::Completed, Some(0), 100)); // New min

        assert_eq!(tracker.total_runs, 4);
        assert_eq!(tracker.timeout_runs, 1);
        assert_eq!(tracker.total_completed_duration, Duration::from_millis(650));
        assert_eq!(tracker.min_run_duration, Duration::from_millis(50));
        assert_eq!(tracker.max_run_duration, Duration::from_millis(300));
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 2);
        assert_eq!(*tracker.exit_code_counts.get(&1).unwrap_or(&0), 1);
    }

    // Testing report output is difficult, but we can call it to ensure it doesn't panic.
    #[test]
    fn report_does_not_panic() {
        let mut tracker = AnalysisTracker::new(false);
        let start_instant = Instant::now();
        tracker.record(&create_record(RunStatus::Completed, Some(0), 10));
        tracker.report(start_instant); // Should run successfully
    }
}

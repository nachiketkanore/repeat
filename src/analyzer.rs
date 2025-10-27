use std::collections::HashMap;
use std::time::Duration;

/// Status indicating the result of a single command execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RunStatus {
    Completed,
    Timeout,
    Killed,
}

/// Stores the detailed results of a single command execution.
#[derive(Debug)]
pub struct RunRecord {
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
}

/// Aggregates performance statistics across all runs.
pub struct AnalysisTracker {
    total_runs: u64,
    timeout_runs: u64,
    total_duration: Duration,
    min_run_duration: Duration,
    max_run_duration: Duration,
    exit_code_counts: HashMap<i32, u64>,
    start_time: std::time::Instant,
}

impl AnalysisTracker {
    /// Creates a new analysis tracker, setting the overall start time.
    pub fn new() -> Self {
        AnalysisTracker {
            total_runs: 0,
            timeout_runs: 0,
            total_duration: Duration::ZERO,
            min_run_duration: Duration::MAX,
            max_run_duration: Duration::ZERO,
            exit_code_counts: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    /// Records the results of a single command execution and updates metrics.
    pub fn record(&mut self, record: &RunRecord) {
        self.total_runs += 1;

        // Update timing metrics
        self.total_duration += record.duration;
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
    pub fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let avg_duration = if self.total_runs > 0 {
            self.total_duration.as_secs_f64() / self.total_runs as f64
        } else {
            0.0
        };

        println!("\n--- RLR Execution Analysis ---");
        println!("Total Run Time:   {:.3}s", elapsed.as_secs_f64());
        println!("Total Executions: {}", self.total_runs);
        println!("Completed Runs:   {}", self.total_runs - self.timeout_runs);
        println!("Timeout Runs:     {}", self.timeout_runs);

        if self.total_runs > 0 {
            println!("\nExecution Duration Metrics:");
            println!("  Average: {:.3}ms", avg_duration * 1000.0);
            println!(
                "  Minimum: {:.3}ms",
                self.min_run_duration.as_secs_f64() * 1000.0
            );
            println!(
                "  Maximum: {:.3}ms",
                self.max_run_duration.as_secs_f64() * 1000.0
            );

            println!("\nExit Code Frequency:");
            let mut sorted_codes: Vec<(&i32, &u64)> = self.exit_code_counts.iter().collect();
            sorted_codes.sort_by_key(|a| a.0);

            for (code, count) in sorted_codes {
                println!("  Code {}: {} times", code, count);
            }
        }
        println!("------------------------------\n");
    }
}

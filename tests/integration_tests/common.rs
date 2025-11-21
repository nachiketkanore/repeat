use repeat::AnalysisTracker;
use serde_json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Finds the path to the compiled 'repeat' executable, building it in release mode if necessary.
pub fn get_binary_path() -> PathBuf {
    Path::new("target/release/repeat").to_path_buf()
}

/// Helper function to build and run the CLI binary with the given arguments, returning raw stdout.
pub fn run_repeat(args: &[&str]) -> Result<String, String> {
    let binary_path = get_binary_path();

    let output = Command::new(binary_path)
        .args(args)
        .output()
        .expect("Failed to execute compiled 'repeat' binary");

    // NOTE: Removed println! of stdout/stderr for cleaner test output when deserializing JSON.

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Capture both stdout and stderr on failure for better debugging
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Command failed with status: {}\nStdout: {}\nStderr: {}",
            output.status, stdout, stderr
        ))
    }
}

/// Helper function to build, run, and deserialize the JSON output.
pub fn run_and_parse_json(args: &[&str]) -> AnalysisTracker {
    // We assume running without -v will produce the final JSON report on stdout.
    let output = run_repeat(args).expect("CLI execution failed");

    // Attempt to deserialize the entire stdout as the AnalysisTracker JSON
    serde_json::from_str(&output).unwrap_or_else(|e| {
        panic!(
            "Failed to deserialize JSON output:\nError: {}\nOutput:\n{}",
            e, output
        )
    })
}

/// Helper to run a test and assert the final analysis report contains expected metrics.
pub fn assert_analysis_metrics(
    args: &[&str],
    expected_runs: u64,
    expected_timeout_runs: u64,
    expected_exit_code_counts: Vec<(i32, u64)>,
    expected_min_duration: Option<Duration>,
    expected_max_duration: Option<Duration>,
) {
    let tracker = run_and_parse_json(args);

    // 1. Check Total Executions
    assert_eq!(
        tracker.total_runs, expected_runs,
        "Total Executions mismatch. Expected: {}, Got: {}.\nTracker: {:?}",
        expected_runs, tracker.total_runs, tracker
    );

    // 2. Check Timeout Runs
    assert_eq!(
        tracker.timeout_runs, expected_timeout_runs,
        "Timeout Runs mismatch. Expected: {}, Got: {}.\nTracker: {:?}",
        expected_timeout_runs, tracker.timeout_runs, tracker
    );

    // 3. Check Exit Code Frequencies
    for (code, count) in &expected_exit_code_counts {
        let actual_count = tracker.exit_code_counts.get(&code).unwrap_or(&0);
        assert_eq!(
            *actual_count, *count,
            "Exit Code {} count mismatch. Expected: {}, Got: {}.\nTracker: {:?}",
            code, count, *actual_count, tracker
        );
    }

    // Check for unexpected exit codes (optional but good practice)
    for (code, count) in &tracker.exit_code_counts {
        if !expected_exit_code_counts.iter().any(|(c, _)| c == code) {
            assert_eq!(
                *count, 0,
                "Unexpected exit code {} found with count {}.\nTracker: {:?}",
                code, count, tracker
            );
        }
    }

    // 4. Check Duration bounds (optional)
    if let Some(min_d) = expected_min_duration {
        // Check min run time is approximately what is expected
        assert!(
            tracker.min_run_duration
                >= min_d
                    .checked_sub(Duration::from_millis(50))
                    .unwrap_or_default()
                && tracker.min_run_duration
                    <= min_d
                        .checked_add(Duration::from_millis(50))
                        .unwrap_or_default(),
            "Min Run Duration ({:?}) is not close to expected {:?}",
            tracker.min_run_duration,
            min_d
        );
    }
    if let Some(max_d) = expected_max_duration {
        // Check max run time is approximately what is expected
        assert!(
            tracker.max_run_duration
                >= max_d
                    .checked_sub(Duration::from_millis(50))
                    .unwrap_or_default()
                && tracker.max_run_duration
                    <= max_d
                        .checked_add(Duration::from_millis(50))
                        .unwrap_or_default(),
            "Max Run Duration ({:?}) is not close to expected {:?}",
            tracker.max_run_duration,
            max_d
        );
    }
}

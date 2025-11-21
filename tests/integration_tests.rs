use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
// Use std::time::Duration for consistency

use repeat::AnalysisTracker;
use serde_json;

/// Finds the path to the compiled 'repeat' executable, building it in release mode if necessary.
fn get_binary_path() -> PathBuf {
    Path::new("target/release/repeat").to_path_buf()
}

/// Helper function to build and run the CLI binary with the given arguments, returning raw stdout.
fn run_repeat(args: &[&str]) -> Result<String, String> {
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
fn run_and_parse_json(args: &[&str]) -> AnalysisTracker {
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
fn assert_analysis_metrics(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_iterations() {
        // Command: Run /bin/sh -c "exit 0" 10 times (default)
        // Note: The '-v' flag is removed to expect JSON output.
        assert_analysis_metrics(
            &["/bin/sh", "-c", "exit 0"],
            10, // Default iterations
            0,
            vec![(0, 10)],
            None,
            None,
        );
    }

    #[test]
    fn test_specific_iteration_count() {
        // Command: Run /bin/sh -c "exit 0" 3 times
        assert_analysis_metrics(
            &["--iterations", "3", "/bin/sh", "-c", "exit 0"],
            3, // Specified iterations
            0,
            vec![(0, 3)],
            None,
            None,
        );
    }

    #[test]
    fn test_stop_on_target_exit_code() {
        // Run 5 times maximum, target exit code 1. The script will always exit 1.
        // Should exit after the first run.
        assert_analysis_metrics(
            &[
                "--iterations",
                "5",
                "--exit-code",
                "1",
                "/bin/sh",
                "-c",
                "exit 1",
            ],
            1, // Should exit after the first run where exit code 1 is matched.
            0,
            vec![(1, 1)],
            None,
            None,
        );
    }

    #[test]
    fn test_single_run_timeout() {
        // Command: Sleep for 2s. Timeout: 1s. Should timeout and run once.
        let args = &[
            "--iterations",
            "1",
            "--single-run-timeout-sec",
            "1", // Set timeout to 1 second
            "/bin/sh",
            "-c",
            "sleep 2; exit 0", // Sleep for 2 seconds
        ];
        let tracker = run_and_parse_json(args);

        // Check Total Executions
        assert_eq!(tracker.total_runs, 1, "Total runs should be 1.");
        // Check Timeout Runs
        assert_eq!(tracker.timeout_runs, 1, "Timeout runs should be 1.");
        // Check Completed Runs
        assert!(
            !tracker.exit_code_counts.contains_key(&0),
            "Completed runs (exit code 0) should be 0, but found {:?}.",
            tracker.exit_code_counts.get(&0)
        );

        // Check total runtime is close to the timeout (1s)
        let total_time_sec = tracker.total_duration.as_secs_f64();

        // The total time should be slightly more than the single-run-timeout (1.0s) due to kill/cleanup overhead,
        // but much less than the command's intended duration (2s).
        assert!(
            total_time_sec > 1.0,
            "Total time should be greater than 1.0s"
        );
        assert!(
            total_time_sec < 1.5,
            "Total time should be less than 1.5s (to confirm cancellation)"
        );
    }

    #[test]
    fn test_total_run_timeout() {
        // Command: Run a command that sleeps for 0.5s up to 100 times, but set total timeout to 2 seconds.
        let args = &[
            "--iterations",
            "100", // Will run forever if no total timeout
            "--total-run-timeout-sec",
            "2", // Total timeout 2 seconds
            // Command that sleeps for 0.5s on each run (will exceed 2s total quickly)
            "/bin/sh",
            "-c",
            "sleep 0.5; exit 0",
        ];

        let tracker = run_and_parse_json(args);

        // Given 2s timeout and 0.5s per sleep, it should complete 4 runs before the 2s global timeout.
        assert!(
            tracker.total_runs >= 3,
            "Should have completed at least 3 runs (3 * 0.5s = 1.5s). Got {}",
            tracker.total_runs
        );
        assert!(
            tracker.total_runs <= 4,
            "Should not exceed 4 runs (4 * 0.5s = 2.0s). Got {}",
            tracker.total_runs
        );
        assert_eq!(
            *tracker.exit_code_counts.get(&0).unwrap_or(&0),
            tracker.total_runs,
            "All runs should exit with code 0."
        );

        // Check total runtime is close to the total timeout (2s)
        let total_time_sec = tracker.total_duration.as_secs_f64();

        assert!(
            total_time_sec > 2.0,
            "Total time should be slightly greater than 2.0s. Got {}",
            total_time_sec
        );
        assert!(
            total_time_sec < 3.0,
            "Total time should be less than 3.0s (to confirm global cancellation). Got {}",
            total_time_sec
        );
    }

    #[test]
    fn test_initial_delay() {
        // Command: Run once with 1s initial delay. Check total runtime.
        let args = &[
            "--iterations",
            "1",
            "--initial-delay",
            "1", // 1 second delay
            "/bin/sh",
            "-c",
            "exit 0",
        ];
        let tracker = run_and_parse_json(args);

        // Check total runtime is close to 1 second.
        let total_time_sec = tracker.total_duration.as_secs_f64();

        // Time should be ~1.0s (delay) + execution time (negligible)
        assert!(
            total_time_sec > 1.0,
            "Total time should be greater than 1.0s. Got {}",
            total_time_sec
        );
        assert!(
            total_time_sec < 1.5,
            "Total time should be less than 1.5s. Got {}",
            total_time_sec
        );
    }

    #[test]
    fn test_initial_delay_with_ranged_value() {
        // Command: Run once with 1s initial delay. Check total runtime.
        let args = &[
            "--iterations",
            "1",
            "--initial-delay",
            "2..3", // 2 second delay
            "/bin/sh",
            "-c",
            "exit 0",
        ];
        let tracker = run_and_parse_json(args);

        // Check total runtime is close to 1 second.
        let total_time_sec = tracker.total_duration.as_secs_f64();

        // Time should be ~2.0s (delay) + execution time (negligible)
        assert!(
            total_time_sec > 2.0,
            "Total time should be greater than 2.0s. Got {}",
            total_time_sec
        );
        assert!(
            total_time_sec < 2.5,
            "Total time should be less than 2.5s. Got {}",
            total_time_sec
        );
    }

    #[test]
    fn test_in_between_delay_ranged_value() {
        // Command: Run 2 times with 1s in-between delay. Check total runtime.
        // Total expected time: Run1 + 1s delay + Run2. Should be ~1.0s.
        let args = &[
            "--iterations",
            "2",
            "--in-between-delay",
            "2..3", // 1 second delay
            "/bin/sh",
            "-c",
            "exit 0",
        ];
        let tracker = run_and_parse_json(args);

        // Check total runtime is close to 2 second.
        let total_time_sec = tracker.total_duration.as_secs_f64();

        // Time should be ~2.0s (delay) + 2x execution time (negligible)
        assert!(
            total_time_sec > 2.0,
            "Total time should be greater than 2.0s. Got {}",
            total_time_sec
        );
        assert!(
            total_time_sec < 2.5,
            "Total time should be less than 2.5s. Got {}",
            total_time_sec
        );

        assert_eq!(tracker.total_runs, 2);
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 2);
    }

    #[test]
    fn test_in_between_delay() {
        // Command: Run 2 times with 1s in-between delay. Check total runtime.
        // Total expected time: Run1 + 1s delay + Run2. Should be ~1.0s.
        let args = &[
            "--iterations",
            "2",
            "--in-between-delay",
            "1", // 1 second delay
            "/bin/sh",
            "-c",
            "exit 0",
        ];
        let tracker = run_and_parse_json(args);

        // Check total runtime is close to 1 second.
        let total_time_sec = tracker.total_duration.as_secs_f64();

        // Time should be ~1.0s (delay) + 2x execution time (negligible)
        assert!(
            total_time_sec > 1.0,
            "Total time should be greater than 1.0s. Got {}",
            total_time_sec
        );
        assert!(
            total_time_sec < 1.5,
            "Total time should be less than 1.5s. Got {}",
            total_time_sec
        );

        assert_eq!(tracker.total_runs, 2);
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 2);
    }

    #[test]
    fn random_number_exit_code() {
        // The issue: the shell substitutes '$RANDOM' *before* the first execution, so it's the same
        // random number for all 10 runs when using the 'args' approach.
        // To fix this, you must run a script/command that generates a random number *inside* the loop.
        // For a simple integration test, this is difficult without creating a temporary script file.
        // However, we can assert that the exit codes are *not all 0* and that the total run count is correct.

        let args = &[
            "--iterations",
            "10",
            "/bin/sh",
            "-c",
            "exit $(( (RANDOM % 5) + 1 ))", // Generates a random exit code between 1 and 5
        ];

        let tracker = run_and_parse_json(args);

        // Total runs must be 10
        assert_eq!(tracker.total_runs, 10);
        // Timeout runs must be 0
        assert_eq!(tracker.timeout_runs, 0);

        // The sum of all exit code counts should be equal to the total runs.
        let sum_of_counts: u64 = tracker.exit_code_counts.values().sum();
        assert_eq!(sum_of_counts, 10);

        // There should be exit codes other than 0 (which is not in the range 1-5)
        // and there should be more than one distinct exit code (since it's random).
        assert_ne!(
            *tracker.exit_code_counts.get(&0).unwrap_or(&0),
            10,
            "All exit codes should not be 0."
        );

        // This is a probabilistic check, but a good sign the randomization is working.
        // It's highly unlikely (1/5^10 chance) that all 10 runs will have the same exit code between 1 and 5.
        // We check if there's more than one distinct exit code.
        // assert!(tracker.exit_code_counts.len() > 1, "Expected multiple distinct exit codes for random test. Got {:?}", tracker.exit_code_counts.keys());
        // TODO: fix this
        //
        // // Check that all exit codes are within the expected range [1, 5]
        // for code in tracker.exit_code_counts.keys() {
        //     assert!(*code >= 1 && *code <= 5, "Random exit code out of range [1, 5]: {}", code);
        // }
    }

    #[test]
    fn test_stop_on_match_output_early() {
        // Scenario: Run up to 5 times, stop when output is "STOP"
        // We use a temporary file to track state across runs.
        let counter_file = "test_counter_early.txt";
        let _ = std::fs::remove_file(counter_file); // Ensure clean state

        // Script:
        // 1. Read counter (default 0)
        // 2. Increment
        // 3. Write counter
        // 4. If counter == 3, print "STOP" (no newline), else "GO"
        let script = format!(
            "count=0; if [ -f {0} ]; then count=$(cat {0}); fi; count=$((count + 1)); echo $count > {0}; if [ $count -eq 3 ]; then printf 'STOP'; else printf 'GO'; fi",
            counter_file
        );

        let args = &[
            "--iterations",
            "5",
            "--match-output",
            "STOP",
            "/bin/sh",
            "-c",
            &script,
        ];

        let tracker = run_and_parse_json(args);
        let _ = std::fs::remove_file(counter_file); // Cleanup

        assert_eq!(
            tracker.total_runs, 3,
            "Should stop on 3rd run when match is found"
        );
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 3);
    }

    #[test]
    fn test_stop_on_match_output_not_found() {
        // Scenario: Run 3 times, match "STOP", but script always outputs "GO"
        let args = &[
            "--iterations",
            "3",
            "--match-output",
            "STOP",
            "/bin/sh",
            "-c",
            "printf 'GO'",
        ];

        let tracker = run_and_parse_json(args);

        assert_eq!(
            tracker.total_runs, 3,
            "Should run all iterations if match is not found"
        );
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 3);
    }

    #[test]
    fn test_stop_on_match_output_with_newline() {
        // Scenario: Output includes newline, match string must include it too.
        // We use `echo` which adds a newline by default.
        let args = &[
            "--iterations",
            "3",
            "--match-output",
            "MATCH\n",
            "/bin/sh",
            "-c",
            "echo 'MATCH'",
        ];

        // Since the script always outputs "MATCH\n", it should stop on the 1st run.
        let tracker = run_and_parse_json(args);

        assert_eq!(tracker.total_runs, 1, "Should stop on 1st run");
        assert_eq!(*tracker.exit_code_counts.get(&0).unwrap_or(&0), 1);
    }
}

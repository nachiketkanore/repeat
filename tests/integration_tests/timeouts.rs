use crate::common::*;

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

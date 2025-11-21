use crate::common::*;

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
fn random_number_exit_code() {
    // We use /bin/bash because /bin/sh (often dash) does not support $RANDOM.
    // We want to ensure that the command is re-evaluated for each iteration,
    // producing different exit codes.

    let args = &[
        "--iterations",
        "10",
        "/bin/bash",
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
    assert!(
        tracker.exit_code_counts.len() > 1,
        "Expected multiple distinct exit codes for random test. Got {:?}",
        tracker.exit_code_counts.keys()
    );

    // Check that all exit codes are within the expected range [1, 5]
    for code in tracker.exit_code_counts.keys() {
        assert!(
            *code >= 1 && *code <= 5,
            "Random exit code out of range [1, 5]: {}",
            code
        );
    }
}

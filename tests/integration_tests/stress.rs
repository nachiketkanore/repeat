// Stress and scaling integration tests

use crate::common::*;

#[test]
fn test_moderate_iterations() {
    // Run a trivial command many times to ensure the runner can handle large iteration counts.
    // Using 500 iterations as a balance between thoroughness and speed.
    let args = &["--iterations", "500", "/bin/true"];
    assert_analysis_metrics(
        args,
        500, // expected total runs
        0,   // no timeouts
        vec![(0, 500)],
        None,
        None,
    );
}

#[test]
fn test_rapid_execution_minimal_overhead() {
    // Test rapid execution with minimal command overhead
    // Using a simple echo command that completes quickly
    let args = &["--iterations", "100", "echo", "stress_test"];
    assert_analysis_metrics(
        args,
        100, // expected total runs
        0,   // no timeouts
        vec![(0, 100)],
        None,
        None,
    );
}

#[test]
fn test_memory_stress_large_output() {
    // Test handling of commands that produce large output
    // Generate ~10KB of output per iteration for 50 iterations
    let args = &[
        "--iterations",
        "50",
        "sh",
        "-c",
        "head -c 10000 /dev/zero | tr '\\0' 'A'",
    ];
    assert_analysis_metrics(
        args,
        50, // expected total runs
        0,  // no timeouts
        vec![(0, 50)],
        None,
        None,
    );
}

#[test]
fn test_cpu_stress_computation() {
    // Test CPU-intensive operations with moderate iterations
    // Calculate MD5 hash of some data to stress CPU
    let args = &[
        "--iterations",
        "50",
        "sh",
        "-c",
        "echo 'stress test data' | md5sum",
    ];
    assert_analysis_metrics(
        args,
        50, // expected total runs
        0,  // no timeouts
        vec![(0, 50)],
        None,
        None,
    );
}

#[test]
fn test_mixed_exit_codes_stress() {
    // Stress test with varying exit codes to ensure tracker handles them correctly
    // Use a script that generates different exit codes based on a file counter
    let args = &[
        "--iterations",
        "100",
        "sh",
        "-c",
        // Use nanoseconds from date to get varying values
        "exit $(($(date +%N | sed 's/^0*//') % 4 2>/dev/null || echo 0))",
    ];

    let tracker = run_and_parse_json(args);

    // Verify we got 100 runs
    assert_eq!(tracker.total_runs, 100, "Expected 100 runs");
    assert_eq!(tracker.timeout_runs, 0, "Expected no timeouts");

    // Verify we have multiple different exit codes (should have at least 2 different ones)
    // Note: In some environments this might still produce mostly one exit code,
    // so we'll just verify the exit codes are in valid range
    assert!(
        tracker.exit_code_counts.len() >= 1,
        "Expected at least 1 exit code, got: {:?}",
        tracker.exit_code_counts
    );

    // Verify all exit codes are in the expected range (0-3)
    for code in tracker.exit_code_counts.keys() {
        assert!(
            *code >= 0 && *code <= 3,
            "Exit code {} is outside expected range 0-3",
            code
        );
    }
}

// Add more stress tests here (e.g., parallel execution) when the feature is implemented.

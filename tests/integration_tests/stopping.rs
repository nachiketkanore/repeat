use crate::common::*;

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

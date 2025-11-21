use crate::common::*;

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

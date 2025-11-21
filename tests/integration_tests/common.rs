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

/// Helper function to run a command and capture its stdout by writing to a temporary file.
/// This is useful for verifying that environment variables or other data are present in the output.
///
/// # Arguments
/// * `args` - Arguments to pass to the repeat binary (excluding the command itself)
/// * `command` - The shell command to execute
///
/// # Returns
/// The contents of the stdout captured in the temporary file
pub fn run_and_capture_stdout(args: &[&str], command: &str) -> String {
    use rand;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create a unique temporary file path using timestamp and random number
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let random: u32 = rand::random();
    let temp_file = PathBuf::from(format!(
        "/tmp/repeat_test_output_{}_{}.txt",
        timestamp, random
    ));

    // Clean up any existing file
    let _ = fs::remove_file(&temp_file);

    let temp_file_str = temp_file.to_str().unwrap();
    let full_command = format!("{} > {}", command, temp_file_str);

    // Build the full args array with the command
    let mut full_args = args.to_vec();
    full_args.push("sh");
    full_args.push("-c");
    full_args.push(&full_command);

    // Run the command
    let tracker = run_and_parse_json(&full_args);

    // Verify the command succeeded
    assert_eq!(tracker.timeout_runs, 0, "Command should not timeout");
    let exit_code_0_count = tracker.exit_code_counts.get(&0).unwrap_or(&0);
    assert!(
        *exit_code_0_count > 0,
        "Command should have at least one successful run"
    );

    // Read and return the output
    let output = fs::read_to_string(&temp_file).expect("Failed to read output file");

    // Clean up
    let _ = fs::remove_file(&temp_file);

    output
}

/// Helper function to assert that stdout contains expected strings.

/// # Arguments
/// * `args` - Arguments to pass to the repeat binary (excluding the command itself)
/// * `command` - The shell command to execute
/// * `expected_strings` - Strings that should be present in the stdout
pub fn assert_stdout_contains(args: &[&str], command: &str, expected_strings: &[&str]) {
    let output = run_and_capture_stdout(args, command);

    for expected in expected_strings {
        assert!(
            output.contains(expected),
            "Output should contain '{}', but got: {}",
            expected,
            output
        );
    }
}

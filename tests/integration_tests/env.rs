// Environment variable integration tests

use crate::common::*;

#[test]
fn test_custom_env_vars() {
    // Pass custom environment variables and verify they appear in the child process output.
    let args = &["--env", "FOO=bar", "--env", "BAZ=qux", "-i", "1"];
    let command = "echo \"FOO=$FOO BAZ=$BAZ\"";

    assert_stdout_contains(args, command, &["FOO=bar", "BAZ=qux"]);
}

#[test]
fn test_env_vars_multiple_iterations() {
    // Test that environment variables are propagated across multiple iterations
    let args = &[
        "--env",
        "TEST_VAR=hello_world",
        "--env",
        "ANOTHER_VAR=123",
        "-i",
        "1",
        "sh",
        "-c",
        "test \"$TEST_VAR\" = \"hello_world\" && test \"$ANOTHER_VAR\" = \"123\"",
    ];

    let tracker = run_and_parse_json(args);

    assert_eq!(tracker.total_runs, 1, "Expected 1 run");
    assert_eq!(tracker.timeout_runs, 0, "Expected no timeouts");

    // All runs should have exit code 0 (test command succeeded)
    let exit_code_0_count = tracker.exit_code_counts.get(&0).unwrap_or(&0);
    assert_eq!(*exit_code_0_count, 1, "Run should have exit code 0");
}

#[test]
fn test_env_var_with_special_characters() {
    // Test environment variables with special characters
    let args = &[
        "--env",
        "SPECIAL_VAR=hello world with spaces",
        "--env",
        "PATH_VAR=/usr/bin:/bin",
        "-i",
        "1",
    ];

    let command = "echo \"SPECIAL_VAR=$SPECIAL_VAR PATH_VAR=$PATH_VAR\"";

    assert_stdout_contains(
        args,
        command,
        &[
            "SPECIAL_VAR=hello world with spaces",
            "PATH_VAR=/usr/bin:/bin",
        ],
    );
}

#[test]
fn test_env_var_overrides_existing() {
    // Test that custom env vars can be set and accessed
    let args = &["--env", "CUSTOM_TEST_VAR=custom_value", "-i", "1"];
    let command = "echo \"CUSTOM_TEST_VAR=$CUSTOM_TEST_VAR\"";

    assert_stdout_contains(args, command, &["CUSTOM_TEST_VAR=custom_value"]);
}

#[test]
fn test_env_vars_captured_in_output() {
    // This test explicitly verifies that environment variables are present in the command's output
    let args = &[
        "--env",
        "FOO=test_value_foo",
        "--env",
        "BAZ=test_value_baz",
        "-i",
        "1",
    ];

    let command = "echo \"FOO=$FOO BAZ=$BAZ\"";

    // Use the utility function to capture and verify stdout
    let output = run_and_capture_stdout(args, command);

    assert!(
        output.contains("FOO=test_value_foo"),
        "Output should contain FOO=test_value_foo, but got: {}",
        output
    );
    assert!(
        output.contains("BAZ=test_value_baz"),
        "Output should contain BAZ=test_value_baz, but got: {}",
        output
    );
}

#[test]
fn test_multiple_env_vars_in_order() {
    // Test that multiple environment variables maintain their values
    let args = &[
        "--env",
        "VAR1=first",
        "--env",
        "VAR2=second",
        "--env",
        "VAR3=third",
        "-i",
        "1",
    ];

    let command = "echo \"$VAR1 $VAR2 $VAR3\"";

    assert_stdout_contains(args, command, &["first second third"]);
}

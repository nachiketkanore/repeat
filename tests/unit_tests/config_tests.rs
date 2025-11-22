// Unit tests for the config module

use repeat::config::CliConfig;

// Helper to create a mock CliConfig
fn mock_config(command: Vec<&str>) -> CliConfig {
    CliConfig {
        command: command.into_iter().map(String::from).collect(),
        exit_code: None,
        single_run_timeout_sec: 1,
        total_run_timeout_sec: 10,
        verbose: false,
        iterations: 1,
        ..Default::default()
    }
}

#[test]
fn executable_and_args_with_arguments() {
    let config = mock_config(vec!["cargo", "test", "--all"]);
    let (exec, args) = config.executable_and_args();

    assert_eq!(exec, "cargo");
    assert_eq!(args, vec!["test", "--all"]);
}

#[test]
fn executable_and_args_single_command() {
    let config = mock_config(vec!["ls"]);
    let (exec, args) = config.executable_and_args();

    assert_eq!(exec, "ls");
    assert_eq!(args.len(), 0);
}

#[test]
fn executable_and_args_command_with_path() {
    let config = mock_config(vec!["/usr/bin/python3", "script.py"]);
    let (exec, args) = config.executable_and_args();

    assert_eq!(exec, "/usr/bin/python3");
    assert_eq!(args, vec!["script.py"]);
}

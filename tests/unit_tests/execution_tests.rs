// Unit tests for the execution module

use repeat::analyzer::RunStatus;
use repeat::execution::{Execution, TimedCommandExecution};
use std::time::Duration;
use tokio::process::Command;

#[tokio::test]
async fn test_command_execution_output_match_success() {
    let mut command = Command::new("echo");
    command.arg("hello");

    let execution = TimedCommandExecution {
        timeout: Duration::from_secs(1),
        command,
    };

    let record = execution.execute().await;
    assert_eq!(record.status, RunStatus::Completed);
    assert_eq!(record.exit_code, Some(0));
    assert_eq!(record.stdout, "hello\n");
}

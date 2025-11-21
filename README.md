# Repeat

**Repeat** is a powerful command-line utility designed to execute a command repeatedly until a specific condition is met. It is perfect for stress testing, monitoring, debugging flaky scripts, or simply running a task multiple times.

## Features

*   **Iteration Control**: Specify the exact number of times to run a command.
*   **Conditional Stopping**: Stop execution based on:
    *   A specific exit code.
    *   A specific output string (stdout).
*   **Timeouts**: Enforce time limits for individual runs and the entire execution process.
*   **Delays**: Add fixed or random delays before the first run or between iterations.
*   **Analysis**: JSON output for easy parsing and analysis of execution metrics.

## Installation

To install `repeat`, ensure you have Rust and Cargo installed, then run:

```bash
cargo install --path .
```

## Usage

```bash
repeat [OPTIONS] <COMMAND>...
```

### Arguments

*   `<COMMAND>...`: The command and its arguments to execute repeatedly.

### Options

*   `-i, --iterations <ITERATIONS>`: Number of iterations to run (default: 10).
*   `--exit-code <CODE>`: Stop repeating if the command returns this specific exit code.
*   `--match-output <STRING>`: Stop repeating if the command's standard output matches this string.
*   `--single-run-timeout-sec <SECS>`: Maximum time (in seconds) allowed for a single execution (default: 10).
*   `--total-run-timeout-sec <SECS>`: Maximum time (in seconds) allowed for the entire operation (default: 100).
*   `--initial-delay <SECS>`: Delay (in seconds) before starting the first execution. Supports fixed values (e.g., `5`) or ranges (e.g., `1..5`).
*   `--in-between-delay <SECS>`: Delay (in seconds) between executions. Supports fixed values (e.g., `2`) or ranges (e.g., `1..3`).
*   `-v, --verbose`: Enable verbose logging.

## Examples

### 1. Basic Repetition
Run a command 10 times (default):
```bash
repeat echo "Hello World"
```

### 2. Specify Iterations
Run a command 5 times:
```bash
repeat --iterations 5 echo "Hello World"
```

### 3. Stop on Error (Exit Code)
Run a script until it fails (returns exit code 1):
```bash
repeat --exit-code 1 ./flaky-test.sh
```

### 4. Stop on Output Match
Run a command until it outputs "Ready":
```bash
repeat --match-output "Ready" ./check-service-status.sh
```

### 5. Timeouts
Kill a command if it takes longer than 2 seconds:
```bash
repeat --single-run-timeout-sec 2 sleep 5
```

### 6. Delays
Wait 5 seconds before starting, and wait 1 second between each run:
```bash
repeat --initial-delay 5 --in-between-delay 1 echo "Polling..."
```

### 7. Random Delays
Wait between 1 and 5 seconds between each run (useful for simulating jitter):
```bash
repeat --in-between-delay 1..5 ./simulate-traffic.sh
```

## Running Locally

To run the project locally without installing:

```bash
cargo run -- [OPTIONS] <COMMAND>
```

Example:
```bash
cargo run -- --iterations 3 echo "Running locally"
```

## Running Tests

This project contains both unit and integration tests.

To run all tests:
```bash
cargo test
```

To run a specific test:
```bash
cargo test test_name
```

## Contributing

Contributions are welcome! If you find a bug or want to add a feature, please follow these steps:

1.  **Fork** the repository.
2.  Create a new **branch** for your changes.
3.  Make your changes and ensure they are **tested**.
4.  Run `cargo fmt` to format your code.
5.  Run `cargo test` to ensure everything is working.
6.  Submit a **Pull Request**.

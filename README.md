# Repeat

**Repeat** is a powerful command-line utility designed to execute a command repeatedly until a specific condition is met. It is perfect for stress testing, monitoring, debugging flaky scripts, or simply running a task multiple times.

## Features

*   **Iteration Control**: Specify the exact number of times to run a command.
*   **Conditional Stopping**: Stop execution based on:
    *   A specific exit code.
    *   A specific output string (stdout).
*   **Environment Variables**: Pass custom environment variables to the command.
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
*   `--env <KEY=VALUE>`: Set environment variables for the command (can be specified multiple times).
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
Or using the short form:
```bash
repeat -i 5 echo "Hello World"
```

### 3. Stop on Error (Exit Code)
Run a script until it fails (returns exit code 1):
```bash
repeat --exit-code 1 ./flaky-test.sh
```

Run a script until it succeeds (returns exit code 0):
```bash
repeat --exit-code 0 ./deployment-check.sh
```

### 4. Stop on Output Match
Run a command until it outputs "Ready":
```bash
repeat --match-output "Ready" ./check-service-status.sh
```

Wait for a server to start:
```bash
repeat --match-output "Server listening" curl -s http://localhost:8080/health
```

### 5. Environment Variables
Pass custom environment variables to the command:
```bash
repeat --env "API_KEY=secret123" --env "ENV=production" ./deploy.sh
```

Set multiple environment variables for testing:
```bash
repeat -i 3 --env "DB_HOST=localhost" --env "DB_PORT=5432" ./test-db-connection.sh
```

Override existing environment variables:
```bash
repeat --env "PATH=/custom/bin:$PATH" which mycommand
```

### 6. Timeouts
Kill a command if it takes longer than 2 seconds:
```bash
repeat --single-run-timeout-sec 2 sleep 5
```

Stop the entire operation after 30 seconds:
```bash
repeat --total-run-timeout-sec 30 ./long-running-test.sh
```

Combine both timeouts:
```bash
repeat -i 100 --single-run-timeout-sec 5 --total-run-timeout-sec 60 ./api-test.sh
```

### 7. Delays
Wait 5 seconds before starting, and wait 1 second between each run:
```bash
repeat --initial-delay 5 --in-between-delay 1 echo "Polling..."
```

### 8. Random Delays
Wait between 1 and 5 seconds between each run (useful for simulating jitter):
```bash
repeat --in-between-delay 1..5 ./simulate-traffic.sh
```

Random initial delay and random in-between delays:
```bash
repeat --initial-delay 2..10 --in-between-delay 1..3 ./load-test.sh
```

### 9. Verbose Mode
See detailed output for each run:
```bash
repeat -v -i 3 echo "Debug mode"
```

### 10. Real-World Examples

#### Stress Testing
Run a load test 1000 times with random delays:
```bash
repeat -i 1000 --in-between-delay 0..2 curl -s http://localhost:8080/api/endpoint
```

#### Flaky Test Detection
Run a test until it fails, with a timeout:
```bash
repeat --exit-code 1 --total-run-timeout-sec 300 npm test
```

#### Service Health Monitoring
Poll a service every 2 seconds until it's ready:
```bash
repeat --match-output "healthy" --in-between-delay 2 curl -s http://localhost:8080/health
```

#### Database Connection Testing
Test database connectivity with custom environment variables:
```bash
repeat -i 10 \
  --env "DB_HOST=localhost" \
  --env "DB_PORT=5432" \
  --env "DB_NAME=testdb" \
  ./test-db-connection.sh
```

#### Deployment Verification
Wait for deployment to complete (max 5 minutes):
```bash
repeat \
  --match-output "Deployment successful" \
  --in-between-delay 10 \
  --total-run-timeout-sec 300 \
  kubectl get deployment my-app -o jsonpath='{.status.conditions[?(@.type=="Available")].status}'
```

#### API Rate Limit Testing
Test API with controlled request rate:
```bash
repeat -i 100 \
  --env "API_TOKEN=your_token" \
  --in-between-delay 1 \
  --single-run-timeout-sec 5 \
  curl -H "Authorization: Bearer \$API_TOKEN" https://api.example.com/endpoint
```

### 11. Combining Multiple Options
Run a comprehensive test with multiple conditions:
```bash
repeat \
  -i 50 \
  --exit-code 0 \
  --env "TEST_ENV=staging" \
  --env "LOG_LEVEL=debug" \
  --initial-delay 5 \
  --in-between-delay 2..5 \
  --single-run-timeout-sec 10 \
  --total-run-timeout-sec 600 \
  ./integration-test.sh
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

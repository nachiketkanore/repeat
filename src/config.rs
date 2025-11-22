use clap::Parser;
use rand::Rng;
use std::str::FromStr;

/// Stores the variant for range in the arguments
#[derive(Debug, Clone)]
pub enum DelayValue {
    Single(u64),
    Range { start: u64, end: u64 },
}

impl DelayValue {
    pub fn get_value(&self) -> u64 {
        match self {
            DelayValue::Single(secs) => *secs,

            DelayValue::Range { start, end } => {
                let mut rng = rand::rng();

                rng.random_range(*start..*end)
            }
        }
    }
}

fn parse_delay_value(s: &str) -> Result<DelayValue, String> {
    // 1. Check for the range separator ".."
    if let Some((start_str, end_str)) = s.split_once("..") {
        let start = u64::from_str(start_str.trim())
            .map_err(|e| format!("Invalid start value '{}': {}", start_str, e))?;

        let end = u64::from_str(end_str.trim())
            .map_err(|e| format!("Invalid end value '{}': {}", end_str, e))?;

        if start >= end {
            return Err(format!(
                "Invalid range '{}': start must be less than end.",
                s
            ));
        }

        Ok(DelayValue::Range { start, end })
    }
    // 2. If no range separator, try parsing as a single number
    else {
        u64::from_str(s.trim())
            .map(DelayValue::Single)
            .map_err(|e| format!("Invalid single number '{}': {}", s, e))
    }
}

impl Default for DelayValue {
    fn default() -> Self {
        // A single delay of 0 seconds makes sense as the default.
        DelayValue::Single(0)
    }
}

/// Configuration derived from command-line arguments
#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Repeat: Repeats a command until a condition is met."
)]
#[derive(Default)]
pub struct CliConfig {
    /// The command and its arguments to execute repeatedly.
    /// Example: repeat --exit-code 127 echo "Running..."
    #[clap(required = true, trailing_var_arg = true, value_name = "COMMAND")]
    pub command: Vec<String>,

    /// Exit the repeating loop when the command returns this specific exit code.
    #[clap(long, value_name = "CODE")]
    pub exit_code: Option<i32>,

    /// Maximum time (in seconds) allowed for a single execution. Kills the process if exceeded.
    #[clap(long, value_name = "SECS")]
    #[arg(long, default_value_t = 10)]
    pub single_run_timeout_sec: u64,

    /// Maximum time (in seconds) allowed for the entire execution. Kills the process if exceeded.
    #[clap(long, value_name = "SECS")]
    #[arg(long, default_value_t = 100)]
    pub total_run_timeout_sec: u64,

    /// Enable verbose logging of each run's output, exit code, and duration.
    #[clap(short, long)]
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Number of iterations for a given command
    #[clap(short, long)]
    #[arg(long, default_value_t = 10)]
    pub iterations: u64,

    /// Initial delay before starting command execution
    #[clap(long, value_name = "SECS")]
    #[arg(long, value_parser = parse_delay_value, default_value = "0")]
    // TODO: add tests for this new range based param
    // Currently, due to backward compatibilty properly configured the existing tests will pass
    pub initial_delay: DelayValue,

    /// Delay in between command execution
    #[clap(long, value_name = "SECS")]
    #[arg(long, value_parser = parse_delay_value, default_value = "0")]
    // TODO: add tests for this new range based param
    // Currently, due to backward compatibilty properly configured the existing tests will pass
    pub in_between_delay: DelayValue,

    /// Delay in between command execution
    #[clap(long, value_name = "TEXT")]
    pub match_output: Option<String>,

    /// Environment variables to pass to the command (can be specified multiple times)
    /// Format: KEY=VALUE
    #[clap(long = "env", value_name = "KEY=VALUE")]
    pub env: Vec<String>,
}

impl CliConfig {
    /// Utility to get the main command executable and its arguments separately.
    pub fn executable_and_args(&self) -> (&String, &[String]) {
        self.command
            .split_first()
            .expect("Command vector cannot be empty as it is required by clap")
    }
}

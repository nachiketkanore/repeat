use clap::Parser;

/// Configuration derived from command-line arguments for the Rust Loop Runner (RLR).
#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Rust Loop Runner (RLR): Repeats a command until a condition is met."
)]
pub struct CliConfig {
    /// The command and its arguments to execute repeatedly.
    /// Example: rlr --exit-code 127 echo "Running..."
    #[clap(required = true, trailing_var_arg = true, value_name = "COMMAND")]
    pub command: Vec<String>,

    /// Exit the repeating loop when the command returns this specific exit code.
    #[clap(long, value_name = "CODE")]
    pub exit_code: Option<i32>,

    /// Maximum time (in seconds) allowed for a single execution. Kills the process if exceeded.
    #[clap(long, value_name = "SECS")]
    pub run_timeout_sec: Option<u64>,

    /// Enable verbose logging of each run's output, exit code, and duration.
    #[clap(short, long)]
    pub verbose: bool,

    /// Number of iterations for a given command
    #[clap(short, long)]
    #[arg(long, default_value_t = 10)]
    pub iterations: u64,
}

impl CliConfig {
    /// Utility to get the main command executable and its arguments separately.
    pub fn executable_and_args(&self) -> (&String, &[String]) {
        self.command
            .split_first()
            .expect("Command vector cannot be empty as it is required by clap")
    }
}

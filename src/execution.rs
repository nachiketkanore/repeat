use std::pin::Pin;
use std::time::Duration;
use tokio::time::timeout;

pub trait Execution {
    async fn execute(self) -> ExecutionResult;
}

pub enum ExecutionResult {
    Success,
    Failure,
    Timeout,
}
pub struct TimedExecution<F>
where
    F: FnOnce() -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>,
{
    pub(crate) timeout: Duration,
    // The executor is now the function itself.
    pub(crate) executor: F,
}

impl<F> Execution for TimedExecution<F>
where
    F: FnOnce() -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>,
{
    async fn execute(self) -> ExecutionResult {
        let result = timeout(self.timeout, (self.executor)()).await;

        match result {
            Ok(Ok(_result)) => ExecutionResult::Success,
            Ok(Err(_)) => ExecutionResult::Failure,
            Err(_elapsed) => ExecutionResult::Timeout,
        }
    }
}

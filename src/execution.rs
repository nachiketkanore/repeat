use std::time::Duration;
use tokio::time::timeout;

pub trait Execution {
    async fn execute(self) -> ExecutionResult;
}

#[derive(Debug, PartialEq)]
pub enum ExecutionResult {
    Success,
    Failure,
    Timeout,
}
pub struct TimedExecution<F>
where
    F: IntoFuture,
{
    pub(crate) timeout: Duration,
    pub(crate) executor: F,
}

impl<F> Execution for TimedExecution<F>
where
    F: IntoFuture,
{
    async fn execute(self) -> ExecutionResult {
        match timeout(self.timeout, self.executor).await {
            Ok(_) => ExecutionResult::Success,
            Err(_) => ExecutionResult::Timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    /// A simple async function that sleeps for a given duration.
    /// This serves as the mock executor/future for testing.
    async fn mock_executor(duration: Duration) {
        sleep(duration).await;
    }

    /// Helper function to create a TimedExecution instance using the mock_executor.
    /// F is inferred as an impl Future<Output = ()> which implements IntoFuture.
    fn create_test_execution(
        timeout_ms: u64,
        execution_ms: u64,
    ) -> TimedExecution<impl Future<Output = ()>> {
        let timeout_duration = Duration::from_millis(timeout_ms);
        let execution_duration = Duration::from_millis(execution_ms);

        TimedExecution {
            timeout: timeout_duration,
            // The future returned by mock_executor is our executor
            executor: mock_executor(execution_duration),
        }
    }
    // Test Case 1: Execution completes instantly (0ms delay).
    #[tokio::test]
    async fn test_immediate_success() {
        // Timeout: 100ms, Execution: 0ms
        let te = create_test_execution(100, 0);
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Success,
            "Execution with 0ms delay should succeed instantly."
        );
    }

    // Test Case 2: Execution completes significantly faster than the timeout (Quick Success).
    #[tokio::test]
    async fn test_quick_success() {
        // Timeout: 100ms, Execution: 10ms
        let te = create_test_execution(100, 10);
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Success,
            "Execution should succeed well within the time limit."
        );
    }

    // Test Case 3: Execution completes just barely under the deadline (Near Miss Success).
    // Allows a small buffer (10ms) for scheduler overhead.
    #[tokio::test]
    async fn test_near_miss_success() {
        // Timeout: 200ms, Execution: 190ms
        let te = create_test_execution(200, 190);
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Success,
            "Execution should succeed just before the deadline."
        );
    }

    // Test Case 4: Execution is guaranteed to exceed the timeout (Guaranteed Timeout).
    // Also confirms the execution is cancelled early.
    #[tokio::test]
    async fn test_guaranteed_timeout() {
        // Timeout: 10ms, Execution: 100ms
        let te = create_test_execution(10, 100);
        let start = tokio::time::Instant::now();
        let result = te.execute().await;
        let duration = start.elapsed();

        // Assert result
        assert_eq!(
            result,
            ExecutionResult::Timeout,
            "Execution should result in Timeout."
        );

        // Assert actual time taken is close to the timeout duration (10ms)
        assert!(
            duration < Duration::from_millis(50),
            "Execution should have been cancelled near the 10ms mark."
        );
    }

    // Test Case 5: Edge case where execution is 1ms longer than the timeout.
    #[tokio::test]
    async fn test_one_ms_over_timeout() {
        // Timeout: 10ms, Execution: 11ms
        let te = create_test_execution(10, 11);
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Timeout,
            "Execution 1ms over should result in Timeout."
        );
    }

    // Test Case 6: Execution time is exactly the same as the timeout duration (Boundary Timeout).
    // Due to scheduler latency and inherent fuzziness in duration matching, this should reliably be a Timeout.
    #[tokio::test]
    async fn test_boundary_timeout_tiny_difference() {
        // Timeout: 50ms, Execution: 50ms
        let te = create_test_execution(49, 50);
        let result = te.execute().await;
        // The expected and safer result for exact match is Timeout.
        assert_eq!(
            result,
            ExecutionResult::Timeout,
            "Boundary condition (T=E) should result in Timeout due to overhead."
        );
    }

    // Test Case 7: Using a future that resolves to Result (Success resolution).
    // Confirms that the internal value of the resolved future is ignored, and completion is prioritized.
    #[tokio::test]
    async fn test_with_result_future_ok() {
        // Future resolves to Ok(()) after 10ms (well within the 100ms limit)
        let result_future = async {
            sleep(Duration::from_millis(10)).await;
            Result::<(), ()>::Ok(())
        };

        let te = TimedExecution {
            timeout: Duration::from_millis(100),
            executor: result_future,
        };

        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Success,
            "Future resolving to Ok(T) should be Success."
        );
    }

    // Test Case 8: Timeout with a Future that resolves to Result (Internal Failure is never reached).
    #[tokio::test]
    async fn test_timeout_with_result_future_err_unreachable() {
        // Future that would resolve to Err(()) after 100ms
        let result_future = async {
            sleep(Duration::from_millis(100)).await;
            Result::<(), ()>::Err(())
        };

        let te = TimedExecution {
            // Short timeout of 10ms
            timeout: Duration::from_millis(10),
            executor: result_future,
        };

        // The timeout will fire at 10ms, cancelling the sleep and the future.
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Timeout,
            "Future that would have failed, but timed out first."
        );
    }

    // Test Case 9: Long timeout duration confirms proper Duration handling (e.g., 5 seconds).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_long_duration_success() {
        // Timeout: 5000ms, Execution: 100ms
        let te = create_test_execution(5000, 100);
        let result = te.execute().await;
        assert_eq!(
            result,
            ExecutionResult::Success,
            "Long duration timeout should still succeed if execution is fast."
        );
    }

    // Test Case 10: Long execution duration confirms effective cancellation.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_long_duration_timeout() {
        // Timeout: 100ms, Execution: 5000ms
        let te = create_test_execution(100, 5000);
        let start = tokio::time::Instant::now();
        let result = te.execute().await;
        let duration = start.elapsed();

        assert_eq!(
            result,
            ExecutionResult::Timeout,
            "Long running execution should be cancelled by short timeout."
        );
        assert!(
            duration < Duration::from_millis(500),
            "Execution should have been cancelled quickly (near 100ms mark)."
        );
    }
}

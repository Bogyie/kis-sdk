use std::time::Duration;

use crate::{endpoint::OperationKind, error::KisError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    max_attempts: usize,
    backoff: Duration,
}

impl RetryPolicy {
    pub fn disabled() -> Self {
        Self {
            max_attempts: 1,
            backoff: Duration::ZERO,
        }
    }

    pub fn conservative_reads() -> Self {
        Self {
            max_attempts: 3,
            backoff: Duration::from_millis(25),
        }
    }

    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    pub fn backoff(&self) -> Duration {
        self.backoff
    }

    pub fn should_retry(
        &self,
        method: &str,
        operation_kind: OperationKind,
        error: &KisError,
        attempt: usize,
    ) -> bool {
        if attempt >= self.max_attempts || !error.retryable() {
            return false;
        }

        operation_kind == OperationKind::Read && method == "GET"
    }
}

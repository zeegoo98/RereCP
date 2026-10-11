use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PoolError<E> {
    #[error("connection borrow timed out after {0:?}")]
    Timeout(Duration),

    #[error("failed to create a new connection: {0}")]
    ConnectionFailed(#[source] E),

    #[error("connection pool has been closed")]
    PoolClosed,

    #[error("connection health check failed: {0}")]
    ValidationFailed(String),

    #[error("internal pool error: {0}")]
    Internal(String),
}

pub type Result<T, E> = std::result::Result<T, PoolError<E>>;

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn test_error_messge_formatting() {
        let timeout_err: PoolError<io::Error> = PoolError::Timeout(Duration::from_millis(500));
        assert_eq!(
            timeout_err.to_string(),
            "connection borrow timed out after 500ms"
        );

        let closed_err: PoolError<io::Error> = PoolError::PoolClosed;
        assert_eq!(closed_err.to_string(), "connection pool has been closed")
    }

    #[test]
    fn test_error_source_chaining() {
        let cause = io::Error::new(io::ErrorKind::ConnectionRefused, "DB port is closed");
        let connection_failed_err: PoolError<io::Error> = PoolError::ConnectionFailed(cause);
        assert_eq!(
            connection_failed_err.to_string(),
            "failed to create a new connection: DB port is closed"
        );
    }
}

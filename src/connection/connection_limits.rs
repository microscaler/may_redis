// Connection resource limits (Story 3, Issue #7).
//
// Provides ConnectionLimitError, DEFAULT_MAX_QUEUE_DEPTH, DEFAULT_MAX_REQUEST_SIZE.

/// Connection errors for resource limit violations.
#[derive(Debug)]
pub enum ConnectionLimitError {
    /// Request queue is full.
    QueueFull(usize),
    /// Request exceeds the maximum size.
    RequestTooLarge(usize, usize),
}

impl std::fmt::Display for ConnectionLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull(max) => {
                write!(f, "request queue is full (max {max} pending requests)")
            }
            Self::RequestTooLarge(max, got) => {
                write!(f, "request too large (max {max} bytes, got {got})")
            }
        }
    }
}

impl std::error::Error for ConnectionLimitError {}

/// Default limits for a safe connection.
///
/// Story 3, Issue #7, AC-3.1, AC-3.4.
pub const DEFAULT_MAX_QUEUE_DEPTH: usize = 1024;
pub const DEFAULT_MAX_REQUEST_SIZE: usize = 65536; // 64 KiB

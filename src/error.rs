//! Session error types

use std::fmt;

/// Errors that can occur during session operations
#[derive(Debug)]
pub enum SessionError {
    /// Error from the session store
    StoreError(String),
    /// Error during serialization/deserialization
    SerializationError(String),
    /// Invalid session ID format
    InvalidSessionId(String),
    /// Invalid cookie signature
    InvalidSignature,
    /// Session not found
    NotFound,
    /// Redis error (when redis-store feature is enabled)
    #[cfg(feature = "redis-store")]
    RedisError(redis::RedisError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::StoreError(msg) => write!(f, "Session store error: {}", msg),
            SessionError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            SessionError::InvalidSessionId(msg) => write!(f, "Invalid session ID: {}", msg),
            SessionError::InvalidSignature => write!(f, "Invalid cookie signature"),
            SessionError::NotFound => write!(f, "Session not found"),
            #[cfg(feature = "redis-store")]
            SessionError::RedisError(e) => write!(f, "Redis error: {}", e),
        }
    }
}

impl std::error::Error for SessionError {}

#[cfg(feature = "redis-store")]
impl From<redis::RedisError> for SessionError {
    fn from(err: redis::RedisError) -> Self {
        SessionError::RedisError(err)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(err: serde_json::Error) -> Self {
        SessionError::SerializationError(err.to_string())
    }
}

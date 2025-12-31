//! Session store trait

use async_trait::async_trait;
use crate::error::SessionError;
use crate::session::SessionData;

/// Trait for session storage backends
/// 
/// This trait is designed to be compatible with express-session store interface.
/// Implementations should store session data as JSON, with the key format:
/// `prefix + session_id`
#[async_trait]
pub trait SessionStore: Send + Sync + 'static {
    /// Get a session by ID
    /// 
    /// Returns None if session doesn't exist
    async fn get(&self, sid: &str) -> Result<Option<SessionData>, SessionError>;

    /// Set/update a session
    /// 
    /// The TTL should be derived from the session cookie's expires field
    async fn set(&self, sid: &str, session: &SessionData, ttl_secs: Option<u64>) -> Result<(), SessionError>;

    /// Destroy/delete a session
    async fn destroy(&self, sid: &str) -> Result<(), SessionError>;

    /// Touch a session - update its TTL without modifying data
    /// 
    /// This is called when the session is accessed but not modified
    async fn touch(&self, sid: &str, session: &SessionData, ttl_secs: Option<u64>) -> Result<(), SessionError>;

    /// Clear all sessions (optional)
    async fn clear(&self) -> Result<(), SessionError> {
        Err(SessionError::StoreError("clear not implemented".to_string()))
    }

    /// Get the count of all sessions (optional)
    async fn length(&self) -> Result<usize, SessionError> {
        Err(SessionError::StoreError("length not implemented".to_string()))
    }

    /// Get all session IDs (optional)
    async fn ids(&self) -> Result<Vec<String>, SessionError> {
        Err(SessionError::StoreError("ids not implemented".to_string()))
    }

    /// Get all sessions (optional)
    async fn all(&self) -> Result<Vec<SessionData>, SessionError> {
        Err(SessionError::StoreError("all not implemented".to_string()))
    }
}

//! In-memory session store
//!
//! This is primarily for development and testing.
//! For production, use RedisStore or another persistent store.

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::SessionStore;
use crate::error::SessionError;
use crate::session::SessionData;

struct StoredSession {
    data: SessionData,
    expires_at: Option<Instant>,
}

/// In-memory session store
///
/// Warning: This store is not suitable for production use because:
/// - Sessions are lost on server restart
/// - Sessions are not shared across multiple server instances
/// - Memory usage grows with number of sessions
pub struct MemoryStore {
    sessions: Arc<RwLock<HashMap<String, StoredSession>>>,
    prefix: String,
}

impl MemoryStore {
    /// Create a new memory store
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            prefix: "sess:".to_string(),
        }
    }

    /// Create a new memory store with a custom prefix
    pub fn with_prefix<S: Into<String>>(prefix: S) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            prefix: prefix.into(),
        }
    }

    /// Make a storage key from session ID
    fn make_key(&self, sid: &str) -> String {
        format!("{}{}", self.prefix, sid)
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write();
        let now = Instant::now();
        sessions.retain(|_, stored| match stored.expires_at {
            Some(exp) => exp > now,
            None => true,
        });
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryStore {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            prefix: self.prefix.clone(),
        }
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn get(&self, sid: &str) -> Result<Option<SessionData>, SessionError> {
        let key = self.make_key(sid);
        let sessions = self.sessions.read();

        if let Some(stored) = sessions.get(&key) {
            // Check if expired
            if let Some(exp) = stored.expires_at {
                if exp <= Instant::now() {
                    return Ok(None);
                }
            }
            Ok(Some(stored.data.clone()))
        } else {
            Ok(None)
        }
    }

    async fn set(
        &self,
        sid: &str,
        session: &SessionData,
        ttl_secs: Option<u64>,
    ) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        let expires_at = ttl_secs.map(|secs| Instant::now() + Duration::from_secs(secs));

        let stored = StoredSession {
            data: session.clone(),
            expires_at,
        };

        self.sessions.write().insert(key, stored);
        Ok(())
    }

    async fn destroy(&self, sid: &str) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        self.sessions.write().remove(&key);
        Ok(())
    }

    async fn touch(
        &self,
        sid: &str,
        _session: &SessionData,
        ttl_secs: Option<u64>,
    ) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        let mut sessions = self.sessions.write();

        if let Some(stored) = sessions.get_mut(&key) {
            stored.expires_at = ttl_secs.map(|secs| Instant::now() + Duration::from_secs(secs));
        }

        Ok(())
    }

    async fn clear(&self) -> Result<(), SessionError> {
        self.sessions.write().clear();
        Ok(())
    }

    async fn length(&self) -> Result<usize, SessionError> {
        self.cleanup_expired();
        Ok(self.sessions.read().len())
    }

    async fn ids(&self) -> Result<Vec<String>, SessionError> {
        self.cleanup_expired();
        let sessions = self.sessions.read();
        let prefix_len = self.prefix.len();
        Ok(sessions
            .keys()
            .map(|k| k[prefix_len..].to_string())
            .collect())
    }

    async fn all(&self) -> Result<Vec<SessionData>, SessionError> {
        self.cleanup_expired();
        let sessions = self.sessions.read();
        Ok(sessions.values().map(|s| s.data.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryStore::new();

        // Create session data
        let mut data = SessionData::new(3600);
        data.set("user", "alice");

        // Set session
        store.set("test-id", &data, Some(3600)).await.unwrap();

        // Get session
        let retrieved = store.get("test-id").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.get::<String>("user"), Some("alice".to_string()));

        // Destroy session
        store.destroy("test-id").await.unwrap();
        let retrieved = store.get("test-id").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_memory_store_expiry() {
        let store = MemoryStore::new();

        let data = SessionData::new(1);
        store.set("test-id", &data, Some(0)).await.unwrap(); // Already expired

        let retrieved = store.get("test-id").await.unwrap();
        assert!(retrieved.is_none());
    }
}

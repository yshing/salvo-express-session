//! Redis session store compatible with connect-redis
//! 
//! This store uses the same storage format as connect-redis:
//! - Key: `prefix + session_id` (default prefix: "sess:")
//! - Value: JSON serialized session data
//! - TTL: Based on session cookie expiration

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::sync::Arc;

use crate::error::SessionError;
use crate::session::SessionData;
use super::SessionStore;

/// Redis session store compatible with connect-redis
/// 
/// This store uses the same format as the Node.js connect-redis package,
/// allowing seamless session sharing between Rust and Node.js applications.
/// 
/// # Example
/// 
/// ```rust,ignore
/// use salvo_express_session::RedisStore;
/// 
/// let client = redis::Client::open("redis://127.0.0.1/")?;
/// let store = RedisStore::new(client).await?;
/// ```
pub struct RedisStore {
    conn: Arc<ConnectionManager>,
    prefix: String,
    default_ttl: u64,
}

impl RedisStore {
    /// Create a new Redis store with default settings
    /// 
    /// - Prefix: "sess:"
    /// - Default TTL: 86400 seconds (1 day)
    pub async fn new(client: redis::Client) -> Result<Self, SessionError> {
        let conn = ConnectionManager::new(client).await?;
        Ok(Self {
            conn: Arc::new(conn),
            prefix: "sess:".to_string(),
            default_ttl: 86400,
        })
    }

    /// Create a new Redis store from a connection string
    pub async fn from_url(url: &str) -> Result<Self, SessionError> {
        let client = redis::Client::open(url)
            .map_err(|e| SessionError::StoreError(format!("Failed to create Redis client: {}", e)))?;
        Self::new(client).await
    }

    /// Create a new Redis store with custom prefix
    pub async fn with_prefix(client: redis::Client, prefix: &str) -> Result<Self, SessionError> {
        let conn = ConnectionManager::new(client).await?;
        Ok(Self {
            conn: Arc::new(conn),
            prefix: prefix.to_string(),
            default_ttl: 86400,
        })
    }

    /// Create a new Redis store from an existing connection manager
    pub fn from_connection_manager(conn: ConnectionManager) -> Self {
        Self {
            conn: Arc::new(conn),
            prefix: "sess:".to_string(),
            default_ttl: 86400,
        }
    }

    /// Set the key prefix (default: "sess:")
    pub fn set_prefix(&mut self, prefix: &str) {
        self.prefix = prefix.to_string();
    }

    /// Set the default TTL in seconds (default: 86400 = 1 day)
    pub fn set_default_ttl(&mut self, ttl: u64) {
        self.default_ttl = ttl;
    }

    /// Build with custom prefix
    pub fn with_custom_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Build with custom default TTL
    pub fn with_default_ttl(mut self, ttl: u64) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Make a storage key from session ID
    fn make_key(&self, sid: &str) -> String {
        format!("{}{}", self.prefix, sid)
    }

    /// Get the TTL to use
    fn get_ttl(&self, ttl_secs: Option<u64>) -> u64 {
        ttl_secs.unwrap_or(self.default_ttl)
    }
}

impl Clone for RedisStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
            prefix: self.prefix.clone(),
            default_ttl: self.default_ttl,
        }
    }
}

#[async_trait]
impl SessionStore for RedisStore {
    async fn get(&self, sid: &str) -> Result<Option<SessionData>, SessionError> {
        let key = self.make_key(sid);
        let mut conn = (*self.conn).clone();
        
        let data: Option<String> = conn.get(&key).await?;
        
        match data {
            Some(json) => {
                let session: SessionData = serde_json::from_str(&json)?;
                
                // Check if expired (connect-redis doesn't do this, but it's a safety check)
                if session.cookie.is_expired() {
                    return Ok(None);
                }
                
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, sid: &str, session: &SessionData, ttl_secs: Option<u64>) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        let mut conn = (*self.conn).clone();
        
        let json = serde_json::to_string(session)?;
        let ttl = self.get_ttl(ttl_secs);
        
        if ttl > 0 {
            // Set with expiration (EX = seconds)
            conn.set_ex::<_, _, ()>(&key, &json, ttl).await?;
        } else {
            // If TTL is 0 or negative, the session should be destroyed
            conn.del::<_, ()>(&key).await?;
        }
        
        Ok(())
    }

    async fn destroy(&self, sid: &str) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        let mut conn = (*self.conn).clone();
        
        conn.del::<_, ()>(&key).await?;
        Ok(())
    }

    async fn touch(&self, sid: &str, session: &SessionData, ttl_secs: Option<u64>) -> Result<(), SessionError> {
        let key = self.make_key(sid);
        let mut conn = (*self.conn).clone();
        
        let ttl = self.get_ttl(ttl_secs);
        
        // Just update the TTL without touching the data
        // This is what connect-redis does with EXPIRE
        let _: bool = conn.expire(&key, ttl as i64).await?;
        
        // If EXPIRE returns false, the key doesn't exist, which is fine
        // connect-redis also doesn't check the return value
        let _ = session; // Silence unused warning
        
        Ok(())
    }

    async fn clear(&self) -> Result<(), SessionError> {
        let mut conn = (*self.conn).clone();
        
        // Get all keys matching our prefix
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        
        if !keys.is_empty() {
            conn.del::<_, ()>(keys).await?;
        }
        
        Ok(())
    }

    async fn length(&self) -> Result<usize, SessionError> {
        let mut conn = (*self.conn).clone();
        
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        
        Ok(keys.len())
    }

    async fn ids(&self) -> Result<Vec<String>, SessionError> {
        let mut conn = (*self.conn).clone();
        
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        
        let prefix_len = self.prefix.len();
        Ok(keys.into_iter()
            .map(|k| k[prefix_len..].to_string())
            .collect())
    }

    async fn all(&self) -> Result<Vec<SessionData>, SessionError> {
        let mut conn = (*self.conn).clone();
        
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        
        if keys.is_empty() {
            return Ok(vec![]);
        }
        
        let values: Vec<Option<String>> = conn.mget(&keys).await?;
        
        let sessions: Vec<SessionData> = values
            .into_iter()
            .filter_map(|v| v)
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();
        
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    // Tests require a running Redis instance
    // Run with: cargo test --features redis-store -- --ignored
    
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_redis_store_basic() {
        let store = RedisStore::from_url("redis://127.0.0.1/").await.unwrap();
        
        // Clear any existing test sessions
        store.clear().await.unwrap();
        
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
        
        // Touch session
        store.touch("test-id", &data, Some(7200)).await.unwrap();
        
        // Destroy session
        store.destroy("test-id").await.unwrap();
        let retrieved = store.get("test-id").await.unwrap();
        assert!(retrieved.is_none());
    }
}

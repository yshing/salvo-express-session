//! Session data structure compatible with express-session

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

/// Cookie data structure compatible with express-session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCookie {
    /// Original max age in milliseconds (as set initially)
    pub original_max_age: Option<i64>,
    
    /// Expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<DateTime<Utc>>,
    
    /// Secure flag
    #[serde(default)]
    pub secure: bool,
    
    /// HttpOnly flag
    #[serde(default = "default_http_only")]
    pub http_only: bool,
    
    /// Cookie path
    #[serde(default = "default_path")]
    pub path: String,
    
    /// Cookie domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    
    /// SameSite attribute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

fn default_http_only() -> bool {
    true
}

fn default_path() -> String {
    "/".to_string()
}

impl Default for SessionCookie {
    fn default() -> Self {
        Self {
            original_max_age: None,
            expires: None,
            secure: false,
            http_only: true,
            path: "/".to_string(),
            domain: None,
            same_site: None,
        }
    }
}

impl SessionCookie {
    /// Create a new session cookie with the given max age in seconds
    pub fn new(max_age_secs: u64) -> Self {
        let max_age_ms = (max_age_secs * 1000) as i64;
        let expires = Utc::now() + chrono::Duration::seconds(max_age_secs as i64);
        
        Self {
            original_max_age: Some(max_age_ms),
            expires: Some(expires),
            ..Default::default()
        }
    }

    /// Get remaining time in milliseconds
    pub fn max_age(&self) -> Option<i64> {
        self.expires.map(|exp| {
            let now = Utc::now();
            (exp - now).num_milliseconds()
        })
    }

    /// Touch the cookie - reset expiration based on original max age
    pub fn touch(&mut self) {
        if let Some(original) = self.original_max_age {
            let secs = original / 1000;
            self.expires = Some(Utc::now() + chrono::Duration::seconds(secs));
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        match self.expires {
            Some(exp) => exp < Utc::now(),
            None => false, // No expiry = browser session
        }
    }
}

/// Session data structure compatible with express-session/connect-redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Cookie information
    pub cookie: SessionCookie,
    
    /// Additional session data (flattened at same level as cookie)
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            cookie: SessionCookie::default(),
            data: HashMap::new(),
        }
    }
}

impl SessionData {
    /// Create a new session data with the given max age in seconds
    pub fn new(max_age_secs: u64) -> Self {
        Self {
            cookie: SessionCookie::new(max_age_secs),
            data: HashMap::new(),
        }
    }

    /// Get a value from session data
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a value in session data
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.to_string(), v);
        }
    }

    /// Remove a value from session data
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Clear all session data (except cookie)
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Check if session data is empty (no user data)
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Session wrapper that tracks modifications
pub struct Session {
    /// Session ID
    id: String,
    
    /// Session data
    data: Arc<RwLock<SessionData>>,
    
    /// Whether the session has been modified
    modified: Arc<AtomicBool>,
    
    /// Whether this is a new session
    is_new: bool,
    
    /// Whether the session should be destroyed
    destroy: Arc<AtomicBool>,
    
    /// Whether the session should be regenerated
    regenerate: Arc<AtomicBool>,
}

impl Session {
    /// Create a new session with the given ID and data
    pub fn new(id: String, data: SessionData, is_new: bool) -> Self {
        Self {
            id,
            data: Arc::new(RwLock::new(data)),
            modified: Arc::new(AtomicBool::new(false)),
            is_new,
            destroy: Arc::new(AtomicBool::new(false)),
            regenerate: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if this is a new session
    pub fn is_new(&self) -> bool {
        self.is_new
    }

    /// Check if the session has been modified
    pub fn is_modified(&self) -> bool {
        self.modified.load(Ordering::SeqCst)
    }

    /// Check if the session should be destroyed
    pub fn should_destroy(&self) -> bool {
        self.destroy.load(Ordering::SeqCst)
    }

    /// Check if the session should be regenerated
    pub fn should_regenerate(&self) -> bool {
        self.regenerate.load(Ordering::SeqCst)
    }

    /// Get a value from the session
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.read().get(key)
    }

    /// Set a value in the session
    pub fn set<T: Serialize>(&self, key: &str, value: T) {
        self.data.write().set(key, value);
        self.modified.store(true, Ordering::SeqCst);
    }

    /// Remove a value from the session
    pub fn remove(&self, key: &str) -> Option<Value> {
        let result = self.data.write().remove(key);
        if result.is_some() {
            self.modified.store(true, Ordering::SeqCst);
        }
        result
    }

    /// Check if a key exists in the session
    pub fn contains(&self, key: &str) -> bool {
        self.data.read().contains(key)
    }

    /// Clear all session data
    pub fn clear(&self) {
        self.data.write().clear();
        self.modified.store(true, Ordering::SeqCst);
    }

    /// Mark the session for destruction
    pub fn destroy(&self) {
        self.destroy.store(true, Ordering::SeqCst);
    }

    /// Mark the session for regeneration (new ID)
    pub fn regenerate(&self) {
        self.regenerate.store(true, Ordering::SeqCst);
        self.modified.store(true, Ordering::SeqCst);
    }

    /// Touch the session - update cookie expiration
    pub fn touch(&self) {
        self.data.write().cookie.touch();
    }

    /// Get a copy of the session data
    pub fn data(&self) -> SessionData {
        self.data.read().clone()
    }

    /// Get the session cookie
    pub fn cookie(&self) -> SessionCookie {
        self.data.read().cookie.clone()
    }

    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        self.data.read().cookie.is_expired()
    }

    /// Check if the session is empty (no user data)
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            data: Arc::clone(&self.data),
            modified: Arc::clone(&self.modified),
            is_new: self.is_new,
            destroy: Arc::clone(&self.destroy),
            regenerate: Arc::clone(&self.regenerate),
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("data", &*self.data.read())
            .field("modified", &self.modified.load(Ordering::SeqCst))
            .field("is_new", &self.is_new)
            .finish()
    }
}

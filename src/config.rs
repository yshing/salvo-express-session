//! Session configuration

use std::time::Duration;

/// Configuration for the session middleware
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Secret key(s) for signing cookies.
    /// The first secret is used for signing new cookies.
    /// All secrets are tried when verifying signatures (for secret rotation).
    pub secrets: Vec<String>,

    /// Name of the session cookie (default: "connect.sid")
    pub cookie_name: String,

    /// Cookie path (default: "/")
    pub cookie_path: String,

    /// Cookie domain (default: None - current domain only)
    pub cookie_domain: Option<String>,

    /// HttpOnly flag for cookie (default: true)
    pub cookie_http_only: bool,

    /// Secure flag for cookie (default: false)
    pub cookie_secure: bool,

    /// SameSite attribute for cookie
    pub cookie_same_site: SameSite,

    /// Max age in seconds (default: None = session cookie)
    /// When None, cookie expires when browser closes (non-persistent cookie)
    /// This is used for both cookie expiry and session TTL in store
    pub max_age: Option<u64>,

    /// Session key prefix in store (default: "sess:")
    pub prefix: String,

    /// Whether to save uninitialized sessions (default: false)
    /// If false, sessions are only saved when modified
    pub save_uninitialized: bool,

    /// Whether to force save on every request (default: false)
    pub resave: bool,

    /// Whether to reset cookie expiry on every request (default: false)
    pub rolling: bool,
}

/// SameSite cookie attribute
#[derive(Clone, Debug, PartialEq)]
pub enum SameSite {
    /// Strict - cookie only sent for same-site requests
    Strict,
    /// Lax - cookie sent for same-site requests and top-level navigations
    Lax,
    /// None - cookie sent for all requests (requires Secure)
    None,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            secrets: vec!["keyboard cat".to_string()],
            cookie_name: "connect.sid".to_string(),
            cookie_path: "/".to_string(),
            cookie_domain: None,
            cookie_http_only: true,
            cookie_secure: false,
            cookie_same_site: SameSite::Lax,
            max_age: None, // Session cookie by default (like express-session)
            prefix: "sess:".to_string(),
            save_uninitialized: false,
            resave: false,
            rolling: false,
        }
    }
}

impl SessionConfig {
    /// Create a new session configuration with the given secret
    pub fn new<S: Into<String>>(secret: S) -> Self {
        Self {
            secrets: vec![secret.into()],
            ..Default::default()
        }
    }

    /// Create a new session configuration with multiple secrets for rotation
    pub fn with_secrets<I, S>(secrets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            secrets: secrets.into_iter().map(|s| s.into()).collect(),
            ..Default::default()
        }
    }

    /// Set the cookie name (default: "connect.sid")
    pub fn with_cookie_name<S: Into<String>>(mut self, name: S) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Set the cookie path (default: "/")
    pub fn with_cookie_path<S: Into<String>>(mut self, path: S) -> Self {
        self.cookie_path = path.into();
        self
    }

    /// Set the cookie domain
    pub fn with_cookie_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }

    /// Set the HttpOnly flag (default: true)
    pub fn with_http_only(mut self, http_only: bool) -> Self {
        self.cookie_http_only = http_only;
        self
    }

    /// Set the Secure flag (default: false)
    pub fn with_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    /// Set the SameSite attribute (default: Lax)
    pub fn with_same_site(mut self, same_site: SameSite) -> Self {
        self.cookie_same_site = same_site;
        self
    }

    /// Set max age in seconds
    /// Pass None for session cookie (expires when browser closes)
    pub fn with_max_age(mut self, max_age: impl Into<Option<u64>>) -> Self {
        self.max_age = max_age.into();
        self
    }

    /// Set max age from Duration
    pub fn with_max_age_duration(mut self, duration: impl Into<Option<Duration>>) -> Self {
        self.max_age = duration.into().map(|d| d.as_secs());
        self
    }

    /// Set the session key prefix in store (default: "sess:")
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Set whether to save uninitialized sessions (default: false)
    pub fn with_save_uninitialized(mut self, save: bool) -> Self {
        self.save_uninitialized = save;
        self
    }

    /// Set whether to force save on every request (default: false)
    pub fn with_resave(mut self, resave: bool) -> Self {
        self.resave = resave;
        self
    }

    /// Set whether to reset cookie expiry on every request (default: false)
    pub fn with_rolling(mut self, rolling: bool) -> Self {
        self.rolling = rolling;
        self
    }

    /// Get max age as Duration
    pub fn max_age_duration(&self) -> Option<Duration> {
        self.max_age.map(Duration::from_secs)
    }
}

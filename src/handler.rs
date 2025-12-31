//! Express-session compatible middleware handler for Salvo

use salvo::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::{SessionConfig, SameSite};
use crate::cookie_signature::{sign, unsign_with_secrets};
use crate::session::{Session, SessionData};
use crate::store::SessionStore;

const SESSION_KEY: &str = "salvo.express.session";

/// Express-session compatible middleware for Salvo
/// 
/// This handler manages sessions in a way that is fully compatible with
/// Node.js express-session and connect-redis, allowing seamless session
/// sharing between Rust and Node.js applications.
pub struct ExpressSessionHandler<S: SessionStore> {
    store: Arc<S>,
    config: SessionConfig,
}

impl<S: SessionStore> ExpressSessionHandler<S> {
    /// Create a new session handler
    pub fn new(store: S, config: SessionConfig) -> Self {
        Self {
            store: Arc::new(store),
            config,
        }
    }

    /// Generate a new session ID
    fn generate_session_id(&self) -> String {
        // Use UUID v4 for session IDs, similar to uid-safe in Node.js
        Uuid::new_v4().to_string()
    }

    /// Get session ID from cookie
    fn get_session_id_from_cookie(&self, req: &Request) -> Option<String> {
        // Get the cookie value
        let cookie_value = req.cookie(&self.config.cookie_name)?;
        let signed_value = cookie_value.value();
        
        // URL decode the value (cookies are URL encoded)
        let decoded = match urlencoding::decode(signed_value) {
            Ok(d) => d.to_string(),
            Err(_) => signed_value.to_string(),
        };
        
        // Unsign the cookie value
        unsign_with_secrets(&decoded, &self.config.secrets)
    }

    /// Set session cookie on response
    fn set_session_cookie(&self, res: &mut Response, session_id: &str) {
        let signed = sign(session_id, &self.config.secrets[0]);
        
        // Build cookie with owned strings to avoid lifetime issues
        let cookie_name = self.config.cookie_name.clone();
        let cookie_path = self.config.cookie_path.clone();
        let cookie_domain = self.config.cookie_domain.clone();
        
        let mut cookie_builder = cookie::Cookie::build((cookie_name, signed))
            .path(cookie_path)
            .http_only(self.config.cookie_http_only)
            .secure(self.config.cookie_secure);
        
        if let Some(domain) = cookie_domain {
            cookie_builder = cookie_builder.domain(domain);
        }
        
        // Set max age
        if self.config.max_age > 0 {
            cookie_builder = cookie_builder.max_age(cookie::time::Duration::seconds(self.config.max_age as i64));
        }
        
        // Set SameSite
        cookie_builder = match self.config.cookie_same_site {
            SameSite::Strict => cookie_builder.same_site(cookie::SameSite::Strict),
            SameSite::Lax => cookie_builder.same_site(cookie::SameSite::Lax),
            SameSite::None => cookie_builder.same_site(cookie::SameSite::None),
        };
        
        res.add_cookie(cookie_builder.build());
    }

    /// Remove session cookie
    fn remove_session_cookie(&self, res: &mut Response) {
        let cookie_name = self.config.cookie_name.clone();
        let cookie_path = self.config.cookie_path.clone();
        
        let cookie = cookie::Cookie::build(cookie_name)
            .path(cookie_path)
            .max_age(cookie::time::Duration::ZERO)
            .build();
        
        res.add_cookie(cookie);
    }

    /// Calculate TTL for session storage
    fn get_session_ttl(&self, session_data: &SessionData) -> Option<u64> {
        // Use cookie expiration if available
        if let Some(expires) = session_data.cookie.expires {
            let now = chrono::Utc::now();
            let diff = expires - now;
            let secs = diff.num_seconds();
            if secs > 0 {
                return Some(secs as u64);
            }
        }
        // Fall back to config max age
        Some(self.config.max_age)
    }
}

impl<S: SessionStore> Clone for ExpressSessionHandler<S> {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            config: self.config.clone(),
        }
    }
}

#[async_trait]
impl<S: SessionStore> Handler for ExpressSessionHandler<S> {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        // Try to get session ID from cookie
        let (session_id, is_new, existing_data) = match self.get_session_id_from_cookie(req) {
            Some(sid) => {
                // Try to load existing session
                match self.store.get(&sid).await {
                    Ok(Some(data)) => {
                        // Check if session is expired
                        if data.cookie.is_expired() {
                            // Session expired, create new one
                            let new_id = self.generate_session_id();
                            let new_data = SessionData::new(self.config.max_age);
                            (new_id, true, new_data)
                        } else {
                            (sid, false, data)
                        }
                    }
                    Ok(None) => {
                        // Session not found, create new one
                        let new_id = self.generate_session_id();
                        let new_data = SessionData::new(self.config.max_age);
                        (new_id, true, new_data)
                    }
                    Err(e) => {
                        tracing::error!("Failed to load session: {}", e);
                        let new_id = self.generate_session_id();
                        let new_data = SessionData::new(self.config.max_age);
                        (new_id, true, new_data)
                    }
                }
            }
            None => {
                // No cookie, create new session
                let new_id = self.generate_session_id();
                let new_data = SessionData::new(self.config.max_age);
                (new_id, true, new_data)
            }
        };

        // Create session wrapper
        let session = Session::new(session_id.clone(), existing_data, is_new);
        
        // Store session in depot
        depot.insert(SESSION_KEY, session.clone());

        // Continue with the request
        ctrl.call_next(req, depot, res).await;

        // After request processing, handle session persistence
        
        // Check if session should be destroyed
        if session.should_destroy() {
            if let Err(e) = self.store.destroy(&session_id).await {
                tracing::error!("Failed to destroy session: {}", e);
            }
            self.remove_session_cookie(res);
            return;
        }

        // Check if session should be regenerated
        let final_session_id = if session.should_regenerate() {
            // Destroy old session
            if let Err(e) = self.store.destroy(&session_id).await {
                tracing::error!("Failed to destroy old session during regeneration: {}", e);
            }
            // Generate new ID
            self.generate_session_id()
        } else {
            session_id
        };

        let session_data = session.data();
        let ttl = self.get_session_ttl(&session_data);
        
        // Determine if we need to save
        let should_save = session.is_modified() 
            || self.config.resave 
            || (is_new && self.config.save_uninitialized)
            || session.should_regenerate();
        
        // Determine if we should set cookie
        let should_set_cookie = is_new 
            || session.should_regenerate()
            || (self.config.rolling && session.is_modified());

        if should_save {
            // Save session to store
            if let Err(e) = self.store.set(&final_session_id, &session_data, ttl).await {
                tracing::error!("Failed to save session: {}", e);
            }
        } else if !is_new && !session.is_modified() {
            // Touch session to reset TTL
            if let Err(e) = self.store.touch(&final_session_id, &session_data, ttl).await {
                tracing::error!("Failed to touch session: {}", e);
            }
        }

        if should_set_cookie {
            self.set_session_cookie(res, &final_session_id);
        }
    }
}

/// Get session from depot
pub fn get_session(depot: &Depot) -> Option<&Session> {
    depot.get::<Session>(SESSION_KEY).ok()
}

/// Get mutable session from depot (returns clone with shared state)
pub fn get_session_mut(depot: &mut Depot) -> Option<Session> {
    depot.get::<Session>(SESSION_KEY).ok().cloned()
}

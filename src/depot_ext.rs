//! Extension trait for Depot to easily access sessions

use salvo::Depot;
use crate::session::Session;

const SESSION_KEY: &str = "salvo.express.session";

/// Extension trait for Salvo's Depot to provide easy session access
pub trait SessionDepotExt {
    /// Get a reference to the session
    fn session(&self) -> Option<&Session>;
    
    /// Get a mutable session (returns a clone with shared atomic state)
    fn session_mut(&mut self) -> Option<Session>;
}

impl SessionDepotExt for Depot {
    fn session(&self) -> Option<&Session> {
        self.get::<Session>(SESSION_KEY).ok()
    }
    
    fn session_mut(&mut self) -> Option<Session> {
        self.get::<Session>(SESSION_KEY).ok().cloned()
    }
}

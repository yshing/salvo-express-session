//! # salvo-express-session
//!
//! Express-session compatible session middleware for Salvo web framework.
//!
//! This crate provides session management that is fully compatible with Node.js
//! express-session and connect-redis, allowing seamless migration or interoperability
//! between Rust and Node.js applications.
//!
//! ## Features
//!
//! - **Express-session compatible cookie format**: Uses the same `s:` prefix and HMAC-SHA256 signature
//! - **Connect-redis compatible storage**: Sessions stored in Redis with the same format as connect-redis
//! - **Pluggable storage backends**: Supports Redis, Memory, or custom stores
//! - **Full session lifecycle**: Create, read, update, delete, touch, and regenerate sessions
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use salvo::prelude::*;
//! use salvo_express_session::{ExpressSessionHandler, MemoryStore, SessionConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let store = MemoryStore::new();
//!     let session_config = SessionConfig::new("your-secret-key")
//!         .with_cookie_name("connect.sid")
//!         .with_max_age(86400);
//!
//!     let session_handler = ExpressSessionHandler::new(store, session_config);
//!
//!     let router = Router::new()
//!         .hoop(session_handler)
//!         .get(index);
//!
//!     Server::new(TcpListener::bind("127.0.0.1:5800"))
//!         .serve(router)
//!         .await;
//! }
//!
//! #[handler]
//! async fn index(depot: &mut Depot) -> &'static str {
//!     let session = depot.session_mut().unwrap();
//!     let views: i32 = session.get("views").unwrap_or(0);
//!     session.set("views", views + 1);
//!     "Hello, World!"
//! }
//! ```

pub mod config;
pub mod cookie_signature;
pub mod error;
pub mod handler;
pub mod session;
pub mod store;

pub use config::SessionConfig;
pub use error::SessionError;
pub use handler::ExpressSessionHandler;
pub use session::{Session, SessionData};
pub use store::{MemoryStore, SessionStore};

#[cfg(feature = "redis-store")]
pub use store::RedisStore;

/// Extension trait for Depot to easily access session
pub mod depot_ext;
pub use depot_ext::SessionDepotExt;

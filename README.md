# salvo-express-session

Express-session compatible session middleware for [Salvo](https://salvo.rs) web framework.

This crate provides session management that is **fully compatible** with Node.js [express-session](https://github.com/expressjs/session) and [connect-redis](https://github.com/tj/connect-redis), allowing seamless session sharing between Rust and Node.js applications.

## Features

- 🔄 **Express-session compatible** - Uses the same `s:` prefix and HMAC-SHA256 cookie signature format
- 🗄️ **Connect-redis compatible** - Sessions stored in Redis with identical format as connect-redis
- 🔌 **Pluggable storage** - Redis, Memory, or implement your own store
- 🔑 **Secret rotation** - Support for multiple secrets for zero-downtime rotation
- 🍪 **Full cookie control** - HttpOnly, Secure, SameSite, Domain, Path, MaxAge
- ⚡ **Async/await** - Fully async implementation

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
salvo-express-session = "0.1"
salvo = { version = "0.87", features = ["cookie"] }
tokio = { version = "1", features = ["full"] }
```

For Redis support:

```toml
[dependencies]
salvo-express-session = { version = "0.1", features = ["redis-store"] }
```

## Quick Start

### Basic Usage (Memory Store)

```rust
use salvo::prelude::*;
use salvo_express_session::{ExpressSessionHandler, MemoryStore, SessionConfig, SessionDepotExt};

#[handler]
async fn index(depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");
    
    let views: i32 = session.get("views").unwrap_or(0);
    session.set("views", views + 1);
    
    format!("Views: {}", views + 1)
}

#[tokio::main]
async fn main() {
    let store = MemoryStore::new();
    
    let config = SessionConfig::new("your-secret-key")
        .with_cookie_name("connect.sid")
        .with_max_age(86400); // 1 day
    
    let session_handler = ExpressSessionHandler::new(store, config);
    
    let router = Router::new()
        .hoop(session_handler)
        .get(index);
    
    let acceptor = TcpListener::new("127.0.0.1:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

### With Redis (Compatible with connect-redis)

```rust
use salvo::prelude::*;
use salvo_express_session::{ExpressSessionHandler, RedisStore, SessionConfig, SessionDepotExt};

#[tokio::main]
async fn main() {
    let store = RedisStore::from_url("redis://127.0.0.1/")
        .await
        .expect("Failed to connect to Redis");
    
    // Use same secret as your Node.js app for session sharing!
    let config = SessionConfig::new("keyboard cat")
        .with_cookie_name("connect.sid")
        .with_prefix("sess:")
        .with_max_age(86400);
    
    let session_handler = ExpressSessionHandler::new(store, config);
    
    // ... rest of your app
}
```

## Session API

```rust
use chrono::{Utc, Duration};

#[handler]
async fn example(depot: &mut Depot) {
    let session = depot.session_mut().expect("Session not found");
    
    // Get session ID
    let id = session.id();
    
    // Get/set values
    let user: Option<String> = session.get("user");
    session.set("user", "alice");
    
    // Remove a value
    session.remove("user");
    
    // Check if key exists
    if session.contains("user") {
        // ...
    }
    
    // Clear all session data
    session.clear();
    
    // Destroy session (removes from store and clears cookie)
    session.destroy();
    
    // Regenerate session ID (security best practice after login)
    session.regenerate();
    
    // Check session status
    let is_new = session.is_new();
    let is_modified = session.is_modified();
    
    // Dynamic cookie expiration (like express-session)
    // Set expiration to 1 hour from now
    session.set_cookie_expires(Some(Utc::now() + Duration::hours(1)));
    
    // Or set max age in seconds
    session.set_cookie_max_age_secs(3600); // 1 hour
    
    // Or set max age in milliseconds (like express-session)
    session.set_cookie_max_age(Some(60 * 60 * 1000)); // 1 hour
}
```

## Configuration Options

```rust
let config = SessionConfig::new("secret")
    // Cookie name (default: "connect.sid")
    .with_cookie_name("connect.sid")
    
    // Cookie path (default: "/")
    .with_cookie_path("/")
    
    // Cookie domain (default: None - current domain only)
    .with_cookie_domain("example.com")
    
    // HttpOnly flag (default: true)
    .with_http_only(true)
    
    // Secure flag - requires HTTPS (default: false)
    .with_secure(true)
    
    // SameSite attribute (default: Lax)
    .with_same_site(SameSite::Strict)
    
    // Max age in seconds (default: 86400 = 1 day)
    .with_max_age(3600)
    
    // Session key prefix in store (default: "sess:")
    .with_prefix("sess:")
    
    // Save uninitialized sessions (default: false)
    .with_save_uninitialized(false)
    
    // Force save on every request (default: false)
    .with_resave(false)
    
    // Reset cookie expiry on every request (default: false)
    .with_rolling(true);
```

## Secret Rotation

For zero-downtime secret rotation:

```rust
// New secret first, old secrets after
let config = SessionConfig::with_secrets(vec![
    "new-secret",
    "old-secret",
    "older-secret",
]);
```

New sessions are signed with the first secret. Existing sessions signed with any secret in the list are accepted.

## Node.js Compatibility

To share sessions between Rust and Node.js:

### Rust Configuration
```rust
let config = SessionConfig::new("keyboard cat")
    .with_cookie_name("connect.sid")
    .with_prefix("sess:");
```

### Node.js Configuration
```javascript
const session = require('express-session');
const RedisStore = require('connect-redis').default;

app.use(session({
    store: new RedisStore({ client: redisClient, prefix: 'sess:' }),
    secret: 'keyboard cat',
    name: 'connect.sid',
    resave: false,
    saveUninitialized: false,
}));
```

With matching configuration, sessions are fully interchangeable!

## Storage Format

Sessions are stored as JSON with this structure (compatible with express-session):

```json
{
    "cookie": {
        "originalMaxAge": 86400000,
        "expires": "2024-12-31T23:59:59.000Z",
        "secure": false,
        "httpOnly": true,
        "path": "/"
    },
    "user": "alice",
    "views": 42
}
```

## Custom Store Implementation

Implement the `SessionStore` trait for custom backends:

```rust
use async_trait::async_trait;
use salvo_express_session::{SessionStore, SessionData, SessionError};

struct MyStore;

#[async_trait]
impl SessionStore for MyStore {
    async fn get(&self, sid: &str) -> Result<Option<SessionData>, SessionError> {
        // ...
    }
    
    async fn set(&self, sid: &str, session: &SessionData, ttl: Option<u64>) -> Result<(), SessionError> {
        // ...
    }
    
    async fn destroy(&self, sid: &str) -> Result<(), SessionError> {
        // ...
    }
    
    async fn touch(&self, sid: &str, session: &SessionData, ttl: Option<u64>) -> Result<(), SessionError> {
        // ...
    }
}
```

## Examples

Run the basic example:
```bash
cargo run --example basic
```

Run the Redis example:
```bash
# Start Redis first
docker run -p 6379:6379 redis

# Run the example
cargo run --example with_redis --features redis-store
```

Test Node.js compatibility:
```bash
# Terminal 1: Start Rust app
cargo run --example with_redis --features redis-store

# Terminal 2: Start Node.js app
cd examples/node-compatibility
npm install
npm start

# Both apps share sessions via Redis!
```

## License

MIT OR Apache-2.0

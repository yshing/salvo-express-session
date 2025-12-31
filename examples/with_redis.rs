//! Redis session store example compatible with connect-redis
//!
//! This example shows how to use Redis as a session store in a way that is
//! compatible with Node.js express-session and connect-redis.
//!
//! Sessions created by this Rust application can be read by Node.js applications
//! and vice versa.

use salvo::prelude::*;
use salvo_express_session::{ExpressSessionHandler, RedisStore, SessionConfig, SessionDepotExt};
use serde::Serialize;

#[derive(Serialize)]
struct JsonResponse {
    server: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    counter: Option<i32>,
    #[serde(rename = "sessionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(rename = "lastModifiedBy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<&'static str>,
}

impl Default for JsonResponse {
    fn default() -> Self {
        Self {
            server: "rust",
            action: None,
            key: None,
            value: None,
            exists: None,
            counter: None,
            session_id: None,
            last_modified_by: None,
            status: None,
        }
    }
}

#[handler]
async fn health() -> Json<JsonResponse> {
    Json(JsonResponse {
        status: Some("ok"),
        ..Default::default()
    })
}

#[handler]
async fn index(depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");

    // Get current view count
    let views: i32 = session.get("views").unwrap_or(0);

    // Increment view count
    session.set("views", views + 1);

    format!(
        "Hello from Rust + Redis!\nViews: {}\nSession ID: {}\n\nThis session is compatible with Node.js express-session + connect-redis!",
        views + 1,
        session.id()
    )
}

#[handler]
async fn get_session_info(depot: &mut Depot) -> Json<serde_json::Value> {
    let session = depot.session_mut().expect("Session not found");
    let data = session.data();

    Json(serde_json::json!({
        "server": "rust",
        "sessionId": session.id(),
        "isNew": session.is_new(),
        "data": data
    }))
}

#[handler]
async fn set_data(req: &mut Request, depot: &mut Depot) -> Json<JsonResponse> {
    let session = depot.session_mut().expect("Session not found");

    let key = req
        .query::<String>("key")
        .unwrap_or_else(|| "testKey".to_string());
    let value = req
        .query::<String>("value")
        .unwrap_or_else(|| "testValue".to_string());

    session.set(&key, &value);
    session.set("lastModifiedBy", "rust");
    session.set("lastModifiedAt", chrono::Utc::now().to_rfc3339());

    Json(JsonResponse {
        action: Some("set"),
        key: Some(key),
        value: Some(serde_json::Value::String(value)),
        session_id: Some(session.id().to_string()),
        ..Default::default()
    })
}

#[handler]
async fn get_data(req: &mut Request, depot: &mut Depot) -> Json<JsonResponse> {
    let session = depot.session_mut().expect("Session not found");

    let key = req
        .query::<String>("key")
        .unwrap_or_else(|| "testKey".to_string());
    let value: Option<serde_json::Value> = session.get(&key);
    let last_modified_by: Option<String> = session.get("lastModifiedBy");

    Json(JsonResponse {
        action: Some("get"),
        key: Some(key),
        value: value.clone(),
        exists: Some(value.is_some()),
        session_id: Some(session.id().to_string()),
        last_modified_by,
        ..Default::default()
    })
}

#[handler]
async fn counter(depot: &mut Depot) -> Json<JsonResponse> {
    let session = depot.session_mut().expect("Session not found");

    let count: i32 = session.get("counter").unwrap_or(0);
    let new_count = count + 1;

    session.set("counter", new_count);
    session.set("lastModifiedBy", "rust");

    Json(JsonResponse {
        counter: Some(new_count),
        session_id: Some(session.id().to_string()),
        ..Default::default()
    })
}

#[handler]
async fn clear_session(depot: &mut Depot) -> Json<serde_json::Value> {
    let session = depot.session_mut().expect("Session not found");
    let session_id = session.id().to_string();

    session.destroy();

    Json(serde_json::json!({
        "server": "rust",
        "action": "clear",
        "previousSessionId": session_id
    }))
}

#[handler]
async fn cookie_info(depot: &mut Depot) -> Json<serde_json::Value> {
    let session = depot.session().expect("Session not found");
    let cookie = session.cookie();

    Json(serde_json::json!({
        "server": "rust",
        "sessionId": session.id(),
        "cookie": {
            "originalMaxAge": cookie.original_max_age,
            "maxAge": cookie.max_age(),
            "expires": cookie.expires,
            "httpOnly": cookie.http_only,
            "secure": cookie.secure,
            "path": cookie.path,
            "sameSite": cookie.same_site,
            "domain": cookie.domain
        }
    }))
}

#[handler]
async fn set_cookie_maxage(req: &mut Request, depot: &mut Depot) -> Json<serde_json::Value> {
    let session = depot.session_mut().expect("Session not found");

    let seconds: u64 = req
        .query::<u64>("seconds")
        .unwrap_or(3600);

    // Set the cookie max age dynamically
    session.set_cookie_max_age_secs(seconds);
    session.set("customMaxAgeSet", true);
    session.set("lastModifiedBy", "rust");

    let cookie = session.cookie();

    Json(serde_json::json!({
        "server": "rust",
        "action": "set-cookie-maxage",
        "maxAgeSecs": seconds,
        "newExpires": cookie.expires,
        "sessionId": session.id()
    }))
}

#[tokio::main]
async fn main() {
    // Set up logging
    tracing_subscriber::fmt::init();

    // Get Redis URL from environment or use default
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());

    println!("Connecting to Redis at {}", redis_url);

    // Create Redis store
    let store = RedisStore::from_url(&redis_url)
        .await
        .expect("Failed to connect to Redis");

    // Configure session - use same settings as your Node.js app for compatibility
    let secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| "keyboard cat".to_string());

    let config = SessionConfig::new(&secret)
        .with_cookie_name("connect.sid") // Must match Node.js config
        .with_prefix("sess:") // Must match connect-redis prefix (default)
        .with_max_age(86400) // 1 day in seconds
        .with_save_uninitialized(false)
        .with_rolling(false);

    // Create session handler
    let session_handler = ExpressSessionHandler::new(store, config);

    // Build router
    let router = Router::new()
        .push(Router::with_path("health").get(health))
        .push(
            Router::new()
                .hoop(session_handler)
                .get(index)
                .push(Router::with_path("session").get(get_session_info))
                .push(Router::with_path("set").get(set_data))
                .push(Router::with_path("get").get(get_data))
                .push(Router::with_path("counter").get(counter))
                .push(Router::with_path("clear").get(clear_session))
                .push(Router::with_path("cookie-info").get(cookie_info))
                .push(Router::with_path("set-cookie-maxage").get(set_cookie_maxage)),
        );

    // Get port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "5800".to_string());
    let addr = format!("127.0.0.1:{}", port);

    // Start server
    let acceptor = TcpListener::new(addr.clone()).bind().await;
    println!("Server running at http://{}", addr);
    println!();
    println!("Endpoints:");
    println!("  GET /health     - Health check (no session)");
    println!("  GET /           - View counter");
    println!("  GET /session    - Get session info");
    println!("  GET /set        - Set data (key=x&value=y)");
    println!("  GET /get        - Get data (key=x)");
    println!("  GET /counter    - Increment counter");
    println!("  GET /clear      - Clear session");
    println!();
    println!("To test compatibility with Node.js:");
    println!("1. Set up a Node.js app with express-session and connect-redis");
    println!(
        "2. Use the same secret ('{}') and cookie name ('connect.sid')",
        secret
    );
    println!("3. Sessions will be shared between both applications!");

    Server::new(acceptor).serve(router).await;
}

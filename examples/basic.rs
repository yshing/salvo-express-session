//! Basic example using in-memory session store

use salvo::prelude::*;
use salvo_express_session::{ExpressSessionHandler, MemoryStore, SessionConfig, SessionDepotExt};

#[handler]
async fn index(depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");
    
    // Get current view count
    let views: i32 = session.get("views").unwrap_or(0);
    
    // Increment view count
    session.set("views", views + 1);
    
    format!(
        "Hello! You have viewed this page {} time(s).\nSession ID: {}",
        views + 1,
        session.id()
    )
}

#[handler]
async fn get_user(depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");
    
    match session.get::<String>("user") {
        Some(user) => format!("Logged in as: {}", user),
        None => "Not logged in".to_string(),
    }
}

#[handler]
async fn set_user(req: &mut Request, depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");
    
    // Get username from query parameter
    let username = req.query::<String>("name").unwrap_or_else(|| "anonymous".to_string());
    
    session.set("user", &username);
    
    format!("User set to: {}", username)
}

#[handler]
async fn logout(depot: &mut Depot) -> &'static str {
    let session = depot.session_mut().expect("Session not found");
    
    // Clear all session data
    session.clear();
    
    "Logged out successfully"
}

#[handler]
async fn destroy_session(depot: &mut Depot) -> &'static str {
    let session = depot.session_mut().expect("Session not found");
    
    // Mark session for destruction
    session.destroy();
    
    "Session destroyed"
}

#[handler]
async fn regenerate_session(depot: &mut Depot) -> String {
    let session = depot.session_mut().expect("Session not found");
    
    let old_id = session.id().to_string();
    
    // Mark session for regeneration (new ID, keep data)
    session.regenerate();
    
    format!("Session regenerated. Old ID: {}", old_id)
}

#[tokio::main]
async fn main() {
    // Set up logging
    tracing_subscriber::fmt::init();

    // Create memory store
    let store = MemoryStore::new();

    // Configure session
    let config = SessionConfig::new("your-super-secret-key-change-in-production")
        .with_cookie_name("connect.sid") // Same as express-session default
        .with_max_age(3600) // 1 hour
        .with_save_uninitialized(false)
        .with_rolling(true); // Reset expiry on each request

    // Create session handler
    let session_handler = ExpressSessionHandler::new(store, config);

    // Build router
    let router = Router::new()
        .hoop(session_handler)
        .get(index)
        .push(Router::with_path("user").get(get_user))
        .push(Router::with_path("login").get(set_user))
        .push(Router::with_path("logout").get(logout))
        .push(Router::with_path("destroy").get(destroy_session))
        .push(Router::with_path("regenerate").get(regenerate_session));

    // Start server
    let acceptor = TcpListener::new("127.0.0.1:5800").bind().await;
    println!("Server running at http://127.0.0.1:5800");
    println!("Try these endpoints:");
    println!("  GET /           - View counter");
    println!("  GET /user       - Get current user");
    println!("  GET /login?name=alice - Set user");
    println!("  GET /logout     - Clear session");
    println!("  GET /destroy    - Destroy session");
    println!("  GET /regenerate - Regenerate session ID");
    
    Server::new(acceptor).serve(router).await;
}

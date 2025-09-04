use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod assets;
mod bounded_set;
mod db;
mod db_models;
mod handlers;
mod models;
mod state;
mod utils;

// The central, shared application state. We use an Arc to allow multiple
// worker threads to share the state, and a DashMap for thread-safe
// concurrent access to the session data.
pub use state::State;

pub type AppState = Arc<Mutex<State>>;

/// Main function to set up and run the `actix-web` server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up a subscriber to log messages to the console, forcing them to be unbuffered.
    // This explicitly writes to stdout and should resolve the issue.
    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .with_target(true)
        .init();

    // Create the shared application state.
    let updates = state::Updates::new();
    let app_state: AppState = Arc::new(Mutex::new(
        State::new(updates).await.expect("Failed to create state"),
    ));

    info!("Starting server at http://127.0.0.1:8080");

    // Start the HTTP server.
    HttpServer::new(move || {
        // Configure CORS to allow cross-origin requests from any origin.
        let cors = Cors::permissive();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .service(handlers::create_session)
            .service(handlers::stop_session)
            .service(handlers::post_location)
            .service(handlers::stream)
            .service(handlers::login)
            .service(assets::assets("", "index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

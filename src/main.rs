use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;
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

/// Command line configuration
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// IP address to bind the server to
    #[arg(long, default_value = "127.0.0.1")]
    address: String,

    /// Port to bind the server to
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Path to the password file for user authentication
    #[arg(long, default_value = "ducktracker.passwd")]
    password_file: PathBuf,

    /// Path to the SQLite database file
    #[arg(long, default_value = "ducktracker.db")]
    database_file: PathBuf,

    /// Default location tag to use for new locations
    #[arg(long, default_value = "duck")]
    default_location_tag: String,

    /// Scheme used when sharing links to the service
    #[arg(long, default_value = "http")]
    scheme: String,

    /// Server name used when sharing links to the service (but usually the Origin of Host header is sufficient)
    #[arg(long)]
    server_name: Option<String>,
}

async fn real_main() -> anyhow::Result<()> {
    // Set up a subscriber to log messages to the console, forcing them to be unbuffered.
    // This explicitly writes to stdout and should resolve the issue.
    FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_line_number(true)
        .with_target(true)
        .init();

    info!("Initializing");

    // Parse command line arguments
    let config = Config::parse();

    info!("Configuration: {:?}", config); // Log the parsed configuration

    // Create the shared application state.
    let updates = state::Updates::new();
    // NOTE: The `State::new` function in `state.rs` will need to be updated
    // to accept these new configuration parameters.
    let app_state: AppState = Arc::new(Mutex::new(
        State::new(
            updates,
            &config.database_file,
            &config.password_file,
            &config.default_location_tag,
            &config.scheme,
            config.server_name.as_ref().map(|s| s.as_str()),
        )
        .await?,
    ));

    info!("Starting server on {}:{}", config.address, config.port);

    // Start the HTTP server.
    Ok(HttpServer::new(move || {
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
    .bind((config.address.as_str(), config.port))? // Use parsed address and port
    .run()
    .await?)
}

#[actix_web::main]
async fn main() -> std::process::ExitCode {
    match real_main().await {
        Ok(()) => std::process::ExitCode::from(0),
        Err(err) => {
            error!("{}", err.to_string());
            std::process::ExitCode::from(10)
        }
    }
}

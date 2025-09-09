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

    /// The public tag that's offered to clients automatically
    #[arg(long, default_value = "duck")]
    default_public_tag: String,

    /// Scheme used when sharing links to the service
    #[arg(long, default_value = "http")]
    scheme: String,

    /// Server name used when sharing links to the service (but usually the Origin or Host header is sufficient)
    #[arg(long)]
    server_name: Option<String>,

    /// Default tag to assign to shares that don't have any share_id
    #[arg(long, default_value = "duck")]
    default_tag: String,

    /// Maximum number of points a share can have. Mostly for to client peformance purposes.
    #[arg(long, default_value = "1000")]
    max_points: usize,

    /// Heart beat interval; changes will be reported to clients latest by this delay, and empty heartbeat messages are sent with this interval
    #[arg(long, default_value = "1000ms")]
    update_interval: humantime::Duration,
}

async fn real_main() -> anyhow::Result<()> {
    FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_line_number(true)
        .with_target(true)
        .init();

    info!("Initializing");

    let config = Config::parse();

    info!("Configuration: {config:?}"); // Log the parsed configuration

    let updates = state::Updates::new(config.update_interval.into()).await;
    let app_state: AppState = Arc::new(Mutex::new(
        State::new(
            updates,
            &config.database_file,
            &config.password_file,
            &config.default_public_tag,
            &config.scheme,
            config.server_name.as_deref(),
            config.max_points,
            config.update_interval.into(),
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
            error!("{err}");
            std::process::ExitCode::from(10)
        }
    }
}

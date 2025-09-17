use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use anyhow::Context;
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::Mutex;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod assets;
mod bounded_set;
mod box_coords;
mod db;
mod db_models;
mod handlers;
mod models;
mod prometheus; // Add this module
mod session_counter;
mod state;
mod utils;
mod version;

// The central, shared application state. We use an Arc to allow multiple
// worker threads to share the state, and a DashMap for thread-safe
// concurrent access to the session data.
pub use state::State;

pub use box_coords::BoxCoords;

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

    /// Maximum number of points a share can have. Mostly for to client peformance purposes. Client cannot exceed this.
    #[arg(long, default_value = "1000")]
    max_points: usize,

    /// Default number of points a share can have. Mostly for to client peformance purposes.
    #[arg(long, default_value = "200")]
    default_points: usize,

    /// Default data expiration time (can be overridden with expire:10s)
    #[arg(long)]
    default_expire_duration: Option<humantime::Duration>,

    /// Heart beat interval; changes will be reported to clients latest by this delay, and empty heartbeat messages are sent with this interval
    #[arg(long, default_value = "1000ms")]
    update_interval: humantime::Duration,

    /// Bounding box for wrapping coordinates, format: "lat1,lng1,lat2,lng2"
    #[arg(long)]
    box_coords: Option<String>,

    /// Username for Prometheus metrics endpoint
    #[arg(long)]
    prometheus_user: Option<String>,

    /// Password for Prometheus metrics endpoint
    #[arg(long)]
    prometheus_password: Option<String>,
}

async fn real_main() -> anyhow::Result<()> {
    FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,turso=error")),
        )
        .with_line_number(true)
        .with_target(true)
        .init();

    info!("ducktracker {}", crate::version::VERSION);

    let config = Config::parse();

    info!("Configuration: {config:?}"); // Log the parsed configuration

    // Parse box_coords if provided
    let parsed_box_coords: Option<BoxCoords> = if let Some(box_str) = &config.box_coords {
        Some(
            box_str
                .parse::<BoxCoords>()
                .context("Failed to parse --box coordinates")?,
        )
    } else {
        None
    };

    // Shared data for metrics
    let start_time = std::time::Instant::now();
    let sse_counter = Arc::new(AtomicU64::new(0));

    let default_max_point_age = config
        .default_expire_duration
        .map(|x| {
            let x: std::time::Duration = x.into();
            chrono::TimeDelta::from_std(x)
        })
        .transpose()?;

    let updates = state::Updates::new(config.update_interval.into()).await;
    let app_state: AppState = State::new(
        updates,
        &config.database_file,
        &config.password_file,
        &config.default_public_tag,
        &config.scheme,
        config.server_name.as_deref(),
        config.max_points,
        config.default_points,
        default_max_point_age,
        config.update_interval.into(),
        parsed_box_coords, // PASS THE PARSED BOX COORDINATES
        config.prometheus_user,
        config.prometheus_password,
    )
    .await?;

    info!("Starting server on {}:{}", config.address, config.port);

    // Start the HTTP server.
    Ok(HttpServer::new(move || {
        // Configure CORS to allow cross-origin requests from any origin.
        let cors = Cors::permissive();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(actix_web::middleware::Compress::default())
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(sse_counter.clone()))
            .app_data(web::Data::new(start_time))
            .service(handlers::create_session)
            .service(handlers::stop_session)
            .service(handlers::post_location)
            .service(handlers::stream)
            .service(handlers::login)
            .service(handlers::prometheus_metrics)
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

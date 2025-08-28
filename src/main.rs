// File: src/main.rs
// This is the main application logic for the Rust backend.
// This content should be placed in a file named 'src/main.rs'.

use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use hex;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

// The central, shared application state. We use an Arc to allow multiple
// worker threads to share the state, and a DashMap for thread-safe
// concurrent access to the session data.
type AppState = Arc<DashMap<String, Session>>;

// ========================
// Data Structures
// ========================

/// Represents the current location data for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Location {
    lat: f64,
    lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    brg: Option<f64>,
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
struct Session {
    session_id: String,
    password_hash: String,
    share_link_token: String,
    last_location: Option<Location>,
    expires_at: DateTime<Utc>,
}

// ========================
// API Request and Response Models
// ========================

/// Request body for the /api/create endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct CreateRequest {
    session_id: String,
    password: Option<String>,
    duration: u64, // In seconds
}

/// Response body for the /api/create endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct CreateResponse {
    status: String,
    session_id: String,
    share_link: String,
}

/// Request body for the /api/post endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct PostRequest {
    session_id: String,
    password: Option<String>,
    #[serde(rename = "lat")]
    latitude: f64,
    #[serde(rename = "lon")]
    longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "acc")]
    accuracy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "alt")]
    altitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "spd")]
    speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "brg")]
    bearing: Option<f64>,
}

/// Request body for the /api/fetch endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct FetchRequest {
    session_id: String,
    #[serde(rename = "sharelink")]
    share_link_token: String,
}

/// Response body for the /api/fetch endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct FetchResponse {
    status: String,
    lat: Option<f64>,
    lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    brg: Option<f64>,
    expires_in: i64, // Seconds remaining until expiration
}

// ========================
// API Endpoints (Handlers)
// ========================

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/create")]
async fn create_session(
    data: web::Json<CreateRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let session_id = data.session_id.clone();

    // Check if a session with this ID already exists.
    if state.contains_key(&session_id) {
        return HttpResponse::BadRequest().body("Session ID already exists.");
    }

    // Hash the password for secure storage.
    let password = data.password.clone().unwrap_or_else(|| {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    });
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let password_hash = hex::encode(hasher.finalize());

    // Generate a unique share link token.
    let share_link_token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    // Calculate the expiration time.
    let expires_at = Utc::now() + Duration::seconds(data.duration as i64);

    // Create a new session and store it in the DashMap.
    let new_session = Session {
        session_id: session_id.clone(),
        password_hash,
        share_link_token: share_link_token.clone(),
        last_location: None, // No location data initially
        expires_at,
    };
    state.insert(session_id.clone(), new_session);

    // Construct the response.
    let response = CreateResponse {
        status: "ok".to_string(),
        session_id: session_id.clone(),
        share_link: share_link_token,
    };

    HttpResponse::Ok().json(response)
}

/// Handler for the `/api/post` endpoint.
///
/// This function updates the location data for an existing session.
#[post("/api/post")]
async fn post_location(data: web::Json<PostRequest>, state: web::Data<AppState>) -> impl Responder {
    // Find and get a mutable reference to the session from the DashMap.
    let mut session = match state.get_mut(&data.session_id) {
        Some(s) => s,
        None => return HttpResponse::NotFound().body("Session not found."),
    };

    // Hash the provided password and compare it to the stored hash.
    let mut hasher = Sha256::new();
    let provided_password = data.password.clone().unwrap_or_default();
    hasher.update(provided_password.as_bytes());
    let provided_password_hash = hex::encode(hasher.finalize());

    if provided_password_hash != session.password_hash {
        return HttpResponse::Unauthorized().body("Invalid password.");
    }

    // Create a new Location struct with the provided data.
    let new_location = Location {
        lat: data.latitude,
        lon: data.longitude,
        acc: data.accuracy,
        alt: data.altitude,
        spd: data.speed,
        brg: data.bearing,
    };

    // Update the last_location field of the session.
    session.last_location = Some(new_location);

    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

/// Handler for the `/api/fetch` endpoint.
///
/// This function retrieves the latest location data for a session.
#[get("/api/fetch")]
async fn fetch_location(
    data: web::Query<FetchRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    // Look up the session in the DashMap using the share link token.
    let session = state
        .iter()
        .find(|entry| entry.share_link_token == data.share_link_token);

    let session = match session {
        Some(s) => s,
        None => return HttpResponse::NotFound().body("Share link invalid or session not found."),
    };

    // Check if the session has expired.
    let now = Utc::now();
    if session.expires_at < now {
        return HttpResponse::Gone().body("Session has expired.");
    }

    // Get the location data from the session.
    let last_location = session.last_location.clone();

    // Calculate time remaining until expiration.
    let time_remaining = session.expires_at.signed_duration_since(now).num_seconds();

    // Construct the response.
    let response = match last_location {
        Some(loc) => FetchResponse {
            status: "ok".to_string(),
            lat: Some(loc.lat),
            lon: Some(loc.lon),
            acc: loc.acc,
            alt: loc.alt,
            spd: loc.spd,
            brg: loc.brg,
            expires_in: time_remaining,
        },
        None => FetchResponse {
            status: "ok".to_string(),
            lat: None,
            lon: None,
            acc: None,
            alt: None,
            spd: None,
            brg: None,
            expires_in: time_remaining,
        },
    };

    HttpResponse::Ok().json(response)
}

/// Main function to set up and run the `actix-web` server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create the shared application state.
    let app_state: AppState = Arc::new(DashMap::new());

    println!("Starting server at http://127.0.0.1:8080");

    // Start the HTTP server.
    HttpServer::new(move || {
        // Configure CORS to allow cross-origin requests from any origin.
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(web::Data::from(app_state.clone()))
            .service(create_session)
            .service(post_location)
            .service(fetch_location)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

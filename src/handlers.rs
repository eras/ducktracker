use crate::AppState;
use crate::models::{
    CreateRequest, CreateResponse, FetchRequest, FetchResponse, Location, PostRequest, Session,
};
use actix_web::{HttpResponse, Responder, get, post, web};
use chrono::{Duration, Utc};
use hex;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/create")]
pub async fn create_session(
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
pub async fn post_location(
    data: web::Json<PostRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
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

    HttpResponse::Ok().json(json!({"status": "ok"}))
}

/// Handler for the `/api/fetch` endpoint.
///
/// This function retrieves the latest location data for a session.
#[get("/api/fetch")]
pub async fn fetch_location(
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

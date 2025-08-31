use std::collections::HashMap;

use crate::AppState;
use crate::models::{
    self, CreateRequest, CreateResponse, FetchRequest, FetchResponse, Location, PostRequest,
    PostResponse, Session, ShareType, TimeUsec,
};
use crate::state;
use actix_web::{HttpResponse, Responder, get, post, web};
use chrono::{Duration, Utc};
use hex;
use log::info;
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use sha2::{Digest, Sha256};
use tokio_stream::StreamExt; // For stream combinators like .next()

/// Placeholder authentication function.
/// This function should be replaced with real authentication logic in the future.
pub fn check_authentication(
    _user: &Option<String>,
    _password: &Option<String>,
    _session: &Session,
) -> bool {
    // For now, we will simply pass the authentication check.
    // In a real-world scenario, you would hash the provided password
    // and compare it to the stored hash in the session object.
    //
    // let mut hasher = Sha256::new();
    // let provided_password = password.clone().unwrap_or_default();
    // hasher.update(provided_password.as_bytes());
    // let provided_password_hash = hex::encode(hasher.finalize());
    // provided_password_hash == session.password_hash
    true
}

pub fn generate_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/create.php")]
pub async fn create_session(
    data: web::Form<CreateRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut state = state.lock().await;
    let session_id = models::SessionId(generate_id());

    // Check if a session with this ID already exists.
    if state.sessions.contains_key(&session_id) {
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
    let share_id = models::ShareId(data.share_id.clone().unwrap_or_else(|| generate_id()));
    let share_link = format!("http://127.0.0.1/{share_id}");

    // Calculate the expiration time.
    let expires_at = Utc::now() + Duration::seconds(data.duration as i64);

    // Create a new session and store it in the DashMap.
    let new_session = Session {
        session_id: session_id.clone(),
        password_hash,
        share_id: share_id.clone(),
        locations: Vec::new(),
        expires_at,
        fetch_id: state.generate_fetch_id(),
    };
    state.sessions.insert(session_id.clone(), new_session);

    // Construct the response.
    let response = CreateResponse {
        status: "OK".to_string(),
        session_id: session_id.clone(),
        share_link: share_link,
        share_id: share_id.clone(),
    };

    // Create an HTTP response with a Content-Type of "text/plain".
    // This tells the client how to interpret the response body.
    HttpResponse::Ok()
        .content_type("text/plain")
        .body(response.to_client())
}

/// Handler for the `/api/post` endpoint.
///
/// This function updates the location data for an existing session.
#[post("/api/post.php")]
pub async fn post_location(
    data: web::Form<PostRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut state = state.lock().await;
    match state.add_location(&data).await {
        Err(state::Error::NoSuchSession) => HttpResponse::NotFound().body("Session not found."),
        Err(state::Error::SessionExpired) => HttpResponse::Gone().body("Session has expired."),
        Ok(()) => {
            let response = PostResponse {
                public_url: "http://localhost".to_string(), // TODO
                target_ids: Vec::new(),
            };

            HttpResponse::Ok()
                .content_type("text/plain")
                .body(response.to_client())
        }
    }
}

/// Handler for the `/api/fetch` endpoint.
///
/// This function retrieves the latest location data for a session.
#[get("/api/fetch.php")]
pub async fn fetch_location(
    data: web::Query<FetchRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let state = state.lock().await;

    // Look up the session in the DashMap using the share link token.
    let session = state
        .sessions
        .iter()
        .find(|entry| entry.share_id == data.share_id);

    let session = match session {
        Some(s) => s,
        None => return HttpResponse::NotFound().body("Share link invalid or session not found."),
    };

    // Check if the session has expired.
    let now = Utc::now();
    if session.expires_at < now {
        return HttpResponse::Gone().body("Session has expired.");
    }

    // Calculate time remaining until expiration.
    let time_remaining = session.expires_at.signed_duration_since(now).num_seconds();

    // Construct the response.
    let points: Vec<Location> = session.locations.clone();

    let mut all_points = HashMap::new();
    all_points.insert(models::FetchId(0u64), points);

    let response = FetchResponse {
        type_: ShareType::Group,
        expire: 0.0f64,
        server_time: TimeUsec(std::time::SystemTime::now()),
        interval: 0u64,
        points: all_points,
    };

    HttpResponse::Ok().json(response)
}

#[actix_web::get("/api/stream")]
async fn stream(state: web::Data<AppState>) -> impl Responder {
    let state = state.lock().await;
    let updates = state.updates.updates(&state).await;
    let events = futures_util::StreamExt::map(updates, |update| {
        let update = update.expect("woot, there should have been an update..");
        let json_data = serde_json::to_string(&update).expect("Failed to encode Update to JSON");
        Ok::<_, std::convert::Infallible>(actix_web_lab::sse::Event::Data(
            actix_web_lab::sse::Data::new(json_data),
        ))
    });

    actix_web_lab::sse::Sse::from_stream(events).with_keep_alive(std::time::Duration::from_secs(5))
}

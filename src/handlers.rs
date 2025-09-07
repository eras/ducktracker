use crate::AppState;
use crate::models::{
    self, CreateRequest, CreateResponse, LoginResponse, PostRequest, PostResponse,
};
use crate::state;
use crate::utils;
use actix_web::{HttpResponse, Responder, post, web};
use chrono::{Duration, Utc};

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/create.php")]
pub async fn create_session(
    data: web::Form<CreateRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut state = state.lock().await;

    if !state.authenticate(
        &data.user.clone().unwrap_or("".to_string()),
        &data.password.clone().unwrap_or("".to_string()),
    ) {
        return HttpResponse::Unauthorized().finish();
    }

    let tags_aux = models::TagsAux::from_share_id(&data.share_id);

    // Calculate the expiration time.
    let expires_at = Utc::now() + Duration::seconds(data.duration as i64);

    let session_id = state.add_session(expires_at, tags_aux).await;

    // Generate a unique share link token.
    let share_id = models::ShareId(
        data.share_id
            .clone()
            .unwrap_or_else(|| utils::generate_id()),
    );
    let share_link = format!("http://127.0.0.1/{share_id}");

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

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/stop.php")]
pub async fn stop_session(
    data: web::Form<models::StopRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut state = state.lock().await;
    state.remove_session(&data.session_id).await;
    let response = models::StopResponse {};
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

#[actix_web::post("/api/login")]
pub async fn login(
    data: web::Json<models::LoginRequest>,
    app_state: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let mut state = app_state.lock().await;

    if let Some(token) = state.create_token(&data.username, &data.password) {
        Ok(web::Json(LoginResponse { token }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Invalid credentials."))
    }
}

#[actix_web::get("/api/stream")]
pub async fn stream(
    data: web::Query<models::StreamRequest>,
    app_state: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let state = app_state.lock().await;
    if !state.check_token(&data.token) {
        //return HttpResponse::Unauthorized().body("Invalid credentials.");
        return Err(actix_web::error::ErrorUnauthorized("Invalid credentials."));
    }
    let tags = if data.tags.0.len() == 0 {
        state.get_public_tags().0.clone()
    } else {
        data.tags.0.iter().map(|x| x.clone()).collect()
    };
    let updates = state
        .updates
        .updates(&state, tags.iter().map(|x| x.clone()).collect())
        .await;
    let events = futures_util::StreamExt::map(updates, |update| {
        let (_update_context, update) = update.expect("woot, there should have been an update..");
        let json_data = serde_json::to_string(&update).expect("Failed to encode Update to JSON");
        Ok::<_, std::convert::Infallible>(actix_web_lab::sse::Event::Data(
            actix_web_lab::sse::Data::new(json_data),
        ))
    });

    Ok(actix_web_lab::sse::Sse::from_stream(events)
        .with_keep_alive(std::time::Duration::from_secs(5)))
}

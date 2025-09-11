use crate::AppState;
use crate::models::{
    self, CreateRequest, CreateResponse, LoginResponse, PostRequest, PostResponse,
};
use crate::state;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use chrono::{Duration, Utc};
use log::error;
use std::pin::Pin;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

/// Handler for the `/api/create` endpoint.
///
/// This function creates a new tracking session and returns a share link.
#[post("/api/create.php")]
pub async fn create_session(
    data: web::Form<CreateRequest>,
    state: web::Data<AppState>,
    request: HttpRequest,
) -> impl Responder {
    let mut state = state.lock().await;

    if !state.authenticate(
        &data.user.clone().unwrap_or("".to_string()),
        &data.password.clone().unwrap_or("".to_string()),
    ) {
        return HttpResponse::Unauthorized().finish();
    }

    let tags_aux = match models::TagsAux::from_share_id(&data.share_id) {
        Ok(x) => x,
        Err(err) => {
            return HttpResponse::BadRequest()
                .body(format!("The format of share id is not permitted: {err}"));
        }
    };

    // Calculate the expiration time.
    let expires_at = Utc::now() + Duration::seconds(data.duration as i64);

    let share_id = models::ShareId(
        tags_aux
            .0
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );

    let session_id = state.add_session(expires_at, tags_aux).await;

    let base_url = if let Some(server_name) = state.server_name.clone() {
        format!("{}://{}", state.http_scheme, server_name)
    } else {
        request
            .headers()
            .get(actix_web::http::header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| {
                let host = request
                    .headers()
                    .get(actix_web::http::header::HOST)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("127.0.0.1");
                format!("{}://{}", state.http_scheme, host)
            })
    };

    let share_link = format!("{base_url}/#{share_id}");

    // Construct the response.
    let response = CreateResponse {
        status: "OK".to_string(),
        session_id: session_id.clone(),
        share_link,
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
        Ok(web::Json(LoginResponse {
            token,
            version: crate::version::VERSION.to_string(),
        }))
    } else {
        Err(actix_web::error::ErrorUnauthorized("Invalid credentials."))
    }
}

async fn coalesce_stream(
    events: Box<dyn futures_util::stream::Stream<Item = state::UpdateBroadcast>>,
    window: std::time::Duration,
) -> Pin<Box<dyn futures_util::stream::Stream<Item = state::UpdateBroadcast>>> {
    // 1. Convert the input Box<dyn Stream> into a Pin<Box<dyn Stream>>.
    //    This is necessary to safely poll the stream using `.next().await`.
    let mut pinned_events = Box::<dyn futures_util::Stream<Item = Result<_, _>>>::into_pin(events);

    use futures_util::StreamExt;

    // 2. Use the `stream!` macro to define your custom filtering logic.
    #[rustfmt::skip]
    let filtered_stream = async_stream::stream! {
        let mut t_prev: Option<f64> = None;
	let mut accum: Option<state::UpdateWithContext> = None;
	let mut first = true; // first event goes right through
	let collect_window_seconds = window.as_secs_f64();
        while let Some(event) = pinned_events.as_mut().next().await {
	    match &event {
		Ok(ok_event) => {
		    let do_accumulate = if let Some(t_prev) = t_prev {
			(ok_event.meta.server_time.epoch() - t_prev) < collect_window_seconds
		    } else {
			true
		    };
		    if first {
			accum = Some(ok_event.clone())
		    }
		    if !first && do_accumulate {
			accum = Some(match accum {
			    None => {
				t_prev = Some(ok_event.meta.server_time.epoch());
				ok_event.clone()
			    }
			    Some(mut prev_accum) => {
				prev_accum.ingest(ok_event.clone());
				prev_accum
			    }
			})
		    } else {
			if let Some(mut prev_accum) = accum {
			    // Stamp with the current server time
			    prev_accum.meta.server_time = models::TimeUsec(std::time::SystemTime::now());
			    yield Ok(prev_accum);
			}
			t_prev = Some(ok_event.meta.server_time.epoch());
			accum = Some(ok_event.clone())
		    }
		    first = false;
		}
		Err(err) => {
		    // These happen when we receiver lags too far behind the source. Seems like a very
		    // severe overload event in the server. Let's just forward these for now.

		    // Proper response would probably be disconnecting the SSE session and let the
		    // client restart it.

		    match err {
			BroadcastStreamRecvError::Lagged(lagged_by) => {
			    error!("Client lagging by {lagged_by} messages")
			}
		    }

		    if let Some(mut prev_accum) = accum {
			prev_accum.meta.server_time = models::TimeUsec(std::time::SystemTime::now());
			yield Ok(prev_accum);
			accum = None;
			t_prev = None;
		    }
		    yield event;
		}
	    }
        }
    };

    // 3. Box and pin the concrete stream returned by the `stream!` macro.
    //    This ensures the output matches your function's return type signature.
    Box::pin(filtered_stream)
}

#[actix_web::get("/api/stream")]
pub async fn stream(
    data: web::Query<models::StreamRequest>,
    app_state: web::Data<AppState>,
) -> actix_web::Result<impl Responder> {
    let state = app_state.lock().await;
    if !state.check_token(&data.token) {
        return Err(actix_web::error::ErrorUnauthorized("Invalid credentials."));
    }
    let tags = if data.tags.0.is_empty() {
        state.get_public_tags().0.clone()
    } else {
        data.tags.0.iter().cloned().collect()
    };
    let events = state
        .updates
        .updates(&state, tags.iter().cloned().collect())
        .await;
    let events = coalesce_stream(Box::new(events), state.update_interval).await;

    let events = futures_util::StreamExt::map(
        events,
        |update| -> anyhow::Result<actix_web_lab::sse::Event> {
            let update: models::Update = update?.into();
            let json_data = serde_json::to_string(&update)?;
            Ok(actix_web_lab::sse::Event::Data(
                actix_web_lab::sse::Data::new(json_data),
            ))
        },
    );

    Ok(actix_web_lab::sse::Sse::from_stream(events)
        .with_keep_alive(std::time::Duration::from_secs(5)))
}

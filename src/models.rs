use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the current location data for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brg: Option<f64>,
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub password_hash: String,
    pub share_link_token: String,
    pub last_location: Option<Location>,
    pub expires_at: DateTime<Utc>,
}

// ========================
// API Request and Response Models
// ========================

/// Request body for the /api/create endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRequest {
    #[serde(rename = "session_id")]
    pub session_id: String,
    pub password: Option<String>,
    pub duration: u64, // In seconds
}

/// Response body for the /api/create endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateResponse {
    pub status: String,
    pub session_id: String,
    pub share_link: String,
}

/// Request body for the /api/post endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct PostRequest {
    #[serde(rename = "session")]
    pub session_id: String,
    #[serde(rename = "usr")]
    pub user: Option<String>,
    #[serde(rename = "pwd")]
    pub password: Option<String>,
    #[serde(rename = "lat")]
    pub latitude: f64,
    #[serde(rename = "lon")]
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "acc")]
    pub accuracy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "alt")]
    pub altitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "spd")]
    pub speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "brg")]
    pub bearing: Option<f64>,
}

/// Request body for the /api/fetch endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest {
    pub session_id: String,
    #[serde(rename = "sharelink")]
    pub share_link_token: String,
}

/// Response body for the /api/fetch endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brg: Option<f64>,
    pub expires_in: i64, // Seconds remaining until expiration
}

use std::collections::HashMap;

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
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub password_hash: String,
    pub share_id: String,
    pub last_location: Option<Location>,
    pub expires_at: DateTime<Utc>,
}

// ========================
// API Request and Response Models
// ========================

/// Request body for the /api/create endpoint.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    #[serde(rename = "usr")]
    pub user: Option<String>,
    #[serde(rename = "pwd")]
    pub password: Option<String>,
    #[serde(rename = "mod")]
    pub mode: u64, // Something?
    #[serde(rename = "lid")]
    pub share_id: Option<String>, // Desired share id
    #[serde(rename = "dur")]
    pub duration: u64, // In seconds
    #[serde(rename = "int")]
    pub interval: u64, // In seconds
}

/// Response body for the /api/create endpoint.
#[derive(Debug)]
pub struct CreateResponse {
    pub status: String,
    pub session_id: String,
    pub share_link: String,
    pub share_id: String,
}

impl CreateResponse {
    pub fn to_client(&self) -> String {
        return format!(
            "{}\n{}\n{}\n{}\n",
            self.status, self.session_id, self.share_link, self.share_id
        );
    }
}

/// Request body for the /api/post endpoint.
#[derive(Debug, Deserialize)]
pub struct PostRequest {
    #[serde(rename = "sid")]
    pub session_id: String,
    pub prv: u64, // what is this?
    pub time: f64,
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
}

#[derive(Debug)]
pub struct PostResponse {
    pub public_url: String,
    pub target_ids: Vec<String>,
}

impl PostResponse {
    pub fn to_client(&self) -> String {
        format!("OK\n{}?{}\n", self.public_url, self.target_ids.join(","))
    }
}

/// Request body for the /api/fetch endpoint.
#[derive(Debug, Deserialize)]
pub struct FetchRequest {
    #[serde(rename = "id")]
    pub share_id: String,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
    pub time: f64,
    pub acc: Option<f64>,
    pub spd: Option<f64>,
    pub prv: Option<bool>,
}

impl Point {
    pub fn from_location(loc: &Location) -> Point {
        Point {
            lat: loc.lat,
            lon: loc.lon,
            time: 0.0f64,
            acc: loc.acc,
            spd: loc.spd,
            prv: Some(false),
        }
    }
}

impl Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // NOTE to update the number of elements, if the structure would ever change
        let mut state = serializer.serialize_seq(Some(8))?;
        use serde::ser::SerializeSeq;
        state.serialize_element(&self.lat)?;
        state.serialize_element(&self.lon)?;
        state.serialize_element(&self.time)?;
        state.serialize_element(&self.spd)?;
        state.serialize_element(&self.acc)?;
        match self.prv {
            None => state.serialize_element::<Option<i64>>(&None)?,
            Some(false) => state.serialize_element(&0)?,
            Some(true) => state.serialize_element(&1)?,
        }
        state.end()
    }
}

#[derive(Debug)]
pub enum ShareType {
    Alone,
    Group,
}

impl Serialize for ShareType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ShareType::Alone => serializer.serialize_i64(0),
            ShareType::Group => serializer.serialize_i64(1),
        }
    }
}

pub type TimeUsec = f64;
pub type Nick = String;

/// Response body for the /api/fetch endpoint.
#[derive(Debug, Serialize)]
pub struct FetchResponse {
    #[serde(rename = "type")]
    pub type_: ShareType, // Must be Group
    pub expire: f64,
    #[serde(rename = "serverTime")]
    pub server_time: TimeUsec,
    pub interval: u64,
    pub points: HashMap<Nick, Vec<Point>>,
}

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the current location data for a session.
#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spd: Option<f64>,
    #[serde(rename = "prv")]
    pub provider: u64, // location provider, seems to be 0 or 1, probably coarse vs fine
    pub time: f64,
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: SessionId,
    pub password_hash: String,
    pub share_id: ShareId,
    pub locations: Vec<Location>,
    pub expires_at: DateTime<Utc>,
    pub fetch_id: FetchId,
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
    pub session_id: SessionId,
    pub share_link: String,
    pub share_id: ShareId,
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
    pub session_id: SessionId,
    #[serde(rename = "prv")]
    pub provider: Option<u64>,
    pub time: f64,
    #[serde(rename = "lat")]
    pub latitude: f64,
    #[serde(rename = "lon")]
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "acc")]
    pub accuracy: Option<f64>,
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

impl Serialize for Location {
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
        state.serialize_element(&self.provider)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct TimeUsec(pub std::time::SystemTime);

// Given to each new publish session
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SessionId(pub String);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

// Useless?
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ShareId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Tag(pub String);

impl std::str::FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

impl std::fmt::Display for ShareId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

// Id used when providing data back to clients
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct FetchId(pub u64);

impl Serialize for TimeUsec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let epoch = self
            .0
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Expected unix time to be available")
            .as_secs_f64();

        serializer.serialize_u64((epoch * 1000000.0) as u64)
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Update {
    #[serde(rename = "serverTime")]
    pub server_time: TimeUsec,
    pub interval: u64,
    pub changes: Vec<UpdateChange>,
}

#[derive(Debug, Serialize, Clone)]
pub enum UpdateChange {
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "add")]
    Add {
        points: HashMap<FetchId, Vec<Location>>,
    },
}

/// Request body for the /api/stream endpoint.
#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    pub tags: crate::utils::CommaSeparatedVec<Tag>,
}

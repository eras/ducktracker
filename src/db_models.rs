// src/db_models.rs

use crate::models::{self, SessionId, TagsAux};
use crate::state;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a session as stored in the database.
/// Only stores necessary fields for persistence, not volatile location data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
    pub fetch_id: models::FetchId,
    pub tags: TagsAux,
}

impl From<&state::Session> for DbSession {
    /// Converts a `state::Session` reference into a `DbSession`.
    fn from(session: &state::Session) -> Self {
        Self {
            session_id: session.session_id.clone(),
            expires_at: session.expires_at,
            fetch_id: session.fetch_id.clone(),
            tags: session.tags.clone(),
        }
    }
}

impl From<DbSession> for state::Session {
    /// Converts a `DbSession` into a `state::Session`.
    /// Note: Location data is not persisted, so `locations` will be empty.
    fn from(db_session: DbSession) -> Self {
        state::Session {
            session_id: db_session.session_id,
            locations: Vec::new(), // Location data is not persisted
            expires_at: db_session.expires_at,
            fetch_id: db_session.fetch_id,
            tags: db_session.tags,
        }
    }
}

// src/db_models.rs

use std::collections::VecDeque;

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
    pub max_points: usize,
}

impl From<&state::Session> for DbSession {
    /// Converts a `state::Session` reference into a `DbSession`.
    fn from(session: &state::Session) -> Self {
        Self {
            session_id: session.session_id().clone(),
            expires_at: session.expires_at(),
            fetch_id: session.fetch_id().clone(),
            tags: session.tags().clone(),
            max_points: session.max_points(),
        }
    }
}

impl From<DbSession> for state::Session {
    /// Converts a `DbSession` into a `state::Session`.
    /// Note: Location data is not persisted, so `locations` will be empty.
    fn from(db_session: DbSession) -> Self {
        state::SessionBuilder::new()
            .with_session_id(db_session.session_id)
            .with_locations(VecDeque::new()) // Location data is not persisted
            .with_expires_at(db_session.expires_at)
            .with_fetch_id(db_session.fetch_id)
            .with_tags(db_session.tags)
            .with_max_points(db_session.max_points)
            .build()
    }
}

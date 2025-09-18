// src/db_models.rs

use std::collections::VecDeque;

use crate::models::{self, SessionId, TagsAux};
use crate::state;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

/// Represents a session as stored in the database.
/// Only stores necessary fields for no_stopence, not volatile location data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbSession {
    pub session_id: SessionId,
    pub expires_at: DateTime<Utc>,
    pub fetch_id: models::FetchId,
    pub tags: TagsAux,
    pub max_points: usize,
    pub max_point_age: Option<TimeDelta>,
    pub reject_data: bool,
    pub no_stop: bool,
    pub log: Option<models::Log>,
    pub name: Option<String>,
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
            max_point_age: session.max_point_age(),
            reject_data: session.reject_data(),
            no_stop: session.no_stop(),
            log: session.log().clone(),
            name: session.name().clone(),
        }
    }
}

impl From<DbSession> for state::Session {
    /// Converts a `DbSession` into a `state::Session`.
    /// Note: Location data is not no_stoped, so `locations` will be empty.
    fn from(db_session: DbSession) -> Self {
        state::SessionBuilder::new()
            .with_session_id(db_session.session_id)
            .with_locations(VecDeque::new()) // Location data is not no_stoped
            .with_expires_at(db_session.expires_at)
            .with_fetch_id(db_session.fetch_id)
            .with_tags(db_session.tags)
            .with_max_points(db_session.max_points)
            .with_max_point_age(db_session.max_point_age)
            .with_reject_data(db_session.reject_data)
            .with_no_stop(db_session.no_stop)
            .with_log(db_session.log)
            .with_name(db_session.name)
            .build()
    }
}

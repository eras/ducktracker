// src/db.rs

use crate::db_models::DbSession;
use crate::models::{FetchId, SessionId, Tags};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde_json;
use std::sync::Arc;
use turso::{Builder, Database, Row, Value};

/// Client for interacting with the Turso (SQLite) database.
pub struct DbClient {
    client: Arc<Database>,
}

impl DbClient {
    /// Creates a new `DbClient` and initializes the database schema.
    pub async fn new() -> Result<Self> {
        let client = DbClient {
            client: Arc::new(Builder::new_local("ducktracker.db").build().await?),
        };
        client.init_db().await?;
        Ok(client)
    }

    /// Initializes the `sessions` table if it doesn't already exist.
    async fn init_db(&self) -> Result<()> {
        let conn = self.client.connect()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                expires_at TEXT NOT NULL,
                fetch_id INTEGER NOT NULL,
                tags TEXT NOT NULL
            )",
            (),
        )
        .await?;
        Ok(())
    }

    /// Inserts a `DbSession` into the database.
    pub async fn insert_session(&self, session: &DbSession) -> Result<()> {
        let conn = self.client.connect()?;
        // Serialize Tags into a JSON string for storage
        let tags_json = serde_json::to_string(&session.tags)?;
        conn.execute(
            "INSERT INTO sessions (session_id, expires_at, fetch_id, tags) VALUES (?, ?, ?, ?)",
            (
                session.session_id.0.clone(),
                session.expires_at.to_rfc3339(), // Store DateTime as ISO 8601 string
                session.fetch_id.0,
                tags_json,
            ),
        )
        .await?;
        Ok(())
    }

    /// Retrieves all `DbSession`s from the database.
    pub async fn get_all_sessions(&self) -> Result<Vec<DbSession>> {
        let conn = self.client.connect()?;
        let mut results = conn
            .query(
                "SELECT session_id, expires_at, fetch_id, tags FROM sessions",
                (),
            )
            .await?;
        let mut rows = Vec::new();
        while let Some(row) = results
            .next()
            .await
            .expect("Failed to read a row from Turso")
        {
            rows.push(Self::map_row_to_dbsession(row)?);
        }
        Ok(rows)
    }

    /// Helper function to convert a `turso::rows::Row` into a `DbSession`.
    fn map_row_to_dbsession(row: Row) -> Result<DbSession> {
        let session_id_val = row.get::<String>(0)?;
        let expires_at_val = row.get::<String>(1)?;
        let fetch_id_val = row.get::<i32>(2)?;
        let tags_val = row.get::<String>(3)?;
        let session_id = SessionId(session_id_val);
        let expires_at = DateTime::parse_from_rfc3339(&expires_at_val)?.with_timezone(&Utc);
        let fetch_id = FetchId(u32::try_from(fetch_id_val).unwrap());
        let tags: Tags =
            serde_json::from_str(&tags_val).expect("Failed to read tags from database");

        Ok(DbSession {
            session_id,
            expires_at,
            fetch_id,
            tags,
        })
    }

    /// Deletes a session from the database by its `SessionId`.
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        let conn = self.client.connect()?;
        conn.execute(
            "DELETE FROM sessions WHERE session_id = ?",
            (session_id.0.clone(),),
        )
        .await?;
        Ok(())
    }

    /// Deletes all sessions from the database that have expired before the given `now` timestamp.
    pub async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> Result<()> {
        let conn = self.client.connect()?;
        conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?",
            (now.to_rfc3339(),),
        )
        .await?;
        Ok(())
    }
}

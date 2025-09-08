// src/db.rs

use crate::db_models::DbSession;
use crate::models::{FetchId, SessionId, TagsAux};
use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use serde_json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use turso::{Builder, Database, Row};

/// Client for interacting with the Turso (SQLite) database.
pub struct DbClient {
    client: Arc<Database>,
    db_file: PathBuf,
}

impl DbClient {
    /// Creates a new `DbClient` and initializes the database schema.
    pub async fn new(db_file: &Path) -> Result<Self> {
        let client = DbClient {
            client: Arc::new(
                Builder::new_local(db_file.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Cannot convert path name to unicode: {:?}", db_file)
                })?)
                .build()
                .await
                .with_context(|| {
                    format!(
                        "Failed to open db (and/or its wal file). File name: {:?}",
                        db_file
                    )
                })?,
            ),
            db_file: PathBuf::from(db_file),
        };
        client.init_db().await.with_context(|| {
            format!("Failed to init db file {:?} (and/or its wal file)", db_file)
        })?;
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
        // Serialize TagAux into a JSON string for storage
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
        .await
        .with_context(|| {
            format!(
                "Failed to insert into sessions. File name: {:?}",
                self.db_file
            )
        })?;
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
            .await
            .with_context(|| format!("Failed to load sessions. File name: {:?}", self.db_file))?;
        let mut rows = Vec::new();
        while let Some(row) = results.next().await? {
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
        let fetch_id = FetchId(u32::try_from(fetch_id_val)?);
        let tags: TagsAux = serde_json::from_str(&tags_val)?;

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
        .await
        .with_context(|| format!("Failed to delete session. File name: {:?}", self.db_file))?;
        Ok(())
    }

    /// Deletes all sessions from the database that have expired before the given `now` timestamp.
    pub async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> Result<()> {
        let conn = self.client.connect()?;
        conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?",
            (now.to_rfc3339(),),
        )
        .await
        .with_context(|| format!("Failed to expire sessions. File name: {:?}", self.db_file))?;
        Ok(())
    }
}

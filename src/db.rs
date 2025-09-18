use crate::db_models::DbSession;
use crate::models::{FetchId, SessionId, TagsAux};
use crate::utils;
use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use turso::{Builder, Connection, Row}; // Added Connection

/// Client for interacting with the Turso (SQLite) database.
pub struct DbClient {
    conn: Arc<Mutex<Connection>>, // Persist the connection
    db_file: PathBuf,
}

impl DbClient {
    /// Creates a new `DbClient` and initializes the database schema.
    pub async fn new(db_file: &Path) -> Result<Self> {
        let turso_db_client = Arc::new(
            Builder::new_local(db_file.to_str().ok_or_else(|| {
                anyhow::anyhow!("Cannot convert path name to unicode: {:?}", db_file)
            })?)
            .build()
            .await
            .with_context(|| {
                format!("Failed to open db (and/or its wal file). File name: {db_file:?}")
            })?,
        );

        // Establish the connection once and no_stop it
        let conn = Arc::new(Mutex::new(turso_db_client.connect()?));

        let client = DbClient {
            conn,
            db_file: PathBuf::from(db_file),
        };
        client
            .init_db()
            .await
            .with_context(|| format!("Failed to init db file {db_file:?} (and/or its wal file)"))?;
        Ok(client)
    }

    /// Initializes the `sessions` table if it doesn't already exist.
    async fn init_db(&self) -> Result<()> {
        self.conn
            .lock()
            .await
            .execute(
                "CREATE TABLE IF NOT EXISTS sessions (
                    session_id TEXT PRIMARY KEY,
                    expires_at TEXT NOT NULL,
                    fetch_id INTEGER NOT NULL,
                    tags TEXT NOT NULL,
                    max_points INTEGER NOT NULL,
                    max_point_age TEXT,
                    reject_data BOOL NOT NULL,
                    no_stop BOOL NOT NULL,
                    log TEXT, -- JSON
                    name TEXT
                )",
                (),
            )
            .await?;
        Ok(())
    }

    /// Inserts a `DbSession` into the database.
    pub async fn insert_session(&self, session: &DbSession) -> Result<()> {
        let tags_json = serde_json::to_string(&session.tags)?;
        self.conn
            .lock()
            .await
            .execute(
                "INSERT INTO sessions (session_id, expires_at, fetch_id, tags, max_points, max_point_age, reject_data, no_stop, log, name) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    session.session_id.0.clone(),
                    session.expires_at.to_rfc3339(), // Store DateTime as ISO 8601 string
                    session.fetch_id.0,
                    tags_json,
		    session.max_points as u64,
		    session.max_point_age.map(|timedelta: chrono::TimeDelta| utils::format_timedelta(&timedelta)),
		    session.reject_data,
		    session.no_stop,
		    session.log.as_ref().map(serde_json::to_string).transpose()?,
		    session.name.clone(),
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
        let mut results = self
            .conn
            .lock()
            .await
            .query(
                "SELECT session_id, expires_at, fetch_id, tags, max_points, max_point_age, reject_data, no_stop, log, name FROM sessions",
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
        let max_points_val = row.get::<u64>(4)?;
        let max_point_age_val = row.get::<Option<String>>(5)?;
        let reject_data_val = row.get::<bool>(6)?;
        let no_stop_val = row.get::<bool>(7)?;
        let log_val = row.get::<Option<String>>(8)?;
        let name_val = row.get::<Option<String>>(9)?;

        let session_id = SessionId(session_id_val);
        let expires_at = DateTime::parse_from_rfc3339(&expires_at_val)?.with_timezone(&Utc);
        let fetch_id = FetchId(u32::try_from(fetch_id_val)?);
        let tags: TagsAux = serde_json::from_str(&tags_val)?;
        let max_points = max_points_val.try_into()?;
        let max_point_age = max_point_age_val
            .map(|x| utils::parse_timedelta(&x))
            .transpose()?;
        let reject_data = reject_data_val;
        let no_stop = no_stop_val;
        let log = log_val.map(|x| serde_json::from_str(&x)).transpose()?;
        let name = name_val;

        Ok(DbSession {
            session_id,
            expires_at,
            fetch_id,
            tags,
            max_points,
            max_point_age,
            reject_data,
            no_stop,
            log,
            name,
        })
    }

    /// Deletes a session from the database by its `SessionId`.
    pub async fn delete_session(&self, session_id: &SessionId) -> Result<()> {
        self.conn
            .lock()
            .await
            .execute(
                "DELETE FROM sessions WHERE session_id = ?",
                (session_id.0.clone(),),
            )
            .await
            .with_context(|| format!("Failed to delete session. File name: {:?}", self.db_file))?;
        Ok(())
    }
}

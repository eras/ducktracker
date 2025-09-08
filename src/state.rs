use crate::bounded_set;
use crate::db::DbClient;
use crate::db_models::DbSession;
use crate::models::{self, Location, Update, UpdateChange};
use crate::utils;
use anyhow::{Context as AnyhowContext, Result as AnyhowResult};
use chrono::Utc;
use std::path::Path;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::broadcast; // Use AnyhowResult to differentiate from crate::Error
use tokio::task;

const MAX_TOKENS: usize = 100000;

pub enum Error {
    NoSuchSession,
    SessionExpired,
}

pub struct State {
    pub sessions: dashmap::DashMap<models::SessionId, models::Session>,
    pub updates: Updates,
    public_tags: models::Tags,
    pub default_tag: models::Tag,

    pub http_scheme: String,
    pub server_name: Option<String>,

    next_fetch_id: models::FetchId,

    pub users: HashMap<String, String>, // Key: username, Value:  password (should be hashed)
    pub tokens: bounded_set::BoundedSet<String>,

    db_client: Arc<DbClient>, // Add the database client
}

impl State {
    pub async fn new(
        updates: Updates,
        database_file: &Path,
        password_file: &Path,
        default_tag: &str,
        http_scheme: &str,
        server_name: Option<&str>,
    ) -> AnyhowResult<Self> {
        let users = utils::read_colon_separated_file(password_file)
            .with_context(|| format!("Failed to open a {:?}", password_file))?;

        let db_client = Arc::new(DbClient::new(&database_file).await?); // Initialize DB client

        let default_tag = models::Tag(default_tag.to_string());
        let mut public_tags = models::Tags::new();
        public_tags.0.insert(default_tag.clone());

        let mut state = Self {
            updates,
            sessions: dashmap::DashMap::new(),
            next_fetch_id: models::FetchId::default(),
            public_tags,
            users,
            tokens: bounded_set::BoundedSet::new(MAX_TOKENS),
            db_client,
            default_tag,
            http_scheme: http_scheme.to_string(),
            server_name: server_name.map(|x| x.to_string()),
        };

        state.load_state().await?;

        Ok(state)
    }

    async fn load_state(&mut self) -> AnyhowResult<()> {
        // Load sessions from the database
        let now = Utc::now();
        let mut highest_fetch_id = 0;

        let db_sessions = self.db_client.get_all_sessions().await?;
        for db_session in db_sessions {
            if db_session.expires_at > now {
                // Session is still active, re-add it to the in-memory state
                let session: models::Session = db_session.into();

                // Re-populate public tags from the restored session
                for tag in session.tags.0.iter() {
                    if tag.is_public() {
                        self.public_tags.0.insert(tag.as_tag());
                    }
                }

                // Update next_fetch_id to be greater than any loaded fetch_id
                if session.fetch_id.0 > highest_fetch_id {
                    highest_fetch_id = session.fetch_id.0;
                }

                self.sessions.insert(session.session_id.clone(), session);
            } else {
                // Session has expired, remove it from the database
                self.db_client
                    .delete_session(&db_session.session_id)
                    .await?;
            }
        }
        self.next_fetch_id = models::FetchId(highest_fetch_id + 1);
        Ok(())
    }

    pub fn authenticate(&self, user: &str, password: &str) -> bool {
        self.users.get(user).map_or(false, |p| p == password)
    }

    pub fn create_token(&mut self, user: &str, password: &str) -> Option<String> {
        if self.authenticate(user, password) {
            let token = utils::generate_id();
            self.tokens.insert(token.clone());
            Some(token)
        } else {
            None
        }
    }

    pub fn check_token(&self, token: &str) -> bool {
        self.tokens.contains(token)
    }

    pub fn get_public_tags(&self) -> models::Tags {
        self.public_tags.clone()
    }

    pub async fn add_session(
        &mut self,
        expires_at: chrono::DateTime<Utc>,
        tags_aux: models::TagsAux,
    ) -> models::SessionId {
        let session_id = models::SessionId(utils::generate_id());
        let fetch_id = self.generate_fetch_id();
        let mut tags_aux = tags_aux;

        if tags_aux.0.is_empty() {
            tags_aux.0.insert(models::TagAux {
                name: self.default_tag.clone(),
                visibility: models::TagVisibility::Public,
            });
        }

        // Create a new session and store it in the DashMap.
        let new_session = models::Session {
            session_id: session_id.clone(),
            locations: Vec::new(),
            expires_at,
            fetch_id: fetch_id.clone(),
            tags: tags_aux.clone().into(),
        };
        self.sessions
            .insert(session_id.clone(), new_session.clone());
        self.add_fetch(fetch_id, tags_aux).await;

        // Persist the new session to the database asynchronously
        let db_client = self.db_client.clone();
        let db_session: DbSession = (&new_session).into();
        task::spawn(async move {
            if let Err(e) = db_client.insert_session(&db_session).await {
                eprintln!("Failed to insert session into DB: {:?}", e);
            }
        });

        session_id
    }

    pub async fn remove_session(&mut self, session_id: &models::SessionId) {
        if let Some((_session_id, session)) = self.sessions.remove(session_id) {
            let context = UpdateContext {
                tags: session.tags.as_tags(),
            };
            let update = Update {
                server_time: models::TimeUsec(std::time::SystemTime::now()),
                interval: 0u64,
                changes: [UpdateChange::ExpireFetch {
                    fetch_id: session.fetch_id,
                }]
                .to_vec(),
            };
            self.updates.send_update(context, update);

            // Delete the session from the database asynchronously
            let db_client = self.db_client.clone();
            let session_id_clone = session_id.clone();
            task::spawn(async move {
                if let Err(e) = db_client.delete_session(&session_id_clone).await {
                    eprintln!("Failed to delete session from DB: {:?}", e);
                }
            });
        }
    }

    pub async fn add_fetch(&mut self, fetch_id: models::FetchId, tags_aux: models::TagsAux) {
        for tag in tags_aux.0.iter() {
            if tag.visibility == models::TagVisibility::Public {
                self.public_tags.0.insert(tag.as_tag());
            }
        }

        let public_tags = tags_aux.public_tags();
        let tags: models::Tags = tags_aux.into();

        let context = UpdateContext { tags: tags.clone() };
        let mut new_tags = HashMap::new();
        new_tags.insert(fetch_id.clone(), tags);
        let update = Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::AddFetch {
                tags: new_tags,
                public: public_tags,
            }]
            .to_vec(),
        };
        self.updates.send_update(context, update);
    }

    pub fn generate_fetch_id(&mut self) -> models::FetchId {
        let id = self.next_fetch_id.clone();
        self.next_fetch_id.0 += 1;
        id
    }

    pub async fn add_location(&mut self, data: &models::PostRequest) -> Result<(), Error> {
        // Find and get a mutable reference to the session from the DashMap.
        let mut session = match self.sessions.get_mut(&data.session_id) {
            Some(s) => s,
            None => return Err(Error::NoSuchSession),
        };

        let now = chrono::Utc::now();
        if session.expires_at < now {
            drop(session);
            self.sessions.remove(&data.session_id);

            // If session expired, also remove it from the database asynchronously
            let db_client = self.db_client.clone();
            let session_id_clone = data.session_id.clone();
            task::spawn(async move {
                if let Err(e) = db_client.delete_session(&session_id_clone).await {
                    eprintln!("Failed to delete expired session from DB: {:?}", e);
                }
            });

            return Err(Error::SessionExpired);
        }

        // Create a new Location struct with the provided data.
        let new_location = Location {
            lat: data.latitude,
            lon: data.longitude,
            acc: data.accuracy,
            spd: data.speed,
            provider: data.provider.unwrap_or(0),
            time: data.time,
        };

        // Update the last_location field of the session.
        session.locations.push(new_location.clone());

        let mut points = std::collections::HashMap::new();
        points.insert(session.fetch_id, [new_location].to_vec());

        let context = UpdateContext {
            tags: session.tags.as_tags().clone(),
        };

        let update = Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::Add { points }].to_vec(),
        };

        self.updates.send_update(context, update);

        Ok(())
    }
}

pub struct Updates {
    updates_tx: broadcast::Sender<(UpdateContext, Update)>,
}

pub type UpdateBroadcast =
    Result<(UpdateContext, Update), tokio_stream::wrappers::errors::BroadcastStreamRecvError>;

// Used to share the context when a particular update was created.
// E.g. in case of AddPoints it is not known which tags are relevant to a fetch_id,
// so we can get this information from there
#[derive(Debug, Clone)]
pub struct UpdateContext {
    pub tags: models::Tags,
}

impl Updates {
    pub fn new() -> Self {
        let (updates_tx, _updates_rx) = tokio::sync::broadcast::channel(10);
        Self { updates_tx }
    }

    fn send_update(&self, context: UpdateContext, update: Update) {
        match self.updates_tx.send((context, update)) {
            Ok(_) => (),
            Err(_) => (), // this is fine.. it happens when there are no subscribers.
        }
    }

    fn initial_update(&self, tags: models::Tags, state: &State) -> (UpdateContext, Update) {
        let mut changes = [UpdateChange::Reset].to_vec();
        let fetch_tags: HashMap<models::FetchId, models::Tags> = state
            .sessions
            .iter()
            .filter_map(|x| {
                let session = &x.value();
                let shared_tags = &session.tags.as_tags() & &tags;
                if shared_tags.len() > 0 {
                    Some((session.fetch_id, shared_tags))
                } else {
                    None
                }
            })
            .collect();
        let points = state
            .sessions
            .iter()
            .filter(|x| (&x.value().tags.as_tags() & &tags).len() > 0)
            .map(|x| (x.value().fetch_id, x.value().locations.clone()))
            .collect();
        changes.push(UpdateChange::AddFetch {
            tags: fetch_tags,
            public: state.public_tags.clone(),
        });
        changes.push(UpdateChange::Add { points });
        (
            UpdateContext { tags },
            Update {
                server_time: models::TimeUsec(std::time::SystemTime::now()),
                interval: 0u64,
                changes,
            },
        )
    }

    pub async fn updates(
        &self,
        state: &State,
        tags: models::Tags,
    ) -> Pin<Box<dyn futures_util::stream::Stream<Item = UpdateBroadcast>>> {
        let updates = tokio_stream::wrappers::BroadcastStream::new(self.updates_tx.subscribe());

        let initial_message = self.initial_update(tags.clone(), &state);
        let first_stream =
            futures_util::stream::once(async move { UpdateBroadcast::Ok(initial_message) });

        // Filter our messages this subscription doesn't see
        // Modify tags so that the client doesn't learn about new tags
        let updates = {
            futures_util::StreamExt::filter_map(updates, move |x| {
                let subscribed_tags = tags.clone();
                async move {
                    match x {
                        Ok((context, update)) => {
                            let shared_tags = &context.tags & &subscribed_tags;
                            let shared_context = UpdateContext { tags: shared_tags };
                            match update.filter_map(&subscribed_tags, &context).await {
                                None => None,
                                Some(update) => Some(Ok((shared_context, update))),
                            }
                        }
                        Err(_) => Some(x),
                    }
                }
            })
        };

        let updates = futures_util::StreamExt::chain(first_stream, updates);

        Box::pin(updates)
    }
}

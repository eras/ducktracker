use crate::bounded_set;
use crate::box_coords::BoxCoords;
use crate::db::DbClient;
use crate::db_models::DbSession;
use crate::models::{self, Location, Update, UpdateChange};
use crate::utils;
use anyhow::{Context as AnyhowContext, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::path::Path;
use std::time::Duration;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::{Mutex, Notify, broadcast};
use tokio::task;

const MAX_TOKENS: usize = 100000;

pub enum Error {
    NoSuchSession,
    SessionExpired,
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: models::SessionId,
    pub locations: VecDeque<Location>,
    pub expires_at: DateTime<Utc>,
    pub fetch_id: models::FetchId,
    pub tags: models::TagsAux,
}

pub struct State {
    sessions: HashMap<models::SessionId, Session>,
    session_added: Arc<Notify>,
    expirations: BinaryHeap<Reverse<(DateTime<Utc>, models::SessionId)>>,

    pub updates: Updates,
    pub public_tags: models::Tags, // Make `public_tags` public
    pub default_tag: models::Tag,

    pub http_scheme: String,
    pub server_name: Option<String>,

    next_fetch_id: models::FetchId,

    pub users: HashMap<String, String>, // Key: username, Value:  password (should be hashed)
    pub tokens: bounded_set::BoundedSet<String>,

    // Prometheus authentication
    pub prometheus_user: Option<String>,
    pub prometheus_password: Option<String>,

    db_client: Arc<DbClient>, // Add the database client

    max_points: usize,

    pub update_interval: Duration,

    pub box_coords: Option<BoxCoords>,
}

impl State {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        updates: Updates,
        database_file: &Path,
        password_file: &Path,
        default_tag: &str,
        http_scheme: &str,
        server_name: Option<&str>,
        max_points: usize,
        update_interval: Duration,
        box_coords: Option<BoxCoords>,
        prometheus_user: Option<String>,
        prometheus_password: Option<String>,
    ) -> AnyhowResult<Arc<Mutex<Self>>> {
        let users = utils::read_colon_separated_file(password_file)
            .with_context(|| format!("Failed to open a {password_file:?}"))?;

        let db_client = Arc::new(DbClient::new(database_file).await?); // Initialize DB client

        let default_tag = models::Tag(default_tag.to_string());
        let mut public_tags = models::Tags::new();
        public_tags.0.insert(default_tag.clone());

        let mut state = Self {
            updates,
            sessions: HashMap::new(),
            session_added: Arc::new(Notify::new()),
            expirations: BinaryHeap::new(),
            next_fetch_id: models::FetchId::default(),
            public_tags,
            users,
            tokens: bounded_set::BoundedSet::new(MAX_TOKENS),
            db_client,
            default_tag,
            http_scheme: http_scheme.to_string(),
            server_name: server_name.map(|x| x.to_string()),
            max_points,
            update_interval,
            box_coords,
            prometheus_user,
            prometheus_password,
        };

        state.load_state().await?;

        let arc_state = Arc::new(Mutex::new(state));

        tokio::spawn(Self::remove_expired_sessions_task(arc_state.clone()));

        Ok(arc_state)
    }

    pub fn iter_sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub fn num_sessions(&self) -> usize {
        self.sessions.len()
    }

    async fn load_state(&mut self) -> AnyhowResult<()> {
        // Load sessions from the database
        let now = Utc::now();
        let mut highest_fetch_id = 0;

        let db_sessions = self.db_client.get_all_sessions().await?;
        for db_session in db_sessions {
            if db_session.expires_at > now {
                // Session is still active, re-add it to the in-memory state
                let session: Session = db_session.into();

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

                self.expirations
                    .push(Reverse((session.expires_at, session.session_id.clone())));
                self.sessions.insert(session.session_id.clone(), session);
                self.session_added.notify_waiters();
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
        self.users.get(user).is_some_and(|p| p == password)
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
        expires_at: DateTime<Utc>,
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
        let new_session = Session {
            session_id: session_id.clone(),
            locations: VecDeque::new(),
            expires_at,
            fetch_id,
            tags: tags_aux.clone(),
        };
        self.expirations.push(Reverse((
            new_session.expires_at,
            new_session.session_id.clone(),
        )));
        self.sessions
            .insert(session_id.clone(), new_session.clone());
        self.session_added.notify_waiters();
        self.add_fetch(fetch_id, tags_aux).await;

        // Persist the new session to the database asynchronously
        let db_client = self.db_client.clone();
        let db_session: DbSession = (&new_session).into();
        task::spawn(async move {
            if let Err(e) = db_client.insert_session(&db_session).await {
                eprintln!("Failed to insert session into DB: {e:?}");
            }
        });

        session_id
    }

    async fn remove_expired_sessions_task(state: Arc<Mutex<State>>) {
        let session_added = state.lock().await.session_added.clone();
        loop {
            let next_sleep_duration;
            {
                let mut state = state.lock().await;

                state.remove_expired_sessions().await;

                if let Some(Reverse((expires_at, _))) = state.expirations.peek() {
                    let now = Utc::now();

                    next_sleep_duration = expires_at
                        .signed_duration_since(now)
                        .to_std()
                        // Handle the extremely unlikely error case reasonably?
                        .unwrap_or(Duration::from_secs(1));
                } else {
                    next_sleep_duration = Duration::from_secs(3600);
                }
            }

            log::debug!("Sleeping for {next_sleep_duration:?}");

            #[rustfmt::skip]
            tokio::select! {
		_ = tokio::time::sleep(next_sleep_duration) => {
                    // Sleep finished, loop to re-process expired sessions
		}
		_ = session_added.notified() => {
		    // A new session was added/updated that might be earlier, or an existing
		    // earliest session was removed. Re-loop immediately to re-evaluate the next
		    // sleep time.
		}
            }
        }
    }

    async fn remove_expired_sessions(&mut self) {
        let now = Utc::now();
        loop {
            if let Some(Reverse((expires_at, session_id))) = self.expirations.peek() {
                if expires_at <= &now {
                    let session_id = session_id.clone();
                    self.expirations.pop();
                    if self.sessions.contains_key(&session_id) {
                        log::debug!("Removing expired session {session_id}");
                        self.remove_session(&session_id).await;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub async fn remove_session(&mut self, session_id: &models::SessionId) {
        // we let the entry in self.expirations remain, it dose no harm as we never*
        // give out duplicate session ids
        if let Some(session) = self.sessions.remove(session_id) {
            let context = UpdateContext {
                tags: session.tags.as_tags(),
                is_heartbeat: false,
            };
            let meta = models::UpdateMeta {
                server_time: models::TimeUsec(std::time::SystemTime::now()),
                interval: 0u64,
            };
            let update = Update {
                meta,
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
                    eprintln!("Failed to delete session from DB: {e:?}");
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

        let context = UpdateContext {
            tags: tags.clone(),
            is_heartbeat: false,
        };
        let mut new_tags = HashMap::new();
        new_tags.insert(fetch_id, tags);
        let meta = models::UpdateMeta {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
        };
        let update = Update {
            meta,
            changes: [UpdateChange::AddFetch {
                tags: new_tags,
                public: public_tags,
                max_points: self.max_points,
            }]
            .to_vec(),
        };
        self.updates.send_update(context, update);
    }

    pub fn generate_fetch_id(&mut self) -> models::FetchId {
        let id = self.next_fetch_id;
        self.next_fetch_id.0 += 1;
        id
    }

    pub async fn add_location(&mut self, data: &models::PostRequest) -> Result<(), Error> {
        let session = match self.sessions.get_mut(&data.session_id) {
            Some(s) => s,
            None => return Err(Error::NoSuchSession),
        };

        let now = Utc::now();
        if session.expires_at < now {
            self.remove_session(&data.session_id).await;
            return Err(Error::SessionExpired);
        }

        // Apply coordinate wrapping if box_coords are configured
        let (mut latitude, mut longitude) = (data.latitude, data.longitude);
        if let Some(box_c) = self.box_coords {
            latitude = box_c.wrap_latitude(latitude);
            longitude = box_c.wrap_longitude(longitude);
        }

        // Create a new Location struct with the provided data.
        let new_location = Location {
            lat: latitude,  // Use potentially wrapped latitude
            lon: longitude, // Use potentially wrapped longitude
            acc: data.accuracy,
            spd: data.speed,
            provider: data.provider.unwrap_or(0),
            time: data.time,
        };
        let _ = data; // prevent accidentally accessing any fields in data

        let locs = &mut session.locations;
        locs.push_back(new_location.clone());
        if locs.len() > self.max_points {
            locs.pop_front();
        }

        // if sessions.locations.len() > state.max_points {
        //     //sessions.locations.
        // }

        let mut points = std::collections::HashMap::new();
        points.insert(session.fetch_id, [new_location].to_vec());

        let context = UpdateContext {
            tags: session.tags.as_tags().clone(),
            is_heartbeat: false,
        };

        let meta = models::UpdateMeta {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
        };

        let update = Update {
            meta,
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
    Result<UpdateWithContext, tokio_stream::wrappers::errors::BroadcastStreamRecvError>;

// Used to share the context when a particular update was created.
// E.g. in case of AddPoints it is not known which tags are relevant to a fetch_id,
// so we can get this information from there
#[derive(Debug, Clone)]
pub struct UpdateContext {
    pub tags: models::Tags,
    pub is_heartbeat: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateChangeWithContext {
    pub change: UpdateChange,
    pub context: UpdateContext,
}

impl std::convert::From<UpdateChangeWithContext> for UpdateChange {
    fn from(val: UpdateChangeWithContext) -> Self {
        val.change
    }
}

#[derive(Debug, Clone)]
pub struct UpdateWithContext {
    pub meta: models::UpdateMeta,
    pub updates: Vec<UpdateChangeWithContext>,
}

impl UpdateWithContext {
    pub fn ingest(&mut self, mut other: UpdateWithContext) {
        self.updates.append(&mut other.updates)
    }
}

impl std::convert::From<(UpdateContext, Update)> for UpdateWithContext {
    fn from(value: (UpdateContext, Update)) -> Self {
        let context = value.0;
        let meta = value.1.meta;
        let changes = value.1.changes;
        UpdateWithContext {
            meta,
            updates: changes
                .into_iter()
                .map(|change| UpdateChangeWithContext {
                    change,
                    context: context.clone(),
                })
                .collect(),
        }
    }
}

impl std::convert::From<UpdateWithContext> for Update {
    fn from(val: UpdateWithContext) -> Self {
        Update {
            meta: val.meta,
            changes: val
                .updates
                .into_iter()
                .map(|change| change.change)
                .collect(),
        }
    }
}

impl Updates {
    pub async fn new(update_interval: Duration) -> Self {
        let (updates_tx, _updates_rx) = tokio::sync::broadcast::channel(10);
        let _ = tokio::task::spawn(Self::update_heartbeat(updates_tx.clone(), update_interval));
        Self { updates_tx }
    }

    async fn update_heartbeat(
        updates_tx: broadcast::Sender<(UpdateContext, Update)>,
        interval: Duration,
    ) {
        let context = UpdateContext {
            tags: models::Tags::new(),
            is_heartbeat: true,
        };
        let interval_seconds = interval.as_secs();
        loop {
            let update = models::Update {
                meta: models::UpdateMeta {
                    server_time: models::TimeUsec(std::time::SystemTime::now()),
                    interval: interval_seconds,
                },
                changes: vec![],
            };
            let _ignore = updates_tx.send((context.clone(), update.clone()));
            tokio::time::sleep(interval).await;
        }
    }

    #[allow(clippy::single_match)]
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
                let session = &x.1;
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
            .filter(|x| (&x.1.tags.as_tags() & &tags).len() > 0)
            .map(|x| (x.1.fetch_id, x.1.locations.iter().cloned().collect()))
            .collect();
        changes.push(UpdateChange::AddFetch {
            tags: fetch_tags,
            public: state.public_tags.clone(),
            max_points: state.max_points,
        });
        changes.push(UpdateChange::Add { points });
        (
            UpdateContext {
                tags,
                is_heartbeat: false,
            },
            Update {
                meta: models::UpdateMeta {
                    server_time: models::TimeUsec(std::time::SystemTime::now()),
                    interval: 0u64,
                },
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

        let initial_message = self.initial_update(tags.clone(), state);
        let first_stream = futures_util::stream::once(async move {
            UpdateBroadcast::Ok(UpdateWithContext::from(initial_message))
        });

        // Filter our messages this subscription doesn't see
        // Modify tags so that the client doesn't learn about new tags
        // Also bake the UpdateContext inside each UpdateChange with UpdateChangeWithContext, so it becomes possible to merge with other UpdateChanges
        let updates = {
            futures_util::StreamExt::filter_map(updates, move |x| {
                let subscribed_tags = tags.clone();
                async move {
                    match x {
                        Ok((context, update)) => {
                            let shared_tags = &context.tags & &subscribed_tags;
                            let shared_context = UpdateContext {
                                tags: shared_tags,
                                ..context
                            };

                            update
                                .filter_map(&subscribed_tags, &context)
                                .await
                                .map(|update| Ok(UpdateWithContext::from((shared_context, update))))
                        }
                        Err(_) => None,
                    }
                }
            })
        };

        let updates = futures_util::StreamExt::chain(first_stream, updates);

        Box::pin(updates)
    }
}

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
use typesafe_builder::*;

const MAX_TOKENS: usize = 100000;

pub enum Error {
    NoSuchSession,
    SessionExpired,
}

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone, Builder)]
pub struct Session {
    #[builder(required)]
    session_id: models::SessionId,
    #[builder(required)]
    locations: VecDeque<Location>,
    #[builder(required)]
    expires_at: DateTime<Utc>,
    #[builder(required)]
    fetch_id: models::FetchId,
    #[builder(required)]
    tags: models::TagsAux,
    #[builder(required)]
    max_points: usize,
    #[builder(required)]
    max_point_age: Option<chrono::TimeDelta>,

    // is the oldest data of this Session added to the data expiration structure?
    #[builder(default)]
    added_to_expiration: bool,

    // should we reject new data? Used to provide no_stop and also deal with https://github.com/bilde2910/Hauk/issues/230
    #[builder(required)]
    reject_data: bool,

    // should the data of this session survive even after stopping it, and let expiration deal with it?
    #[builder(required)]
    no_stop: bool,
}

impl Session {
    pub fn session_id(&self) -> &models::SessionId {
        &self.session_id
    }

    pub fn locations(&self) -> &VecDeque<Location> {
        &self.locations
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn fetch_id(&self) -> &models::FetchId {
        &self.fetch_id
    }

    pub fn tags(&self) -> &models::TagsAux {
        &self.tags
    }

    pub fn max_points(&self) -> usize {
        self.max_points
    }

    pub fn max_point_age(&self) -> Option<chrono::TimeDelta> {
        self.max_point_age
    }

    pub fn reject_data(&self) -> bool {
        self.reject_data
    }

    pub fn no_stop(&self) -> bool {
        self.no_stop
    }
}

type Expiration = BinaryHeap<Reverse<(DateTime<Utc>, models::SessionId)>>;

pub struct State {
    sessions: HashMap<models::SessionId, Session>,
    session_added: Arc<Notify>,

    // A fast way to find the oldest expiring session
    session_expirations: Expiration,

    // A fast way to find the oldest expiring datum
    data_expirations: Expiration,
    data_expire_added: Arc<Notify>,

    pub updates: Updates,
    public_tags: HashMap<models::Tag, usize>, // reference count
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

    default_points: usize,

    default_max_point_age: Option<chrono::TimeDelta>,

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
        default_points: usize,
        default_max_point_age: Option<chrono::TimeDelta>,
        update_interval: Duration,
        box_coords: Option<BoxCoords>,
        prometheus_user: Option<String>,
        prometheus_password: Option<String>,
    ) -> AnyhowResult<Arc<Mutex<Self>>> {
        let users = utils::read_colon_separated_file(password_file)
            .with_context(|| format!("Failed to open a {password_file:?}"))?;

        let db_client = Arc::new(DbClient::new(database_file).await?); // Initialize DB client

        let default_tag = models::Tag(default_tag.to_string());

        let mut state = Self {
            updates,
            sessions: HashMap::new(),
            session_added: Arc::new(Notify::new()),
            session_expirations: Expiration::new(),
            data_expirations: Expiration::new(),
            data_expire_added: Arc::new(Notify::new()),
            next_fetch_id: models::FetchId::default(),
            public_tags: HashMap::new(),
            users,
            tokens: bounded_set::BoundedSet::new(MAX_TOKENS),
            db_client,
            default_tag: default_tag.clone(),
            http_scheme: http_scheme.to_string(),
            server_name: server_name.map(|x| x.to_string()),
            max_points,
            default_points,
            default_max_point_age,
            update_interval,
            box_coords,
            prometheus_user,
            prometheus_password,
        };

        state.add_public_tag(default_tag);

        state.load_state().await?;

        let arc_state = Arc::new(Mutex::new(state));

        tokio::spawn(Self::remove_expired_sessions_task(arc_state.clone()));
        tokio::spawn(Self::remove_expired_data_task(arc_state.clone()));

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
                self.add_public_tags_for_session(&session);

                // Update next_fetch_id to be greater than any loaded fetch_id
                if session.fetch_id.0 > highest_fetch_id {
                    highest_fetch_id = session.fetch_id.0;
                }

                self.session_expirations
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

    pub fn authenticate(&self, user: &str, password: &str) -> anyhow::Result<bool> {
        self.users.get(user).map_or_else(
            || Err(anyhow::anyhow!("No such user: {}", user)),
            |p| {
                if p.starts_with("$") {
                    bcrypt::verify(password, &p)
                        .map_err(|e| anyhow::anyhow!("Failed to use bcrypt verify: {}", e))
                } else {
                    Ok(p == password)
                }
            },
        )
    }

    pub fn create_token(&mut self, user: &str, password: &str) -> anyhow::Result<Option<String>> {
        if self.authenticate(user, password)? {
            let token = utils::generate_id();
            self.tokens.insert(token.clone());
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    pub fn check_token(&self, token: &str) -> bool {
        self.tokens.contains(token)
    }

    pub fn public_tags<'a>(&self) -> models::Tags {
        self.public_tags
            .iter()
            .map(|(tag, _)| tag.clone())
            .collect()
    }

    pub async fn add_session(
        &mut self,
        expires_at: DateTime<Utc>,
        tags_aux: models::TagsAux,
        options: models::Options,
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

        let max_points = std::cmp::min(
            self.max_points,
            std::cmp::max(1usize, options.max_points.unwrap_or(self.default_points)),
        );

        // Create a new session and store it in the DashMap.
        let new_session = Session {
            session_id: session_id.clone(),
            locations: VecDeque::new(),
            expires_at,
            fetch_id,
            tags: tags_aux.clone(),
            max_points,
            max_point_age: options.max_point_age.or(self.default_max_point_age),
            added_to_expiration: false,
            reject_data: false,
            no_stop: options.no_stop,
        };
        log::debug!(
            "Creating new session {} with options {:?} expires at {}",
            &session_id,
            &options,
            &expires_at,
        );

        self.session_expirations.push(Reverse((
            new_session.expires_at,
            new_session.session_id.clone(),
        )));
        self.sessions
            .insert(session_id.clone(), new_session.clone());
        self.session_added.notify_waiters();
        self.add_public_tags_for_session(&new_session);
        self.add_fetch(
            fetch_id,
            tags_aux,
            new_session.max_points,
            new_session.max_point_age,
        )
        .await;

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
            let next_sleep_duration = {
                let mut state = state.lock().await;

                state.remove_expired_sessions().await;

                get_sleep_duration(&state.session_expirations)
            };

            log::debug!("remove_expired_sessions_task: Sleeping for {next_sleep_duration:?}");

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
            if let Some(Reverse((expires_at, session_id))) = self.session_expirations.peek() {
                if expires_at <= &now {
                    let session_id = session_id.clone();
                    self.session_expirations.pop();
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

    // I have sinned, I just copypasted the previous function instead of figuring out how to merge them more
    async fn remove_expired_data_task(state: Arc<Mutex<State>>) {
        let data_expire_added = state.lock().await.data_expire_added.clone();
        loop {
            let next_sleep_duration = {
                let mut state = state.lock().await;

                state.remove_expired_data().await;

                get_sleep_duration(&state.data_expirations)
            };

            log::debug!("remove_expired_data_task: Sleeping for {next_sleep_duration:?}");

            #[rustfmt::skip]
            tokio::select! {
		_ = tokio::time::sleep(next_sleep_duration) => {
                    // Sleep finished, loop to re-process expired datas
		}
		_ = data_expire_added.notified() => {
		    // A new data was added/updated that might be earlier, or an existing
		    // earliest data was removed. Re-loop immediately to re-evaluate the next
		    // sleep time.
		}
            }
        }
    }

    // Walks the data_expirations BinaryHeap and finds candidates for sessions with expired data;
    // invokes expire_data on such sessions
    async fn remove_expired_data(&mut self) {
        let now = Utc::now();
        loop {
            if let Some(Reverse((expires_at, session_id))) = self.data_expirations.peek() {
                if expires_at <= &now {
                    let session_id = session_id.clone();
                    self.data_expirations.pop();
                    if self.sessions.contains_key(&session_id) {
                        log::debug!("Removing expired data for {session_id}");
                        let _ignored = self.expire_data(&session_id, now).await;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Expire data that is older than now - session.max_point_age
    async fn expire_data(
        &mut self,
        session_id: &models::SessionId,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let session = match self.sessions.get_mut(session_id) {
            Some(s) => s,
            None => return Err(anyhow::anyhow!("No such session")),
        };
        if let Some(max_point_age) = session.max_point_age {
            let deadline = now - max_point_age;
            while let Some(front) = session.locations.front() {
                let location_time =
                    DateTime::<Utc>::from_timestamp_micros((front.time * 1000000f64) as i64)
                        .ok_or(anyhow::anyhow!(
                            "Failed to convert location timestamp to DateTime"
                        ))?;
                if location_time < deadline {
                    session.locations.pop_front();
                    session.added_to_expiration = false;
                } else {
                    break;
                }
            }

            State::add_data_expiration(
                &mut self.data_expirations,
                session,
                &mut self.data_expire_added,
            );
        }
        Ok(())
    }

    pub async fn request_remove_session(&mut self, session_id: &models::SessionId) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            log::debug!("Requested to remove a session: no_stop={}", session.no_stop);
            if session.no_stop {
                session.reject_data = true;
            } else {
                self.remove_session(session_id).await;
            }
        }
    }

    async fn remove_session(&mut self, session_id: &models::SessionId) {
        // we let the entry in self.expirations remain, it dose no harm as we never*
        // give out duplicate session ids
        if let Some(session) = self.sessions.remove(session_id) {
            log::debug!("Start removing session {session_id}");
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

            self.remove_public_tags_for_session(&session);

            // Delete the session from the database asynchronously
            let db_client = self.db_client.clone();
            let session_id_clone = session_id.clone();
            task::spawn(async move {
                if let Err(e) = db_client.delete_session(&session_id_clone).await {
                    eprintln!("Failed to delete session from DB: {e:?}");
                }
            });

            log::debug!("Done removing session {session_id}")
        }
    }

    fn add_public_tag(&mut self, tag: models::Tag) {
        *self.public_tags.entry(tag).or_insert(0) += 1;
    }

    fn add_public_tags_for_session(&mut self, session: &Session) {
        for tag in session.tags.0.iter() {
            if tag.is_public() {
                self.add_public_tag(tag.as_tag());
            }
        }
    }

    fn remove_public_tag(&mut self, tag: models::Tag) {
        use std::collections::hash_map::Entry;
        match self.public_tags.entry(tag) {
            Entry::Occupied(mut entry) => {
                let value = entry.get_mut();
                *value -= 1;
                if *value == 0 {
                    entry.remove();
                }
            }
            Entry::Vacant(_vacant_entry) => (),
        }
    }

    fn remove_public_tags_for_session(&mut self, session: &Session) {
        for tag in session.tags.0.iter() {
            if tag.is_public() {
                self.remove_public_tag(tag.as_tag());
            }
        }
    }

    pub async fn add_fetch(
        &mut self,
        fetch_id: models::FetchId,
        tags_aux: models::TagsAux,
        max_points: usize,
        max_point_age: Option<chrono::TimeDelta>,
    ) {
        let public_tags = tags_aux.public_tags();
        let tags: models::Tags = tags_aux.into();

        let context = UpdateContext {
            tags: tags.clone(),
            is_heartbeat: false,
        };
        let mut fetches = HashMap::new();
        let max_point_age = max_point_age.map(|x| x.num_seconds() as f64);
        fetches.insert(
            fetch_id,
            models::Fetch {
                tags,
                max_points,
                max_point_age,
            },
        );
        let meta = models::UpdateMeta {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
        };
        let update = Update {
            meta,
            changes: [UpdateChange::AddFetch {
                fetches,
                public: public_tags,
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

        if session.reject_data {
            return Err(Error::NoSuchSession);
        }

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
        if locs.len() > session.max_points {
            locs.pop_front();
            session.added_to_expiration = false;
        }

        State::add_data_expiration(
            &mut self.data_expirations,
            session,
            &mut self.data_expire_added,
        );

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

    // If there is more than one point of data in session, and we haven't added it to
    // data_expirations, do that, and notfiy the expiration task we've done it.
    fn add_data_expiration(
        data_expirations: &mut Expiration,
        session: &mut Session,
        notify: &mut Arc<Notify>,
    ) {
        if let Some(max_point_age) = session.max_point_age {
            if !session.added_to_expiration {
                if let Some(front) = session.locations.front() {
                    if !session.locations.is_empty() {
                        let expiration = front.time_timedelta() + max_point_age;
                        log::debug!(
                            "Added data to be expired for session {}: {}+{}={}",
                            &session.session_id,
                            front.time_timedelta(),
                            &max_point_age,
                            &expiration
                        );
                        data_expirations.push(Reverse((expiration, session.session_id.clone())));
                        session.added_to_expiration = true;
                        notify.notify_waiters();
                    }
                }
            }
        }
    }
}

fn get_sleep_duration(expirations: &Expiration) -> Duration {
    if let Some(Reverse((expires_at, _))) = expirations.peek() {
        let now = Utc::now();

        expires_at
            .signed_duration_since(now)
            .to_std()
            // Handle the extremely unlikely error case reasonably?
            .unwrap_or(Duration::from_secs(1))
    } else {
        Duration::from_secs(3600)
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
        let fetches: HashMap<models::FetchId, models::Fetch> = state
            .sessions
            .iter()
            .filter_map(|x| {
                let session = &x.1;
                let shared_tags = &session.tags.as_tags() & &tags;
                if shared_tags.len() > 0 {
                    Some((
                        session.fetch_id,
                        models::Fetch {
                            tags: shared_tags,
                            max_points: session.max_points,
                            max_point_age: session.max_point_age.map(|x| x.num_seconds() as f64),
                        },
                    ))
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
            fetches,
            public: state
                .public_tags
                .iter()
                .map(|(tag, _)| tag.clone())
                .collect(),
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

    pub fn update_subscribed_tags(update: &Update, subscribed_tags: &mut models::Tags) {
        for change in update.changes.iter() {
            if let models::UpdateChange::AddFetch { public, .. } = change {
                subscribed_tags.merge(public);
            }
        }
    }

    pub async fn updates(
        &self,
        state: &State,
        tags: models::Tags,
        auto_subscribe: bool, // automatically subscribe to new public tags
    ) -> Pin<Box<dyn futures_util::stream::Stream<Item = UpdateBroadcast>>> {
        let updates = tokio_stream::wrappers::BroadcastStream::new(self.updates_tx.subscribe());

        let initial_message = self.initial_update(tags.clone(), state);
        let first_stream = futures_util::stream::once(async move {
            UpdateBroadcast::Ok(UpdateWithContext::from(initial_message))
        });

        let subscribed_tags = Arc::new(Mutex::new(tags));

        // Filter our messages this subscription doesn't see
        // Modify tags so that the client doesn't learn about new tags
        // Also bake the UpdateContext inside each UpdateChange with UpdateChangeWithContext, so it becomes possible to merge with other UpdateChanges
        let updates = {
            futures_util::StreamExt::filter_map(updates, move |x| {
                let subscribed_tags = subscribed_tags.clone();
                async move {
                    match x {
                        Ok((context, update)) => {
                            let mut subscribed_tags = subscribed_tags.lock().await;
                            if auto_subscribe {
                                Self::update_subscribed_tags(&update, &mut subscribed_tags);
                            }

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

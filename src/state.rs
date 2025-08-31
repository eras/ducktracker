use crate::models::{self, Location, Update, UpdateChange};
use std::pin::Pin;
use tokio::sync::broadcast;

pub enum Error {
    NoSuchSession,
    SessionExpired,
}

pub struct State {
    pub sessions: dashmap::DashMap<models::SessionId, models::Session>,
    pub updates: Updates,

    next_fetch_id: models::FetchId,
}

impl State {
    pub fn new(updates: Updates) -> Self {
        Self {
            updates,
            sessions: dashmap::DashMap::new(),
            next_fetch_id: models::FetchId(0u64),
        }
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

        let update = Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::Add { points }].to_vec(),
        };

        self.updates.send_update(update);

        Ok(())
    }
}

pub struct Updates {
    updates_tx: broadcast::Sender<Update>,
}

pub type UpdateBroadcast = Result<Update, tokio_stream::wrappers::errors::BroadcastStreamRecvError>;

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

    fn initial_update(&self, state: &State) -> Update {
        let points = state
            .sessions
            .iter()
            .map(|x| (x.value().fetch_id, x.value().locations.clone()))
            .collect();
        Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::Reset, UpdateChange::Add { points }].to_vec(),
        }
    }

    pub async fn updates(
        &self,
        state: &State,
    ) -> Pin<Box<dyn futures_util::stream::Stream<Item = UpdateBroadcast>>> {
        let updates = tokio_stream::wrappers::BroadcastStream::new(self.updates_tx.subscribe());

        let initial_message = self.initial_update(state);
        let first_stream =
            futures_util::stream::once(async move { UpdateBroadcast::Ok(initial_message) });

        let updates = futures_util::StreamExt::chain(first_stream, updates);

        Box::pin(updates)
    }
}

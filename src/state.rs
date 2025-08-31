use crate::models::{self, Location, Update, UpdateChange};
use std::pin::Pin;
use tokio::sync::broadcast;

pub enum Error {
    NoSuchSession,
    SessionExpired,
}

pub struct State {
    pub sessions: dashmap::DashMap<String, models::Session>,
    pub updates: Updates,
}

impl State {
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
        points.insert("hello".to_string(), [new_location].to_vec());

        let update = Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::Loc { points }].to_vec(),
        };

        self.updates.updates_tx.send(update).unwrap();

        Ok(())
    }
}

pub struct Updates {
    pub updates_tx: broadcast::Sender<Update>,
}

pub type UpdateBroadcast = Result<Update, tokio_stream::wrappers::errors::BroadcastStreamRecvError>;

impl Updates {
    fn initial_update(&self, state: &State) -> Update {
        let points = state
            .sessions
            .iter()
            .map(|x| (x.key().clone(), x.value().locations.clone()))
            .collect();
        Update {
            server_time: models::TimeUsec(std::time::SystemTime::now()),
            interval: 0u64,
            changes: [UpdateChange::Loc { points }].to_vec(),
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

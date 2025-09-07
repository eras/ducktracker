use std::collections::{HashMap, HashSet};

use crate::state;
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents the current location data for a session.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(as = "LocationTS")]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spd: Option<f64>,
    #[serde(rename = "prv")]
    pub provider: u64, // location provider, seems to be 0 or 1, probably coarse vs fine
    pub time: f64,
}

#[derive(TS)]
#[allow(dead_code)]
struct LocationTS([f64; 6]);

/// Represents a single tracking session. This data is stored in memory.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: SessionId,
    pub locations: Vec<Location>,
    pub expires_at: DateTime<Utc>,
    pub fetch_id: FetchId,
    pub tags: TagsAux,
}

// ========================
// API Request and Response Models
// ========================

/// Request body for the /api/create endpoint.
#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    #[serde(rename = "usr")]
    pub user: Option<String>,
    #[serde(rename = "pwd")]
    pub password: Option<String>,
    #[serde(rename = "mod")]
    pub mode: u64, // Something?
    #[serde(rename = "lid")]
    pub share_id: Option<String>, // Desired share id; actually list of tags to publish to
    #[serde(rename = "dur")]
    pub duration: u64, // In seconds
    #[serde(rename = "int")]
    pub interval: u64, // In seconds
}

/// Response body for the /api/create endpoint.
#[derive(Debug)]
pub struct CreateResponse {
    pub status: String,
    pub session_id: SessionId,
    pub share_link: String,
    pub share_id: ShareId,
}

impl CreateResponse {
    pub fn to_client(&self) -> String {
        return format!(
            "{}\n{}\n{}\n{}\n",
            self.status, self.session_id, self.share_link, self.share_id
        );
    }
}

/// Request body for the /api/stop.php endpoint.
#[derive(Debug, Deserialize)]
pub struct StopRequest {
    #[serde(rename = "sid")]
    pub session_id: SessionId,

    // We don't use it, but it's part of the protocol
    #[allow(dead_code)]
    #[serde(rename = "lid")]
    pub share_id: Option<String>,
}

#[derive(Debug)]
pub struct StopResponse {}

impl StopResponse {
    pub fn to_client(&self) -> String {
        return format!("OK\n",);
    }
}

/// Request body for the /api/post endpoint.
#[derive(Debug, Deserialize)]
pub struct PostRequest {
    #[serde(rename = "sid")]
    pub session_id: SessionId,
    #[serde(rename = "prv")]
    pub provider: Option<u64>,
    pub time: f64,
    #[serde(rename = "lat")]
    pub latitude: f64,
    #[serde(rename = "lon")]
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "acc")]
    pub accuracy: Option<f64>,
    #[serde(rename = "spd")]
    pub speed: Option<f64>,
}

#[derive(Debug)]
pub struct PostResponse {
    pub public_url: String,
    pub target_ids: Vec<String>,
}

impl PostResponse {
    pub fn to_client(&self) -> String {
        format!("OK\n{}?{}\n", self.public_url, self.target_ids.join(","))
    }
}

impl Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // NOTE to update the number of elements, if the structure would ever change
        let mut state = serializer.serialize_seq(Some(8))?;
        use serde::ser::SerializeSeq;
        state.serialize_element(&self.lat)?;
        state.serialize_element(&self.lon)?;
        state.serialize_element(&self.time)?;
        state.serialize_element(&self.spd)?;
        state.serialize_element(&self.acc)?;
        state.serialize_element(&self.provider)?;
        state.end()
    }
}

#[derive(Debug, Clone, TS)]
#[ts(as = "TimeUsecTS")]
pub struct TimeUsec(pub std::time::SystemTime);

#[derive(TS)]
#[allow(dead_code)]
struct TimeUsecTS(f64);

//impl TS for TimeUse// c {
//     type WithoutGenerics = Self;

//     type OptionInnerType = Self;

//     fn decl() -> String {
//         "type TimeUsec = number".to_string()
//     }

//     fn decl_concrete() -> String {
//         "type TimeUsec = number".to_string()
//     }

//     fn name() -> String {
//         "TimeUsec".to_string()
//     }

//     fn inline() -> String {
//         "number".to_string()
//     }

//     fn inline_flattened() -> String {
//         "number".to_string()
//     }
// }

// This doesn't work. Why? The expects fail.
// #[cfg(test)]
// mod tests {
//     #[test]
//     fn export_bindings_manual() {
//         use ts_rs::TS;
//         super::Location::export().expect("Failed to export type Location");
//         super::TimeUsec::export().expect("Failed to export type TimeUsec");
//     }
// }

// Given to each new publish session
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, TS)]
#[ts(export)]
pub struct SessionId(pub String);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

// Useless?
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, TS)]
#[ts(export)]
pub struct ShareId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, TS)]
#[ts(export)]
pub struct Tag(pub String);

impl Tag {
    pub fn new(tag: String) -> Self {
        Tag(tag)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Tags(pub HashSet<Tag>);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TagVisibility {
    Private,
    Public,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TagAux {
    pub name: Tag,
    pub visibility: TagVisibility,
}

impl TagAux {
    pub fn new(name: &str, visibility: TagVisibility) -> Self {
        TagAux {
            name: Tag::new(name.to_string()),
            visibility,
        }
    }

    pub fn as_tag(&self) -> Tag {
        self.name.clone()
    }

    pub fn is_public(&self) -> bool {
        self.visibility == TagVisibility::Public
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsAux(pub HashSet<TagAux>);

impl TagsAux {
    pub fn as_tags(&self) -> Tags {
        Tags(self.0.iter().map(|x| x.name.clone()).collect())
    }
}

impl Into<Tags> for TagsAux {
    fn into(self) -> Tags {
        Tags(self.0.into_iter().map(|tag_aux| tag_aux.as_tag()).collect())
    }
}

impl std::ops::BitAnd for &Tags {
    type Output = Tags;

    fn bitand(self, rhs: Self) -> Self::Output {
        Tags(&self.0 & &rhs.0)
    }
}

impl FromIterator<Tag> for Tags {
    fn from_iter<T: IntoIterator<Item = Tag>>(iter: T) -> Self {
        Tags(HashSet::from_iter(iter))
    }
}

impl Tags {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl TagsAux {
    pub fn from_share_id(share_id: &Option<String>) -> Self {
        match share_id {
            None => TagsAux(HashSet::new()), // TODO: in this case, use some configurable default tag
            Some(share_id) => {
                let mut tags = HashSet::new();
                let mut visibility = TagVisibility::Public;
                for field in share_id.split(",") {
                    if let Some((keyword, tag)) = field.split_once(':') {
                        let set_visibility = match keyword {
                            "pub" | "public" => Some(TagVisibility::Public),
                            "priv" | "private" => Some(TagVisibility::Private),
                            _ => None,
                        };
                        match set_visibility {
                            Some(set_visibility) => {
                                visibility = set_visibility;
                                tags.insert(TagAux::new(tag, visibility.clone()));
                            }
                            None => {
                                // Ignore further tags, there was an invalid keyord
                                break;
                            }
                        }
                    } else {
                        tags.insert(TagAux::new(field, visibility.clone()));
                    }
                }
                TagsAux(tags)
            }
        }
    }

    pub fn public_tags(&self) -> Tags {
        self.0
            .iter()
            .filter_map(|tag_aux| match tag_aux.visibility {
                TagVisibility::Private => None,
                TagVisibility::Public => Some(tag_aux.as_tag()),
            })
            .collect()
    }
}

impl std::str::FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

impl std::fmt::Display for ShareId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.0)
    }
}

// Id used when providing data back to clients
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, TS, Default)]
#[ts(export)]
pub struct FetchId(pub u32);

impl Serialize for TimeUsec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let epoch = self
            .0
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Expected unix time to be available")
            .as_secs_f64();

        serializer.serialize_u64((epoch * 1000000.0) as u64)
    }
}

#[derive(Debug, Serialize, Clone, TS)]
#[ts(export)]
pub struct Update {
    #[serde(rename = "serverTime")]
    pub server_time: TimeUsec,
    pub interval: u64,
    pub changes: Vec<UpdateChange>,
}

impl Update {
    pub async fn filter_map(
        self,
        filter_tags: &Tags,
        update_context: &state::UpdateContext,
    ) -> Option<Update> {
        let changes: Vec<_> = stream::iter(self.changes)
            .filter_map(|x| x.filter_map(filter_tags, update_context))
            .collect()
            .await;
        if changes.len() > 0 {
            Some(Update { changes, ..self })
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Clone, TS)]
#[ts(export)]
pub enum UpdateChange {
    // Reset all client state
    #[serde(rename = "reset")]
    Reset,
    #[serde(rename = "add_fetch")]
    AddFetch {
        // Only includes the tags the client has subscribed to
        tags: HashMap<FetchId, Tags>,

        // There are these new public tags
        public: Tags,
    },
    #[serde(rename = "add")]
    Add {
        points: HashMap<FetchId, Vec<Location>>,
    },
    #[serde(rename = "expire_fetch")]
    ExpireFetch { fetch_id: FetchId },
}

impl UpdateChange {
    // Filter the UpdateChange so that it includes only information relevant to a certain tag subscriptions
    // state is used to find out tags for sources referred by their fetch_ids
    async fn filter_map(
        self,
        filter_tags: &Tags,
        update_context: &state::UpdateContext,
    ) -> Option<UpdateChange> {
        match self {
            Self::Reset => Some(self.clone()),
            Self::AddFetch { tags, public } => {
                let tags = tags
                    .into_iter()
                    .filter_map(|(fetch_id, tags)| {
                        let shared_tags = &tags & &filter_tags;
                        if shared_tags.len() > 0 {
                            Some((fetch_id, shared_tags))
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(UpdateChange::AddFetch {
                    tags,
                    public: public.clone(),
                })
            }
            Self::Add { points } => {
                let points = stream::iter(points)
                    .filter_map(|(fetch_id, locations)| async move {
                        let shared_tags = &update_context.tags & filter_tags;
                        if shared_tags.len() > 0 {
                            Some((fetch_id, locations))
                        } else {
                            None
                        }
                    })
                    .collect()
                    .await;
                Some(UpdateChange::Add { points })
            }
            Self::ExpireFetch { fetch_id } => {
                let shared_tags = &update_context.tags & filter_tags;
                if shared_tags.len() > 0 {
                    Some(UpdateChange::ExpireFetch { fetch_id })
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LoginResponse {
    pub token: String,
}

/// Request body for the /api/stream endpoint.
#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    // No tags -> use all public tags
    #[serde(default = "crate::utils::CommaSeparatedVec::new")]
    pub tags: crate::utils::CommaSeparatedVec<Tag>,
    pub token: String,
}

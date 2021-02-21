use chrono::{DateTime, offset::Utc};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct PostPollRequest {
    pub name: String,
    pub description: Option<String>,
    pub configuration: Configuration,
    pub candidates: Vec<Candidate>,
}

#[derive(Serialize, Deserialize)]
pub struct GetPollResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub expires: DateTime<Utc>,
    pub close: Option<DateTime<Utc>>,
    pub candidates: Vec<Candidate>,
    pub ballots: Vec<BallotSummary>,
    pub configuration: Configuration,
}

#[derive(Serialize, Deserialize)]
pub struct BallotSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub name: Arc<String>,
    pub rankings: Vec<Arc<String>>,
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(Clone))]
pub struct Candidate {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(Clone))]
pub struct Configuration {
    pub write_ins: bool
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone))]
pub struct PostPollResponse {
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone))]
pub struct PutBallotRequest {
    pub name: String,
    pub rankings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone))]
pub enum Identity {
    SecretKey(String),
}

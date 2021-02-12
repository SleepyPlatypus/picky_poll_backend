use chrono::{DateTime, offset::Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[cfg_attr(test, derive(Clone))]
pub struct PostPollRequest {
    pub name: String,
    pub description: String,
    pub candidates: Vec<Candidate>,
}

#[derive(Serialize, Deserialize)]
pub struct GetPollResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub expires: DateTime<Utc>,
    pub close: Option<DateTime<Utc>>,
    pub candidates: Vec<Candidate>,
}

#[derive(Serialize, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(Clone))]
pub struct Candidate {
    pub name: String,
    pub description: Option<String>,
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
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone))]
pub enum Identity {
    SecretKey(String),
}

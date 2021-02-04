use chrono::{DateTime, offset::Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct PostPollRequest {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetPollResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub expires: DateTime<Utc>,
    pub close: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PostPollResponse {
    pub id: String,
}

pub enum Identity {
    SecretKey(String),
}

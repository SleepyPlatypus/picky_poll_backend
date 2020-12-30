mod operations;
mod paths;

use chrono::{Duration, offset::Utc, DateTime};
use serde::{Deserialize, Serialize};
use super::db;

#[derive(Clone, Deserialize, Serialize)]
struct PostPollRequest {
    name: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct GetPollResponse {
    id: String,
    name: String,
    description: String,
    expires: DateTime<Utc>,
    close: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PostPollResponse {
    id: String,
}

enum Identity {
    SecretKey(String),
}

#[derive(Debug)]
enum PostPollError {
    Conflict,
    Error(sqlx::Error),
}

impl From<db::PutPollErr> for PostPollError {
    fn from(e : db::PutPollErr) -> Self {
        match e {
            db::PutPollErr::PostgresErr(e) => PostPollError::Error(e),
            db::PutPollErr::Conflict => PostPollError::Conflict,
        }
    }
}

#[derive(Debug)]
enum GetPollError {
    NotFound,
    Error(sqlx::Error),
}

impl From<db::GetPollErr> for GetPollError {
    fn from(e: db::GetPollErr) -> Self {
        match e {
            db::GetPollErr::PostgresErr(e) => GetPollError::Error(e),
            db::GetPollErr::NotFound => GetPollError::NotFound,
        }
    }
}

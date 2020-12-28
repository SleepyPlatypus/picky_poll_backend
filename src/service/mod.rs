mod operations;
// mod paths;

use chrono::{Duration, offset::Utc, DateTime};
use super::db;

#[derive(Clone)]
struct PostPollRequest {
    name: String,
    description: String,
}

struct GetPollResponse {
    id: String,
    name: String,
    description: String,
    expires: DateTime<Utc>,
    close: Option<DateTime<Utc>>,
}

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

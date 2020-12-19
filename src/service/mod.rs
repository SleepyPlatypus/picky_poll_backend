use chrono::{
    DateTime,
    Duration,
    offset::Utc,
};
use rand::{
    distributions::Alphanumeric,
    thread_rng,
    Rng
};
use sqlx::{Connection, Executor};
use super::db::{
    self,
    VoteDb,
};
use chrono::format::Numeric::Timestamp;
use std::ops::Add;

struct PostPollRequest {
    name: String,
    description: String,
}

struct PostPollResponse {
    id: String,
}

enum Identity {
    SecretKey(String),
}

struct PollService<'d, 'e, E>
    where E: Executor<'e> {
    db: &'d VoteDb<'e, E>,
}

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

impl<'d, 'e, E> PollService<'d, 'e, E> where E: Executor<'e, Database=sqlx::Postgres> + Copy {
    async fn post_poll(&self, id: Identity, request: PostPollRequest)
        -> Result<PostPollResponse, PostPollError>
    {
        let poll_id = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let owner_id_str = match id { Identity::SecretKey(s) => s};
        let expires = Utc::now() + Duration::days(7);

        let row = db::Poll {
            id: poll_id,
            name: request.name,
            description: request.description,
            owner_id: owner_id_str,
            expires,
            close: None
        };

        let casted: &VoteDb<'e, E> = self.db;
        casted.put_poll(&row).await?;

        Ok(PostPollResponse {id: row.id})
    }
}

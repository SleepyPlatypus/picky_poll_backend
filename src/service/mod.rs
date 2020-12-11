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
use super::db::{
    self,
    VoteDbClient,
};
struct PostPollRequest {
    name: String,
    description: String,
    expires: DateTime<Utc>,
    close: DateTime<Utc>,
}

struct PollService {
    db: VoteDbClient,
}

enum PostPollError {
    Conflict,
    Error(tokio_postgres::Error),
}

impl From<db::PutPollErr> for PostPollError {
    fn from(e : db::PutPollErr) -> Self {
        match e {
            db::PutPollErr::PostgresErr(e) => PostPollError::Error(e),
            db::PutPollErr::Conflict => PostPollError::Conflict,
        }
    }
}

impl PollService {
    async fn post_poll(&self, request: PostPollRequest, id: String) -> Result<String, PostPollError> {
        let poll_row = db::Poll {
            id: thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
            name: request.name,
            description: request.description,
            owner_id: id,
            expires: Utc::now() + Duration::days(7),
            close: None,
        };
        self.db.put_poll(&poll_row).await?;
        Ok(poll_row.id)
    }
}
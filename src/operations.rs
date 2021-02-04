use crate::model::*;
use crate::db::{
    self,
    PickyDb,
};
use async_trait::async_trait;
use rand::{
    distributions::Alphanumeric,
    Rng,
    thread_rng
};
use mockall::automock;
use chrono::{Utc, Duration};

#[derive(Debug)]
pub enum PostPollError {
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
pub enum GetPollError {
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

#[derive(Clone)]
pub struct PollOperationsImpl {
    db: PickyDb,
}

impl PollOperationsImpl {
    pub fn new(db: PickyDb) -> PollOperationsImpl {
        PollOperationsImpl {
            db
        }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait PollOperations {
    async fn post_poll(&self, user: &Identity, request: PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>;
    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError>;
}

#[async_trait]
impl PollOperations for PollOperationsImpl {

    async fn post_poll(&self, user: &Identity, request: PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>
    {
        let poll_id = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let Identity::SecretKey(owner_id_str) = user;
        let expires = Utc::now() + Duration::days(7);

        let row = db::Poll {
            id: poll_id,
            name: request.name,
            description: request.description,
            owner_id: owner_id_str.clone(),
            expires,
            close: None
        };

        self.db.insert_poll(&row).await?;

        Ok(PostPollResponse {id: row.id})
    }

    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError> {
        let row: db::Poll = self.db.select_poll(id).await?;

        Ok(GetPollResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            expires: row.expires,
            close: row.close,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use sqlx::postgres::{
        PgPoolOptions,
    };
    use crate::db::PickyDb;
    use super::*;

    const DATABASE_URL: &str = "PICKYPOLL_TEST_DB";

    #[tokio::test]
    async fn test_post_poll() {
        let db_url = &env::var(&DATABASE_URL).unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .unwrap();

        let client = PickyDb::new(pool);
        let service = PollOperationsImpl {db: client};

        let mock_user = Identity::SecretKey("test user".to_string());

        let post_poll_request = PostPollRequest {
            name: "test poll name".to_string(),
            description: "test poll description".to_string(),
        };
        let post_poll_response = service
            .post_poll(&mock_user, post_poll_request.clone())
            .await
            .unwrap();

        let get_poll_response = service
            .get_poll(post_poll_response.id.as_str())
            .await
            .unwrap();

        assert_eq!(post_poll_request.name, get_poll_response.name)
    }
}
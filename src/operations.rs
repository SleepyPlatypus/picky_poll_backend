use crate::model::*;
use crate::db::{
    self,
    PickyDb,
};
use async_trait::async_trait;
use chrono::{Utc, Duration};
use futures::join;
use rand::{
    distributions::Alphanumeric,
    Rng,
    thread_rng
};

#[cfg(test)]
use mockall::automock;

#[derive(Debug)]
pub enum PostPollError {
    Conflict,
    DuplicateCandidate,
    Error(sqlx::Error),
}

impl From<db::InsertPollErr> for PostPollError {
    fn from(e : db::InsertPollErr) -> Self {
        match e {
            db::InsertPollErr::PostgresErr(e) => PostPollError::Error(e),
            db::InsertPollErr::Conflict => PostPollError::Conflict,
        }
    }
}

#[derive(Debug)]
pub enum GetPollError {
    NotFound,
    Error(sqlx::Error),
}

impl From<db::SelectPollErr> for GetPollError {
    fn from(e: db::SelectPollErr) -> Self {
        match e {
            db::SelectPollErr::PostgresErr(e) => GetPollError::Error(e),
            db::SelectPollErr::NotFound => GetPollError::NotFound,
        }
    }
}

#[derive(Debug)]
pub enum PutBallotError {
    PollNotFound,
    NotOwner,
    NotSameName,
    Error(sqlx::Error),
}

impl PutBallotError {
    fn sqlx(e: sqlx::Error) -> PutBallotError {
        PutBallotError::Error(e)
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
    async fn post_poll(&self, user: Identity, request: PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>;
    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError>;
    async fn put_ballot(&self,
        poll_id: &str,
        user_id: Identity,
        ballot_id: String,
        ballot: PutBallotRequest) -> Result<(), PutBallotError>;
}

#[async_trait]
impl PollOperations for PollOperationsImpl {

    async fn post_poll(&self, user: Identity, request: PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>
    {
        let poll_id: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let Identity::SecretKey(owner_id_str) = user;
        let expires = Utc::now() + Duration::days(7);

        let row = db::Poll {
            id: poll_id.clone(),
            name: request.name,
            description: request.description,
            owner_id: owner_id_str,
            expires,
            close: None
        };

        let candidate_rows = request.candidates
            .into_iter()
            .map(move |req_c| db::Candidate {
                name: req_c.name,
                description: req_c.description,
            }).collect();

        self.db.insert_poll(&row).await?;
        self.db.insert_candidates(&poll_id, &candidate_rows)
            .await
            .map_err(|e| match e {
                db::InsertCandidateErr::Conflict => PostPollError::DuplicateCandidate,
                db::InsertCandidateErr::PostgresErr(e) => PostPollError::Error(e)
            })?;

        Ok(PostPollResponse {id: row.id})
    }

    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError> {
        let poll = self.db.select_poll(id);
        let candidates = self.db.select_candidates(id);
        let (poll, candidates) = join!(poll, candidates);
        let poll = poll?;

        let candidates = candidates.map_err(|e| {
            GetPollError::Error(e)
        })?
        .into_iter()
        .map(|row| Candidate{
            name: row.name,
            description: row.description
        })
        .collect();

        Ok(GetPollResponse {
            id: poll.id,
            name: poll.name,
            description: poll.description,
            expires: poll.expires,
            close: poll.close,
            candidates,
        })
    }

    async fn put_ballot(&self,
        poll_id: &str,
        user_id: Identity,
        ballot_id: String,
        ballot: PutBallotRequest) -> Result<(), PutBallotError> {
        let Identity::SecretKey(owner_id) = user_id;
        let db_ballot = db::Ballot {
            id: ballot_id,
            name: ballot.name,
            timestamp: Utc::now(),
            owner_id,
        };

        self.db.upsert_ballot(poll_id, db_ballot)
            .await
            .map_err(|db_e| match db_e {
                db::UpsertBallotErr::PollNotFound => PutBallotError::PollNotFound,
                db::UpsertBallotErr::NotSameName => PutBallotError::NotSameName,
                db::UpsertBallotErr::NotOwner => PutBallotError::NotOwner,
                db::UpsertBallotErr::PostgresErr(e) => PutBallotError::Error(e)
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::db::PickyDb;
    use super::db::test_db;
    use super::*;

    #[tokio::test]
    async fn test_post_poll() {
        let client = PickyDb::new(test_db::new_pool().await);
        let service = PollOperationsImpl {db: client};

        let mock_user = Identity::SecretKey("test user".to_string());

        let post_poll_request = PostPollRequest {
            name: "test poll name".to_owned(),
            description: "test poll description".to_owned(),
            candidates: vec!(
                Candidate{
                    name: "candidate".to_owned(),
                    description: Some("candidate description".to_owned()),
                }
            ),
        };
        let post_poll_response = service
            .post_poll(mock_user, post_poll_request.clone())
            .await
            .unwrap();

        let get_poll_response = service
            .get_poll(&post_poll_response.id)
            .await
            .unwrap();

        assert_eq!(post_poll_request.name, get_poll_response.name);
        
        let mut request_candidates = post_poll_request.candidates.clone();
        request_candidates.sort_by_key(|c|c.name.clone());
        let mut response_candidates = get_poll_response.candidates.clone();
        response_candidates.sort_by_key(|c|c.name.clone());
        assert_eq!(post_poll_request.candidates, response_candidates);
    }
}
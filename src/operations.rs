use crate::{model::*, util};
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

#[cfg(test)]
use mockall::automock;

#[derive(Debug)]
pub enum PostPollError {
    Conflict,
    DuplicateCandidate(String),
    Unexpected,
}

#[derive(Debug)]
pub enum GetPollError {
    NotFound,
    Unexpected,
}

#[derive(Debug)]
pub enum PutBallotError {
    CandidateNotFound(String),
    DuplicateRanking(String),
    PollNotFound,
    NotOwner,
    NotSameName,
    Unexpected,
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
    async fn post_poll(&self, user: &Identity, request: &PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>;
    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError>;
    async fn put_ballot(&self,
        poll_id: &str,
        user_id: &Identity,
        ballot_id: &str,
        ballot: &PutBallotRequest) -> Result<(), PutBallotError>;
}

#[async_trait]
impl PollOperations for PollOperationsImpl {

    async fn post_poll(&self, user: &Identity, request: &PostPollRequest)
                       -> Result<PostPollResponse, PostPollError>
    {
        let poll_id: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();

        let duplicate_candidate = util::first_duplicate(
            request.candidates.iter().map(|c| &c.name)
        );
        if let Some(duplicate_candidate) = duplicate_candidate {
            return Err(PostPollError::DuplicateCandidate(duplicate_candidate.clone()));
        }

        self.db.insert_poll(&poll_id, &user, &request)
            .await
            .map_err(|e| match e {
                db::InsertPollErr::Conflict => PostPollError::Unexpected,
                db::InsertPollErr::Unexpected => PostPollError::Unexpected,
            })?;

        Ok(PostPollResponse {id: poll_id})
    }

    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError> {
        self.db.select_poll(id).await
        .map_err(|e| match e {
            db::SelectPollErr::PollNotFound => GetPollError::NotFound,
            db::SelectPollErr::Unexpected => GetPollError::Unexpected,
        })
    }

    async fn put_ballot(&self,
        poll_id: &str,
        user_id: &Identity,
        ballot_id: &str,
        ballot: &PutBallotRequest
    ) -> Result<(), PutBallotError> {

        let duplicate = util::first_duplicate(ballot.rankings.iter());
        if let Some(duplicate) = duplicate {
            return Err(PutBallotError::DuplicateRanking(duplicate.clone()));
        }

        self.db.upsert_ballot(poll_id, ballot_id, &user_id, ballot)
            .await
            .map_err(|db_e| match db_e {
                db::UpsertBallotErr::CandidateNotFound(candidate_name) =>
                    PutBallotError::CandidateNotFound(candidate_name),
                db::UpsertBallotErr::PollNotFound => PutBallotError::PollNotFound,
                db::UpsertBallotErr::NotSameName => PutBallotError::NotSameName,
                db::UpsertBallotErr::NotOwner => PutBallotError::NotOwner,
                db::UpsertBallotErr::Unexpected => PutBallotError::Unexpected,
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
            .post_poll(&mock_user, &post_poll_request)
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
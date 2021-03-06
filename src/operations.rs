use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::{model::*, util};
use crate::db::{
    self,
    PickyDb,
    PickyPollTransaction,
};
use chrono::{Duration, Utc};
use itertools::Itertools;
use rand::{
    distributions::Alphanumeric,
    Rng,
    thread_rng
};

#[cfg(test)]
use mockall::automock;

#[derive(Debug)]
pub enum PostPollError {
    DuplicateCandidate(String),
    Unexpected,
}

impl From<sqlx::Error> for PostPollError {
    fn from(e: sqlx::Error) -> Self {
        log_sql_error(e);
        Self::Unexpected
    }
}

#[derive(Debug)]
pub enum PostCandidateError {
    PollNotFound,
    NoWriteIns,
    DuplicateCandidate(String),
    Unexpected,
}

impl From<sqlx::Error> for PostCandidateError {
    fn from(e: sqlx::Error) -> Self {
        log_sql_error(e);
        Self::Unexpected
    }
}

#[derive(Debug)]
pub enum GetPollError {
    NotFound,
    Unexpected,
}

impl From<sqlx::Error> for GetPollError {
    fn from(e: sqlx::Error) -> Self {
        log_sql_error(e);
        Self::Unexpected
    }
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

impl From<sqlx::Error> for PutBallotError {
    fn from(e: sqlx::Error) -> Self {
        log_sql_error(e);
        Self::Unexpected
    }
}

fn log_sql_error(e: sqlx::Error) {
    error!("unexpected sql error: {:?}", e);
    if let Some(e) = e.into_database_error() {
        error!("{}", e.message())
    };
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait PollOperationsT {
    async fn post_poll(&self, identity: &Identity, request: &PostPollRequest) -> Result<PostPollResponse, PostPollError>;
    async fn post_candidate(&self, poll_id: &str, request: &Candidate) -> Result<(), PostCandidateError>;
    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError>;
    async fn put_ballot(&self,
        poll_id: &str,
        user_id: &Identity,
        ballot_id: &str,
        request: &PutBallotRequest
    ) -> Result<(), PutBallotError>;
    async fn insert_rankings<'a>(&self,
        tx: &mut PickyPollTransaction<'a>,
        poll_id: &str,
        ballot_id: &str,
        rankings: &[String]
    ) -> Result<(), PutBallotError>;
}

#[derive(Clone)]
pub struct PollOperations {
    db: PickyDb,
}

impl PollOperations {
    pub fn new(db: PickyDb) -> PollOperations {
        PollOperations {
            db
        }
    }
}

#[async_trait]
impl PollOperationsT for PollOperations {

    async fn post_poll(&self, identity: &Identity, request: &PostPollRequest)
    -> Result<PostPollResponse, PostPollError> {
        if let Some(duplicate) = util::first_duplicate(request.candidates.iter().map(|c| &c.name)) {
            return Err(PostPollError::DuplicateCandidate(duplicate.clone()));
        }

        let poll_id: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .collect();

        let Identity::SecretKey(owner_id) = identity;

        let mut transaction = self.db.new_transaction().await?;

        let poll = db::Poll {
            id: poll_id.to_owned(),
            name: request.name.clone(),
            description: request.description.clone(),
            owner_id: owner_id.clone(),
            expires: Utc::now() + Duration::days(7),
            close: None,
            write_ins: request.configuration.write_ins,
        };

        transaction.insert_poll(&poll).await?;

        for c in request.candidates.iter() {
            transaction.insert_candidate(&poll_id, &c.name, &c.description).await?;
        }

        transaction.commit().await?;

        Ok(PostPollResponse {
            poll: Poll {
                id: poll_id,
                name: request.name.clone(),
                description: request.description.clone(),
                expires: poll.expires,
                close: None,
                configuration: request.configuration.clone(),
                candidates: request.candidates.clone(),
            }
        })
    }

    async fn post_candidate(&self, poll_id: &str, request: &Candidate) -> Result<(), PostCandidateError> {
        let mut transaction = self.db.new_transaction()
        .await?;

        let poll = transaction.select_poll(poll_id)
        .await?
        .ok_or(PostCandidateError::PollNotFound)?;

        if !poll.write_ins {
            return Err(PostCandidateError::NoWriteIns);
        }

        let mut existing_candidates = transaction.select_candidates(&poll.id)
        .await?
        .into_iter()
        .map(|c| c.name);

        if let Some(_) = existing_candidates.find(|e| e == &request.name) {
            return Err(PostCandidateError::DuplicateCandidate(request.name.clone()));
        }

        transaction.insert_candidate(&poll_id, &request.name, &request.description).await?;
        transaction.commit().await?;
        
        Ok(())
    }

    async fn get_poll(&self, id: &str) -> Result<GetPollResponse, GetPollError> {
        let mut transaction = self.db.new_transaction()
        .await?;

        let poll = transaction.select_poll(id)
            .await?
            .ok_or(GetPollError::NotFound)?;

        let candidates = transaction.select_candidates(id)
        .await?;
        let ballots = transaction.select_ballots(id)
        .await?;
        let rankings = transaction.select_rankings(id)
        .await?;

        let candidate_id_to_name: HashMap<i32, Arc<String>> = candidates.iter()
        .map(|c| (c.id, Arc::new(c.name.clone())))
        .collect();

        let mut rankings_by_ballot_id: HashMap<String, Vec<db::Ranking>> = rankings
        .into_iter()
        .into_group_map_by(|r| r.ballot_id.clone());
        
        let ballots = ballots.into_iter()
        .map(|b| {
            let mut local_rankings = rankings_by_ballot_id
            .remove(b.id.as_str())
            .unwrap_or_default();
            local_rankings.sort_by_key(|r| r.ranking);
            let local_rankings = local_rankings
            .into_iter()
            .flat_map(|r| {
                candidate_id_to_name
                .get(&r.candidate_id)
                .map(|arc| arc.clone())
                .or_else(|| {
                    error!("Candidate not found for ballot_id={},candidate_id={}", &r.ballot_id, r.candidate_id);
                    None
                })
            })
            .collect();
            BallotSummary {
                id: b.id,
                name: Arc::new(b.name),
                timestamp: b.timestamp,
                rankings: local_rankings,
            }
        }).collect();
        
        let candidates = candidates.into_iter()
        .map(|c| Candidate {
            name: c.name,
            description: c.description,
        })
        .collect();

        Ok(GetPollResponse {
            poll: Poll {
                id: poll.id,
                name: poll.name,
                description: poll.description,
                candidates,
                expires: poll.expires,
                close: poll.close,
                configuration: Configuration {
                    write_ins: poll.write_ins,
                }
            },
            ballots,
        })
    }

    async fn put_ballot(&self,
        poll_id: &str,
        user_id: &Identity,
        ballot_id: &str,
        request: &PutBallotRequest
    ) -> Result<(), PutBallotError> {

        if let Some(duplicate) = util::first_duplicate(request.rankings.iter()) {
            return Err(PutBallotError::DuplicateRanking(duplicate.clone()));
        }

        let Identity::SecretKey(owner_id) = user_id;

        let mut tx = self.db.new_transaction().await?;

        /*poll exists?*/ tx.select_poll(poll_id).await?
        .ok_or(PutBallotError::PollNotFound)?;

        let previous_row = tx.select_ballot(poll_id, ballot_id)
        .await?;

        let ballot = db::Ballot {
            id: String::from(ballot_id),
            name: request.name.clone(),
            timestamp: Utc::now(),
            owner_id: String::from(owner_id)
        };
        
        match previous_row {
            None => {
                tx.insert_ballot(poll_id, &ballot)
                .await?
            },
            Some(previous_row) => {
                if previous_row.owner_id != ballot.owner_id {
                    return Err(PutBallotError::NotOwner);
                } else if previous_row.name != ballot.name {
                    return Err(PutBallotError::NotSameName);
                } else {
                    tx.update_ballot(poll_id, &ballot)
                    .await?
                }
            },
        };

        tx.delete_rankings(poll_id, &ballot.id).await?;
        self.insert_rankings(&mut tx, poll_id, &ballot.id, &request.rankings).await?;
        tx.commit().await?;
        Ok(())
    }


    async fn insert_rankings<'a>(&self,
        tx: &mut PickyPollTransaction<'a>,
        poll_id: &str,
        ballot_id: &str,
        rankings: &[String]) -> Result<(), PutBallotError>
    {
        let candidates = tx.select_candidates(poll_id)
        .await?;

        let mut candidate_name_to_id: HashMap<String, i32> = candidates
            .into_iter()
            .map(|c| (c.name, c.id))
            .collect();
        
        for (i, candidate_name) in rankings.iter().enumerate() {
            let candidate_id = candidate_name_to_id
                .remove(candidate_name)
                .ok_or_else(|| PutBallotError::CandidateNotFound(candidate_name.clone()))?;
            let row = db::Ranking {
                poll_id: String::from(poll_id),
                ballot_id: String::from(ballot_id),
                candidate_id,
                ranking: i as i16,
            };
            tx.insert_ranking(poll_id, &row)
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::PickyDb;
    use super::db::test_db;
    use super::*;

    #[tokio::test]
    async fn test_post_poll() {
        let db = PickyDb::new(test_db::new_pool().await);
        let service = PollOperations::new(db);

        let mock_user = Identity::SecretKey("test user".to_string());

        let post_poll_request = PostPollRequest {
            name: "test poll name".to_owned(),
            description: Some("test poll description".to_owned()),
            candidates: vec!(
                Candidate{
                    name: "candidate".to_owned(),
                    description: Some("candidate description".to_owned()),
                }
            ),
            configuration: Configuration {
                write_ins: false,
            },
        };
        let post_poll_response = service
            .post_poll(&mock_user, &post_poll_request)
            .await
            .unwrap();

        let get_poll_response = service
            .get_poll(&post_poll_response.id)
            .await
            .unwrap();

        assert_eq!(post_poll_request.name, get_poll_response.poll.name);
        
        let mut request_candidates = post_poll_request.candidates.clone();
        request_candidates.sort_by_key(|c|c.name.clone());
        let mut response_candidates = get_poll_response.poll.candidates.clone();
        response_candidates.sort_by_key(|c|c.name.clone());
        assert_eq!(post_poll_request.candidates, response_candidates);
    }

    mod test_put_ballot {
        use super::*;
        
        async fn post_mock_poll(ops: &PollOperations) -> String {
            
            ops.post_poll(
                &Identity::SecretKey("secret".to_string()),
                &PostPollRequest{
                    name: "Dessert".to_string(),
                    description: Some("What dessert should be served?".to_string()),
                    candidates: vec!(
                        Candidate{name: "cookies".to_string(), description: None},
                        Candidate{name: "cake".to_string(), description: None},
                        Candidate{name: "ice cream".to_string(), description: None},
                    ),
                    configuration: Configuration {
                        write_ins: false,
                    },
                },
            ).await
            .expect("Should post poll")
            .id
        }

        #[tokio::test]
        async fn happy_path() {
            let db = PickyDb::new(test_db::new_pool().await);
            let ops = PollOperations::new(db);

            //given a poll
            let mock_poll_id = post_mock_poll(&ops).await;

            //when mock_identity puts a ballot
            let mock_identity = Identity::SecretKey("mock user".to_string());
            let mock_ballot_id = "mock_ballot_id";
            let mock_request = PutBallotRequest {
                name: "mock username".to_string(),
                rankings: vec!(
                    "cake".to_string(),
                    "cookies".to_string(),
                )
            };
            ops.put_ballot(&mock_poll_id, &mock_identity, &mock_ballot_id, &mock_request).await
            .expect("put ballot should succeed");

            //and we get the poll back
            let get_poll_response = ops.get_poll(&mock_poll_id)
            .await
            .expect("get poll should succeed");
            //then the poll should contain the mock ballot
            let ballot = get_poll_response.ballots
            .first()
            .expect("poll should have ballot");

            assert_eq!(ballot.name.as_ref(), &mock_request.name);
            assert_eq!(&ballot.rankings.iter().map(|r| (**r).clone()).collect::<Vec<String>>(), &mock_request.rankings)
        }

        #[tokio::test]
        async fn replace_ballot() {
            let db = PickyDb::new(test_db::new_pool().await);
            let ops = PollOperations::new(db);

            //given a poll
            let mock_poll_id = post_mock_poll(&ops).await;

            //when mock_identity puts a ballot
            let mock_identity = Identity::SecretKey("mock user".to_string());
            let mock_ballot_id = "mock_ballot_id";
            let mut mock_request = PutBallotRequest {
                name: "mock username".to_string(),
                rankings: vec!(
                    "cake".to_string(),
                    "cookies".to_string(),
                )
            };
            ops.put_ballot(&mock_poll_id, &mock_identity, &mock_ballot_id, &mock_request).await
            .expect("put ballot should succeed");

            //and mock_identity replaces the ballot with different rankings
            mock_request.rankings.reverse();
            ops.put_ballot(&mock_poll_id, &mock_identity, &mock_ballot_id, &mock_request).await
            .expect("put ballot should succeed");

            //then the poll should contain the updated rankings
            let get_poll_response = ops.get_poll(&mock_poll_id)
            .await
            .expect("get poll should succeed");

            let ballot = get_poll_response.ballots
            .first()
            .expect("poll should have ballot");

            assert_eq!(ballot.name.as_ref(), &mock_request.name);
            assert_eq!(&ballot.rankings.iter().map(|r| (**r).clone()).collect::<Vec<String>>(), &mock_request.rankings)
        }
    }
}
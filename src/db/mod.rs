use std::{collections::HashMap, intrinsics::transmute};
use std::sync::Arc;
use itertools::Itertools;
use chrono::{
    DateTime,
    offset::Utc,
};
use model::PostPollRequest;
use sqlx::{Executor, PgPool, Postgres, Transaction, postgres::PgDone};

use crate::{model::{self, BallotSummary, GetPollResponse}, operations};

#[derive(Clone)]
pub struct PickyDb {
    pool: PgPool
}

type Timestamp = DateTime<Utc>;

#[derive(sqlx::FromRow, Debug, Eq, PartialEq)]
pub struct Poll {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub expires: Timestamp,
    pub close: Option<Timestamp>,
}

#[derive(sqlx::FromRow, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct Candidate {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(sqlx::FromRow, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct Ballot {
    pub id: String,
    pub name: String,
    pub timestamp: Timestamp,
    pub owner_id: String,
}

#[derive(sqlx::FromRow, Debug, Eq, PartialEq)]
pub struct Ranking {
    pub ballot_id: String,
    pub poll_id: String,
    pub candidate_id: i32,
    pub ranking: i16,
}

#[derive(Debug)]
pub enum InsertPollErr {
    Unexpected,
    Conflict,
}

#[derive(Debug)]
pub enum InsertCandidateErr {
    Conflict,
    PostgresErr(sqlx::Error),
}

#[derive(Debug)]
pub enum UpsertBallotErr {
    NotSameName,
    NotOwner,
    PollNotFound,
    CandidateNotFound(String),
    Unexpected,
}

// #[derive(Debug, Eq, PartialEq)]
// pub struct BallotSummary {
//     pub id: String,
//     pub name: String,
//     pub timestamp: Timestamp,
//     pub rankings: Vec<Arc<String>>,
// }


fn log_error(e: sqlx::Error) {
    error!("sqlx error {:?}", e);
}

pub enum SelectPollErr {
    PollNotFound,
    Unexpected,
}

struct PickyPollTransaction<'a>{
    tx: Transaction<'a, Postgres>,
}

impl<'a> PickyPollTransaction<'a> {
    async fn select_ballot(&mut self, poll_id: &str, ballot_id: &str)
    -> Result<Option<Ballot>, sqlx::Error> {
    
        sqlx::query_as(
            "select id, name, timestamp, owner_id from ballot where poll_id = $1"
        ).bind(poll_id)
        .fetch_optional(&mut self.tx)
        .await
    }

    async fn select_ballots(&mut self, poll_id: &str)
    -> Result<Vec<Ballot>, sqlx::Error> {
    
        sqlx::query_as(
            "select id, name, timestamp, owner_id from ballot where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await
    }

    async fn insert_ballot(&mut self, poll_id: &str, ballot: &Ballot)
    -> Result<PgDone, sqlx::Error> {
        let query = sqlx::query(
            "insert into ballot(id, name, timestamp, owner_id, poll_id) values ($1, $2, $3, $4, $5)"
        ).bind(&ballot.id)
        .bind(&ballot.name)
        .bind(&ballot.timestamp)
        .bind(&ballot.owner_id)
        .bind(poll_id);

        query.execute(&mut self.tx).await
    }

    async fn update_ballot(&mut self, poll_id: &str, ballot: &Ballot)
    -> Result<PgDone, sqlx::Error> {
        sqlx::query(
            "update ballot set timestamp=$1 where id = $2 and name = $3 and poll_id = $4"
        ).bind(&ballot.timestamp)
        .bind(&ballot.id)
        .bind(&ballot.name)
        .bind(poll_id)
        .execute(&mut self.tx)
        .await
    }

    pub async fn select_candidates(&mut self, poll_id: &str)-> Result<Vec<Candidate>, sqlx::Error> {
        sqlx::query_as::<_, Candidate>(
            "select id, name, description from candidate where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await
    }

    pub async fn insert_candidate(&mut self, poll_id: &str, name: &str, description: &Option<String>)
    -> Result<PgDone, sqlx::Error>{
        sqlx::query("insert into candidate(poll_id, name, description) values ($1, $2, $3)")
        .bind(poll_id)
        .bind(name)
        .bind(description)
        .execute(&mut self.tx)
        .await
    }

    pub async fn select_poll(&mut self, id: &str) -> Result<Option<Poll>, sqlx::Error> {
        sqlx::query_as::<_, Poll>(
            "select id, name, description, owner_id, expires, close \
            from poll where id=$1",
        ).bind(id)
        .fetch_optional(&mut self.tx).await
    }

    pub async fn insert_poll(&mut self, poll: &Poll) -> Result<PgDone, sqlx::Error> {
        sqlx::query(
            "insert \
                into poll(id, name, description, owner_id, expires, close) \
                values ($1, $2, $3, $4, $5, $6)"
        ).bind(&poll.id)
        .bind(&poll.name)
        .bind(&poll.description)
        .bind(&poll.owner_id)
        .bind(poll.expires)
        .bind(poll.close)
        .execute(&mut self.tx)
        .await
    }

    pub async fn select_rankings(&mut self, poll_id: &str) -> Result<Vec<Ranking>, sqlx::Error> {
        sqlx::query_as(
            "select ballot_id, candidate_id, ranking from ranking where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await
    }

    pub async fn delete_rankings(&mut self, poll_id: &str, ballot_id: &str)
    -> Result<PgDone, sqlx::Error> {
        sqlx::query(
            "delete from ranking where poll_id = $1 and ballot_id = $2"
        ).bind(poll_id)
        .bind(ballot_id)
        .execute(&mut self.tx)
        .await
    }

    pub async fn insert_ranking(&mut self, poll_id: &str, ranking: &Ranking)
    -> Result<PgDone, sqlx::Error>{
        sqlx::query(
            "insert into ranking(poll_id, ballot_id, candidate_id, ranking)
                values ($1, $2, $3, $4)"
            ).bind(poll_id)
            .bind(&ranking.ballot_id)
            .bind(ranking.candidate_id)
            .bind(ranking.ranking as i32)
            .execute(&mut self.tx)
            .await
    }

    pub async fn commit(self)
    -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }
}

impl From<sqlx::Error> for InsertCandidateErr {
    fn from(e: sqlx::Error) -> InsertCandidateErr {
        if e.as_database_error()
            .and_then(|de| de.code())
            .map(|code| code == "23505")
            .unwrap_or(false)
            { 
                InsertCandidateErr::Conflict
            } else {
                InsertCandidateErr::PostgresErr(e)
            }
    }
}

impl PickyDb {
    pub fn new(db_pool: PgPool) -> PickyDb {
        PickyDb{ pool: db_pool }
    }

    pub async fn insert_poll(&self, id: &str, identity: &model::Identity, request: &model::PostPollRequest) -> Result<(), InsertPollErr>
    {
        let transaction = self
        .pool
        .begin()
        .await
        .map_err(|e| {
            error!("Failed to get transaction: {:?}", e);
            InsertPollErr::Unexpected
        })?;

        let model::Identity::SecretKey(owner_id) = identity;

        let mut transaction = PickyPollTransaction{tx: transaction};
        let poll = Poll {
            id: id.to_owned(),
            name: request.name.clone(),
            description: request.description.clone(),
            owner_id: owner_id.clone(),
            expires: Utc::now(),
            close: None,
        };

        transaction.insert_poll(&poll)
        .await
        .map_err(|e| {
            error!("Failed to insert poll: {:?}", e);
            InsertPollErr::Unexpected
        })?;

        for c in request.candidates.iter() {
            transaction.insert_candidate(id, &c.name, &c.description)
            .await
            .map_err(|e| {
                error!("Failed inserting candidate: {:?}", e);
                InsertPollErr::Unexpected
            })?;
        }

        transaction.commit()
        .await
        .map_err(|e| {
            error!("Failed to commit");
            InsertPollErr::Unexpected
        })
    }

    pub async fn select_poll(&self, id: &str) -> Result<GetPollResponse, SelectPollErr> {
        let transaction = self.pool.begin().
            await
            .map_err(|e| {
                error!("Failed to get transaction: {:?}", e);
                SelectPollErr::Unexpected
            })?;
        let mut transaction = PickyPollTransaction{tx: transaction};
        let poll = transaction.select_poll(id)
            .await
            .map_err(|e| {
                error!("Error selecting poll: {:?}", e);
                SelectPollErr::Unexpected
            })?
            .ok_or(SelectPollErr::PollNotFound)?;

        let candidates = transaction.select_candidates(id).await;
        let ballots = transaction.select_ballots(id).await;
        let rankings = transaction.select_rankings(id).await;
        
        let (candidates, ballots, rankings) = candidates.and_then(|c|
            ballots.and_then(|b|
                rankings.map(|r| (c, b, r))
            )
        ).map_err(|e| {
            error!("Error selecting poll: {:?}", e);
            SelectPollErr::Unexpected
        })?;

        let candidate_id_to_name: HashMap<i32, Arc<String>> = candidates.iter()
            .map(|c| (c.id, Arc::new(c.name.clone())))
            .collect();

        let mut rankings_by_ballot_id: HashMap<String, Vec<Ranking>> = rankings
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
            .map(|c| model::Candidate {
                name: c.name,
                description: c.description,
            })
            .collect();

        Ok(GetPollResponse {
            id: poll.id,
            name: poll.name,
            description: poll.description,
            candidates: candidates,
            expires: poll.expires,
            close: poll.close,
            ballots: ballots,
        })
    }

    pub async fn upsert_ballot(
        &self,
        poll_id: &str,
        ballot_id: &str,
        identity: &model::Identity,
        request: &model::PutBallotRequest) -> Result<(), UpsertBallotErr>
    {
        let tx = self.pool.begin()
        .await
        .map_err(|e| {
            error!("Failed to start transaction: {:?}", e);
            UpsertBallotErr::Unexpected
        })?;
        let mut tx = PickyPollTransaction{tx};

        /*poll exists?*/ tx.select_poll(poll_id)
        .await
        .map_err(|e| {
            error!("Failed to select poll: {:?}", e);
            UpsertBallotErr::Unexpected
        })?
        .ok_or(UpsertBallotErr::PollNotFound)?;

        let previous_row = tx.select_ballot(poll_id, ballot_id)
        .await
        .map_err(|e| {
            error!("Failed to select ballot: {:?}", e);
            UpsertBallotErr::Unexpected
        })?;
        
        let model::Identity::SecretKey(owner_id) = identity;

        let ballot = Ballot {
            id: String::from(ballot_id),
            name: request.name.clone(),
            timestamp: Utc::now(),
            owner_id: String::from(owner_id)
        };
        
        match previous_row {
            None => {
                tx.insert_ballot(poll_id, &ballot)
                    .await
                    .map_err(|e| {
                        error!("Failed to insert ballot: {:?}", e);
                        UpsertBallotErr::Unexpected
                    })
            },
            Some(previous_row) => {
                if previous_row.owner_id != ballot.owner_id {
                    Err(UpsertBallotErr::NotOwner)
                } else if previous_row.name != ballot.name {
                    Err(UpsertBallotErr::NotSameName)
                } else {
                    tx.update_ballot(poll_id, &ballot)
                        .await
                        .map_err(|e| {
                            error!("Error updating ballot: {:?}", e);
                            UpsertBallotErr::Unexpected
                        })
                }
            },
        }?;
        tx.delete_rankings(poll_id, &ballot.id);
        self.upsert_rankings(&mut tx, poll_id, &ballot.id, &request.rankings).await?;
        tx.commit().await.map_err(|e| UpsertBallotErr::Unexpected)?;
        Ok(())
    }

    async fn upsert_rankings<'a>(&self,
        tx: &mut PickyPollTransaction<'a>,
        poll_id: &str,
        ballot_id: &str,
        rankings: &[String]) -> Result<(), UpsertBallotErr>
    {
        let candidates = tx.select_candidates(poll_id)
        .await
        .map_err(|e| {
            error!("Error selecting candidates: {:?}", e);
            UpsertBallotErr::Unexpected
        })?;
        let mut candidate_name_to_id: HashMap<String, i32> = candidates
            .into_iter()
            .map(|c| (c.name, c.id))
            .collect();
        
        for (i, candidate_name) in rankings.iter().enumerate() {
            let candidate_id = candidate_name_to_id
                .remove(candidate_name)
                .ok_or_else(|| UpsertBallotErr::CandidateNotFound(candidate_name.clone()))?;
            let row = Ranking {
                poll_id: String::from(poll_id),
                ballot_id: String::from(ballot_id),
                candidate_id,
                ranking: i as i16,
            };
            tx.insert_ranking(poll_id, &row)
            .await
            .map_err(|e| {
                error!("Error inserting ranking: {:?}", e);
                UpsertBallotErr::Unexpected
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod test_db {
    use std::env;
    use sqlx::{Pool, Postgres};
    use sqlx::postgres::PgPoolOptions;
    const DATABASE_URL: &str = "PICKYPOLL_TEST_DB";

    pub async fn new_pool() -> Pool<Postgres> {
        let db_url = &env::var(&DATABASE_URL)
            .expect(&format!("env variable for {} must be set", DATABASE_URL));
        PgPoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .expect("Failed to connect to the database")
    }
}

// #[cfg(test)]
// mod tests {
//     use std::vec;
//     use chrono::SubsecRound;
//     use rand::{
//         distributions::Alphanumeric,
//         Rng,
//         thread_rng,
//     };
//     use super::test_db;

//     use super::*;

//     fn new_mock_poll() -> Poll {
//         Poll {
//             id: thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
//             name: String::from("Dessert"),
//             description: String::from("What shall be served for dessert? 🍦🍪🎂"),
//             owner_id: String::from("A"),
//             close: None,
//             expires: Utc::now().round_subsecs(0),
//         }
//     }

//     #[tokio::test]
//     async fn test_insert_poll() {

//         let client = PickyDb::new(test_db::new_pool().await);
//         let mock_poll_row = new_mock_poll();
 
//         client.insert_poll(&mock_poll_row, &Vec::new()).await.unwrap();
//         let got_poll = client.select_poll(&mock_poll_row.id).await.unwrap();

//         assert_eq!(mock_poll_row, got_poll)
//     }

//     mod insert_candidate {
//         use super::*;

//         #[tokio::test]
//         async fn happy_path() {
//             let client = PickyDb::new(test_db::new_pool().await);
//             let mock_poll_row = new_mock_poll();
     
//             client.insert_poll(&mock_poll_row).await.unwrap();
    
//             let mock_candidate = Candidate{name: "mock row".to_owned(), description: Some("mock description".to_owned())};
//             client.insert_candidates(&mock_poll_row.id, &vec![
//                 mock_candidate.clone(),
//             ]).await.expect("Failed to insert candidate");
    
//             let selected_candidates = client.select_candidates(&mock_poll_row.id)
//                 .await
//                 .expect("Should successfully select candidates");
            
//             assert_eq!(selected_candidates.len(), 1);
//             assert_eq!(selected_candidates[0], mock_candidate);
//         }

//         #[tokio::test]
//         async fn conflict() {
//             let client = PickyDb::new(test_db::new_pool().await);
//             let mock_poll_row = new_mock_poll();
     
//             client.insert_poll(&mock_poll_row).await.unwrap();
    
//             let mock_candidate = Candidate{name: "mock row".to_owned(), description: Some("mock description".to_owned())};
//             client.insert_candidates(&mock_poll_row.id, &vec![
//                 mock_candidate.clone(),
//             ]).await.expect("Failed to insert candidate");
    
//             let error = client.insert_candidates(&mock_poll_row.id, &vec![
//                     mock_candidate.clone(),
//             ]).await
//                 .expect_err("Should fail when inserting the same candidate again");
//             match error {
//                 InsertCandidateErr::Conflict => (),
//                 _ => panic!("Expected InsertCandidateErr {:?}", error),
//             }
//         }
//     }

//     mod upsert_candidate {
//         use super::*;
//         #[tokio::test]
//         async fn happy_path_create() {
//             let client = PickyDb::new(test_db::new_pool().await);
//             let mock_poll_row = new_mock_poll();
//             client.insert_poll(&mock_poll_row).await.expect("Test setup failed to create poll");

//             let mock_ballot = Ballot {
//                 id: "1".to_owned(),
//                 name: "".to_owned(),
//                 owner_id: "".to_owned(),
//                 timestamp: Utc::now(),
//                 rankings: Vec::new(),
//             };

//             client.upsert_ballot(&mock_poll_row.id, mock_ballot)
//                 .await
//                 .expect("Should successfully create the ballot");
//         }

//         #[tokio::test]
//         async fn poll_not_found() {
//             let client = PickyDb::new(test_db::new_pool().await);
            
//             let mock_ballot = Ballot {
//                 id: "1".to_owned(),
//                 name: "".to_owned(),
//                 owner_id: "".to_owned(),
//                 timestamp: Utc::now(),
//                 rankings: Vec::new(),
//             };

//             let poll_id: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();

//             let error = client.upsert_ballot(&poll_id, mock_ballot)
//             .await
//             .expect_err("should error when poll does not exist");

//             match error {
//                 UpsertBallotErr::PollNotFound => (),
//                 _ => panic!("Should return PollNotFound {:?}", error),
//             }
//         }
//     }
// }

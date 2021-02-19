use std::collections::HashMap;
use std::sync::Arc;
use itertools::Itertools;
use chrono::{
    DateTime,
    offset::Utc,
};
use sqlx::{Executor, PgPool, Postgres, Transaction};

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
    PostgresErr(sqlx::Error),
    Conflict,
}

impl From<sqlx::Error> for InsertPollErr {
    fn from(e: sqlx::Error) -> InsertPollErr {
        InsertPollErr::PostgresErr(e)
    }
}

impl From<sqlx::Error> for SelectPollErr {
    fn from(e: sqlx::Error) -> SelectPollErr {
        SelectPollErr::PostgresErr(e)
    }
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
    PostgresErr(sqlx::Error),
}

impl UpsertBallotErr {
    pub fn postgres(e: sqlx::Error) -> UpsertBallotErr {
        UpsertBallotErr::PostgresErr(e)
    }
}

// #[derive(Debug, Eq, PartialEq)]
// pub struct BallotSummary {
//     pub id: String,
//     pub name: String,
//     pub timestamp: Timestamp,
//     pub rankings: Vec<Arc<String>>,
// }

struct PickyPollTransaction<'a>{
    tx: Transaction<'a, Postgres>,
}

fn log_error(e: sqlx::Error) {
    error!("sqlx error {:?}", e);
}

enum SelectPollErr {
    PollNotFound,
    Unexpected,
}

impl<'a> PickyPollTransaction<'a> {
    async fn select_ballots(&mut self, poll_id: &str)
    -> Result<Vec<Ballot>, sqlx::Error> {
    
        let ballots: Vec<Ballot> = sqlx::query_as(
            "select id, name, timestamp, owner_id from ballot where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await?;
        Ok(ballots)
    }

    pub async fn select_candidates(&mut self, poll_id: &str)-> Result<Vec<Candidate>, sqlx::Error> {
        let query = sqlx::query_as::<_, Candidate>(
            "select id, name, description from candidate where poll_id = $1"
        ).bind(poll_id);

        let result = query.fetch_all(&mut self.tx)
            .await?;

        Ok(result)
    }

    pub async fn select_poll(&mut self, id: &str) -> Result<Option<Poll>, sqlx::Error> {
        let query = sqlx::query_as::<_, Poll>(
            "select id, name, description, owner_id, expires, close \
            from poll where id=$1",
        ).bind(id);
    
        query.fetch_optional(&mut self.tx).await
    }

    pub async fn select_rankings(&mut self, poll_id: &str) -> Result<Vec<Ranking>, sqlx::Error> {
        sqlx::query_as(
            "select ballot_id, candidate_id, ranking from ranking where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await
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

    pub async fn insert_poll(&self, poll: &Poll) -> Result<(), InsertPollErr>
    {
        let query = sqlx::query(
            "insert \
                into poll(id, name, description, owner_id, expires, close) \
                values ($1, $2, $3, $4, $5, $6)"
        ).bind(&poll.id)
            .bind(&poll.name)
            .bind(&poll.description)
            .bind(&poll.owner_id)
            .bind(poll.expires)
            .bind(poll.close);

        let complete = self.pool.execute(query).await;
        complete?;
        Ok(())
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

    pub async fn insert_candidates(&self, poll_id: &str, candidates: &Vec<Candidate>) -> Result<(), InsertCandidateErr> {
        let mut tx = self.pool.begin()
            .await?;

        for candidate in candidates {
            let query = sqlx::query(
                "insert \
                into candidate(name, description, poll_id) \
                    values ($1, $2, $3)")
                .bind(&candidate.name)
                .bind(&candidate.description)
                .bind(poll_id);
            query.execute(&mut tx).await?;
        };
        tx.commit().await?;
        Ok(())
    }

    pub async fn select_candidates(&self, poll_id: &str) -> Result<Vec<Candidate>, sqlx::Error> {
        let mut tx = self.pool.begin()
            .await?;
        let query = sqlx::query_as::<_, Candidate>(
            "select name, description from candidate where poll_id = $1"
        ).bind(poll_id);

        Ok(query.fetch_all(&mut tx).await?)
    }

    pub async fn upsert_ballot(&self, poll_id: &str, ballot: Ballot) -> Result<(), UpsertBallotErr>
    {
        let get = sqlx::query_as::<_, Ballot>(
            "select id, name, timestamp, owner_id from ballot where id = $1 and poll_id=$2"
        ).bind(&ballot.id)
        .bind(poll_id);

        let mut tx = self.pool.begin().await.map_err(UpsertBallotErr::postgres)?;
        let previous_row = get
            .fetch_optional(&mut tx)
            .await.map_err(UpsertBallotErr::postgres)?;
        
        match previous_row {
            None => {
                let insert = sqlx::query(
                    "insert into ballot(id, name, timestamp, owner_id, poll_id) values ($1, $2, $3, $4, $5)"
                ).bind(&ballot.id)
                .bind(&ballot.name)
                .bind(&ballot.timestamp)
                .bind(&ballot.owner_id)
                .bind(poll_id);
                insert.execute(&mut tx)
                    .await
                    .map_err(|e| {
                        let optional_code = e.as_database_error()
                            .and_then(|dbe| dbe.code());
                        match optional_code.as_deref() {
                            Some("23503") => UpsertBallotErr::PollNotFound,
                            _ => UpsertBallotErr::postgres(e)
                        }
                    })?;
            },
            Some(previous_row) => {
                if previous_row.owner_id != ballot.owner_id {
                    return Err(UpsertBallotErr::NotOwner);
                } else if previous_row.name != ballot.name {
                    return Err(UpsertBallotErr::NotSameName);
                } else {
                    let update = sqlx::query(
                        "update ballot set timestamp=$1 where id = $2 and poll_id = $3"
                    ).bind(&ballot.timestamp)
                    .bind(&ballot.id)
                    .bind(poll_id);
                    update.execute(&mut tx)
                        .await
                        .map_err(UpsertBallotErr::postgres)?;
                }
            },
        }
        self.upsert_rankings(&mut tx, poll_id, &ballot.id, &ballot.rankings).await?;
        tx.commit().await.map_err(UpsertBallotErr::postgres)?;
        Ok(())
    }

    async fn upsert_rankings(&self,
        tx: &mut Transaction<'_, Postgres>,
        poll_id: &str,
        ballot_id: &String,
        rankings: &Vec<String>) -> Result<(), UpsertBallotErr>
    {
        sqlx::query("delete from ranking where poll_id = $1 and ballot_id = $2")
            .bind(poll_id)
            .bind(ballot_id)
        .execute(&mut *tx).await.map_err(UpsertBallotErr::postgres)?;
        let candidate_name_to_id: HashMap<String, i32> = sqlx::query_as::<_, (String, i32)>(
            "select name, id from candidate where poll_id = $1"
            )
            .bind(poll_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(UpsertBallotErr::postgres)?
            .into_iter()
            .collect();
        
        for (i, candidate_name) in rankings.iter().enumerate() {
            let candidate_id = candidate_name_to_id
                .get(candidate_name)
                .ok_or_else(|| UpsertBallotErr::CandidateNotFound(candidate_name.clone()))?;
            sqlx::query(
                "insert into ranking(poll_id, ballot_id, candidate_id, ranking)
                    values ($1, $2, $3, $4)"
                ).bind(poll_id)
                .bind(ballot_id)
                .bind(candidate_id)
                .bind(i as i32)
                .execute(&mut *tx)
                .await
                .map_err(UpsertBallotErr::postgres)?;
        }

        Ok(())
    }

    pub async fn select_ballots(&self, poll_id: &str) -> Result<Vec<BallotSummary>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        
        let candidate_id_to_name: HashMap<i32, Arc<String>> = sqlx::query_as(
            "select id, name from candidate where poll_id = $1"
            ).bind(poll_id)
            .fetch_all(&mut tx)
            .await?
            .into_iter()
            .map(|(id, name)| (id, Arc::new(name)))
            .collect();

        let ballots: Vec<(String, String, Timestamp)> = sqlx::query_as(
            "select id, name, timestamp from ballot where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut tx)
        .await?;

        let all_rankings: Vec<(String, i32, i16)> = sqlx::query_as(
            "select ballot_id, candidate_id, ranking from ranking where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut tx)
        .await?;

        let mut ballots_to_rankings: HashMap<String, Vec<Arc<String>>> = all_rankings.into_iter()
        .map(|(ballot_id, candidate_id, ranking)| (ballot_id, (candidate_id, ranking)))
        .into_group_map()
        .into_iter()
        .map(|(ballot_id, mut local_rankings)| {
            local_rankings.sort_by_key(|(_, r)| *r);
            let ranked_candidates: Vec<Arc<String>> = local_rankings
                .into_iter()
                .flat_map(|(candidate_id, _)| {
                    candidate_id_to_name
                        .get(&candidate_id)
                        .map(|name| name.clone())
                }).collect();
            (ballot_id, ranked_candidates)
        })
        .collect();

        let ret_val: Vec<_> = ballots.into_iter()
            .map(|(ballot_id, name, timestamp)| {
                let this_ballot_rankings = ballots_to_rankings
                    .remove(&ballot_id)
                    .unwrap_or(Vec::new());
                BallotSummary {
                    id: ballot_id,
                    name: name,
                    timestamp: timestamp,
                    rankings: this_ballot_rankings,
                }
            })
            .collect();

        Ok(ret_val)
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

#[cfg(test)]
mod tests {
    use std::vec;
    use chrono::SubsecRound;
    use rand::{
        distributions::Alphanumeric,
        Rng,
        thread_rng,
    };
    use super::test_db;

    use super::*;

    fn new_mock_poll() -> Poll {
        Poll {
            id: thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
            name: String::from("Dessert"),
            description: String::from("What shall be served for dessert? 🍦🍪🎂"),
            owner_id: String::from("A"),
            close: None,
            expires: Utc::now().round_subsecs(0),
        }
    }

    #[tokio::test]
    async fn test_insert_poll() {

        let client = PickyDb::new(test_db::new_pool().await);
        let mock_poll_row = new_mock_poll();
 
        client.insert_poll(&mock_poll_row).await.unwrap();
        let got_poll = client.select_poll(&mock_poll_row.id).await.unwrap();

        assert_eq!(mock_poll_row, got_poll)
    }

    mod insert_candidate {
        use super::*;

        #[tokio::test]
        async fn happy_path() {
            let client = PickyDb::new(test_db::new_pool().await);
            let mock_poll_row = new_mock_poll();
     
            client.insert_poll(&mock_poll_row).await.unwrap();
    
            let mock_candidate = Candidate{name: "mock row".to_owned(), description: Some("mock description".to_owned())};
            client.insert_candidates(&mock_poll_row.id, &vec![
                mock_candidate.clone(),
            ]).await.expect("Failed to insert candidate");
    
            let selected_candidates = client.select_candidates(&mock_poll_row.id)
                .await
                .expect("Should successfully select candidates");
            
            assert_eq!(selected_candidates.len(), 1);
            assert_eq!(selected_candidates[0], mock_candidate);
        }

        #[tokio::test]
        async fn conflict() {
            let client = PickyDb::new(test_db::new_pool().await);
            let mock_poll_row = new_mock_poll();
     
            client.insert_poll(&mock_poll_row).await.unwrap();
    
            let mock_candidate = Candidate{name: "mock row".to_owned(), description: Some("mock description".to_owned())};
            client.insert_candidates(&mock_poll_row.id, &vec![
                mock_candidate.clone(),
            ]).await.expect("Failed to insert candidate");
    
            let error = client.insert_candidates(&mock_poll_row.id, &vec![
                    mock_candidate.clone(),
            ]).await
                .expect_err("Should fail when inserting the same candidate again");
            match error {
                InsertCandidateErr::Conflict => (),
                _ => panic!("Expected InsertCandidateErr {:?}", error),
            }
        }
    }

    mod upsert_candidate {
        use super::*;
        #[tokio::test]
        async fn happy_path_create() {
            let client = PickyDb::new(test_db::new_pool().await);
            let mock_poll_row = new_mock_poll();
            client.insert_poll(&mock_poll_row).await.expect("Test setup failed to create poll");

            let mock_ballot = Ballot {
                id: "1".to_owned(),
                name: "".to_owned(),
                owner_id: "".to_owned(),
                timestamp: Utc::now(),
                rankings: Vec::new(),
            };

            client.upsert_ballot(&mock_poll_row.id, mock_ballot)
                .await
                .expect("Should successfully create the ballot");
        }

        #[tokio::test]
        async fn poll_not_found() {
            let client = PickyDb::new(test_db::new_pool().await);
            
            let mock_ballot = Ballot {
                id: "1".to_owned(),
                name: "".to_owned(),
                owner_id: "".to_owned(),
                timestamp: Utc::now(),
                rankings: Vec::new(),
            };

            let poll_id: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();

            let error = client.upsert_ballot(&poll_id, mock_ballot)
            .await
            .expect_err("should error when poll does not exist");

            match error {
                UpsertBallotErr::PollNotFound => (),
                _ => panic!("Should return PollNotFound {:?}", error),
            }
        }
    }
}

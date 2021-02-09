
use chrono::{
    DateTime,
    offset::Utc,
};
use sqlx::{Executor, PgPool};

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
    pub name: String,
    pub description: Option<String>,
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

#[derive(Debug)]
pub enum SelectPollErr {
    NotFound,
    PostgresErr(sqlx::Error),
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
            .await
            ?;
        let query = sqlx::query_as::<_, Candidate>(
            "select name, description from candidate where poll_id = $1"
        ).bind(poll_id);

        Ok(query.fetch_all(&mut tx).await?)
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

    pub async fn select_poll(&self, id: &str) -> Result<Poll, SelectPollErr> {
        let query = sqlx::query_as::<_, Poll>(
            "select id, name, description, owner_id, expires, close \
            from poll where id=$1",
        ).bind(id);

        let poll = query.fetch_optional(&self.pool).await?;

        poll.ok_or(SelectPollErr::NotFound)
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
}
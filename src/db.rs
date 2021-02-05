
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
mod tests {
    use std::{
        env,
    };

    use chrono::SubsecRound;
    use rand::{
        distributions::Alphanumeric,
        Rng,
        thread_rng,
    };
    use sqlx::postgres::{
        PgPoolOptions,
    };

    use super::*;

    const DATABASE_URL: &str = "PICKYPOLL_TEST_DB";

    #[tokio::test]
    async fn test_put_poll() {
        let db_url = &env::var(&DATABASE_URL).unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .unwrap();

        let client = PickyDb::new(pool);
        let mock_poll_row = Poll {
            id: thread_rng().sample_iter(&Alphanumeric).take(10).collect(),
            name: String::from("Dessert"),
            description: String::from("What shall be served for dessert? 🍦🍪🎂"),
            owner_id: String::from("A"),
            close: None,
            expires: Utc::now().round_subsecs(0),
        };
 
        client.insert_poll(&mock_poll_row).await.unwrap();
        let got_poll = client.select_poll(&mock_poll_row.id).await.unwrap();

        assert_eq!(mock_poll_row, got_poll)
    }
}
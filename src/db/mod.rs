
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
pub enum PutPollErr {
    PostgresErr(sqlx::Error),
    Conflict,
}

impl From<sqlx::Error> for PutPollErr {
    fn from(e: sqlx::Error) -> PutPollErr {
        PutPollErr::PostgresErr(e)
    }
}

#[derive(Debug)]
pub enum GetPollErr {
    NotFound,
    PostgresErr(sqlx::Error),
}

impl From<sqlx::Error> for GetPollErr {
    fn from(e: sqlx::Error) -> GetPollErr {
        GetPollErr::PostgresErr(e)
    }
}

impl PickyDb {
    pub fn new(db_pool: PgPool) -> PickyDb {
        PickyDb{ pool: db_pool }
    }

    pub async fn put_poll(&self, poll: &Poll) -> Result<(), PutPollErr>
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

    pub async fn get_poll(&self, id: &str) -> Result<Poll, GetPollErr> {
        let query = sqlx::query_as::<_, Poll>(
            "select id, name, description, owner_id, expires, close \
            from poll where id=$1",
        ).bind(id);

        let poll = query.fetch_optional(&self.pool).await?;

        poll.ok_or(GetPollErr::NotFound)
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
            name: String::from("My poll"),
            description: String::from("what a great poll"),
            owner_id: String::from("A"),
            close: None,
            expires: Utc::now().round_subsecs(0),
        };
 
        client.put_poll(&mock_poll_row).await.unwrap();
        let got_poll = client.get_poll(&mock_poll_row.id).await.unwrap();

        assert_eq!(mock_poll_row, got_poll)
    }
}
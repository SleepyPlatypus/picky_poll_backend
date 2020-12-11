use chrono::{
    DateTime,
    offset::Utc,
};
use tokio_postgres::Client;

pub struct VoteDbClient {
    postgres: Client,
}

type Timestamp = DateTime<Utc>;

pub struct Poll {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub expires: Timestamp,
    pub close: Option<Timestamp>,
}

pub enum PutPollErr {
    PostgresErr(tokio_postgres::Error),
    Conflict,
}

impl From<tokio_postgres::Error> for PutPollErr {
    fn from(e: tokio_postgres::Error) -> PutPollErr {
        PutPollErr::PostgresErr(e)
    }
}

pub enum GetPollErr {
    PostgresErr(tokio_postgres::Error),
    NotFound(String),
}
impl From<tokio_postgres::Error> for GetPollErr {
    fn from(e: tokio_postgres::Error) -> GetPollErr {
        GetPollErr::PostgresErr(e)
    }
}

impl VoteDbClient {
    pub async fn put_poll(self: &VoteDbClient, poll: &Poll) -> Result<(), PutPollErr> {
        self.postgres
            .query(
                "insert into poll(id, name, description, ownerId, expires, close)
                values ($1, $2, $3, $4, $5, $6)",
                &[&poll.id, &poll.name, &poll.description, &poll.owner_id, &poll.expires, &poll.close]
            )
            .await?;
        Ok(())
    }

    pub async fn get_poll(self: &VoteDbClient, id: &str) -> Result<Poll, GetPollErr> {
        let rows = self
            .postgres
            .query(
                "select id, name, description, ownerId, expires, close from poll where id=$1",
                &[&id],
            )
            .await?;

        let row = rows.get(0).ok_or(GetPollErr::NotFound(String::from(id)))?;

        let poll = Poll {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            owner_id: row.get(3),
            expires: row.get(4),
            close: row.get(5),
        };

        Ok(poll)
    }
}

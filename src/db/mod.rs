use chrono::{
    DateTime,
    offset::Utc,
};
use sqlx::{
    Connection,
    Executor,
    postgres::PgPool,
};

pub struct VoteDbClient {
    db: PgPool,
}

type Timestamp = DateTime<Utc>;

#[derive(sqlx::FromRow)]
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

pub enum GetPollErr {
    PostgresErr,
    NotFound(String),
}


impl From<sqlx::Error> for GetPollErr {
    fn from(e: sqlx::Error) -> GetPollErr {
        GetPollErr::PostgresErr
    }
}

pub async fn put_poll(voteDb: &VoteDbClient, poll: &Poll) -> Result<(), PutPollErr> {

    let mut query = sqlx::query("
insert
into poll(id, name, description, ownerId, expires, close)
values ($1, $2, $3, $4, $5, $6)")
        .bind(&poll.id)
        .bind(&poll.name)
        .bind(&poll.description)
        .bind(&poll.owner_id)
        .bind(&poll.expires)
        .bind(&poll.close);
    
    voteDb.db.execute(query).await?;
    Ok(())
}

// impl<'e, E> VoteDbClient<'e, E>
// where E: Executor<'e> {
//     pub async fn put_poll(self: &VoteDbClient<'e, E>, poll: &Poll) -> Result<(), PutPollErr> {
//         let query = sqlx::query("
//             INSERT into poll(id, name, description, ownerId, expires, close)
//             values (?, ?, ?, ?, ?, ?)
//         ").bind(poll.id)
//         .bind(poll.name)
//         .bind(poll.description)
//         .bind(poll.owner_id)
//         .bind(poll.expires)
//         .bind(poll.close).execute(self.db).await?;
//         Ok(())
//     }
    // pub async fn get_poll(self: &VoteDbClient<'c, E>, id: &str) -> Result<Poll, GetPollErr> {
    //     let rows = sqlx::query(
    //         "select id, name, description, ownerId, expires, close from poll where id=?",
    //     ).bind(id).execute(self.db).await?;

    //     let row = rows.get(0).ok_or(GetPollErr::NotFound(String::from(id)))?;

    //     let poll = Poll {
    //         id: row.get(0),
    //         name: row.get(1),
    //         description: row.get(2),
    //         owner_id: row.get(3),
    //         expires: row.get(4),
    //         close: row.get(5),
    //     };

    //     Ok(poll)
    // }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        any::Any,
        env,
    };
    use sqlx::postgres::PgPoolOptions;

    const DATABASE_URL: &str = "DATABASE_URL";

    #[tokio::test]
    async fn test_put_poll() -> Result<(), PutPollErr> {
        let db_url = &env::var(&DATABASE_URL).unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .unwrap();

        let client = VoteDbClient{db: pool};
        let mockPollRow = Poll {
            id: String::from(""),
            name: String::from("My poll"),
            description: String::from("what a great poll"),
            owner_id: String::from("A"),
            close: None,
            expires: Utc::now(),
        };
        put_poll(&client, &mockPollRow).await?;
        Ok(())
    }
}
use tokio_postgres::{Client};

pub struct VoteDbClient{
  postgres: Client,
}

pub struct Poll {
  pub id: String,
  pub name: String,
  pub description: String,
  pub owner_id: String,
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

impl VoteDbClient{
    async fn get_poll(self: &VoteDbClient, id: &str) -> Result<Poll, GetPollErr> {
        let rows = self
            .postgres
            .query("select id, name, description, ownerId from poll where id=$1", &[&id]).await?;

        let row = rows.get(0).ok_or(GetPollErr::NotFound(String::from(id)))?;

        let poll = Poll {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            owner_id: row.get(3),
        };

        Ok(poll)
  }
}
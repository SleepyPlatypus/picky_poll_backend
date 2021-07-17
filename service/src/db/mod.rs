mod transaction;

pub use transaction::*;
use chrono::{
    DateTime,
    offset::Utc,
};
use sqlx::PgPool;

pub const ENV_KEY: &str = "PICKYPOLL_DB_URL";

#[derive(Clone)]
pub struct PickyDb {
    pool: PgPool
}

type Timestamp = DateTime<Utc>;

#[derive(sqlx::FromRow, Debug, Eq, PartialEq)]
pub struct Poll {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub expires: Timestamp,
    pub close: Option<Timestamp>,
    pub write_ins: bool,
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

impl PickyDb {
    pub fn new(db_pool: PgPool) -> PickyDb {
        PickyDb{ pool: db_pool }
    }

    pub async fn new_transaction(&self) -> Result<PickyPollTransaction<'_>, sqlx::Error> {
        PickyPollTransaction::new(&self.pool).await
    }
}

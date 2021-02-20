use sqlx::{Postgres, Transaction, postgres::PgDone};

use super::*;

pub struct PickyPollTransaction<'a>{
    tx: Transaction<'a, Postgres>,
}

impl<'a> PickyPollTransaction<'a> {

    pub async fn new(db: &'a PgPool) -> Result<PickyPollTransaction<'a>, sqlx::Error> {
        Ok(PickyPollTransaction {
            tx: db.begin().await?
        })
    }

    pub async fn select_ballot(&mut self, poll_id: &str, ballot_id: &str)
    -> Result<Option<Ballot>, sqlx::Error> {
    
        sqlx::query_as(
            "select id, name, timestamp, owner_id from ballot where id = $1 and poll_id = $2"
        ).bind(ballot_id)
        .bind(poll_id)
        .fetch_optional(&mut self.tx)
        .await
    }

    pub async fn select_ballots(&mut self, poll_id: &str)
    -> Result<Vec<Ballot>, sqlx::Error> {
    
        sqlx::query_as(
            "select id, name, timestamp, owner_id from ballot where poll_id = $1"
        ).bind(poll_id)
        .fetch_all(&mut self.tx)
        .await
    }

    pub async fn insert_ballot(&mut self, poll_id: &str, ballot: &Ballot)
    -> Result<PgDone, sqlx::Error> {
        sqlx::query(
            "insert into ballot(id, name, timestamp, owner_id, poll_id) values ($1, $2, $3, $4, $5)"
        ).bind(&ballot.id)
        .bind(&ballot.name)
        .bind(&ballot.timestamp)
        .bind(&ballot.owner_id)
        .bind(poll_id)
        .execute(&mut self.tx)
        .await
    }

    pub async fn update_ballot(&mut self, poll_id: &str, ballot: &Ballot)
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

    pub async fn select_candidates(&mut self, poll_id: &str)
    -> Result<Vec<Candidate>, sqlx::Error> {
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
        .fetch_optional(&mut self.tx)
        .await
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
            "select poll_id, ballot_id, candidate_id, ranking from ranking where poll_id = $1"
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

    pub async fn commit(self)-> Result<(), sqlx::Error> {
        self.tx.commit().await
    }
}

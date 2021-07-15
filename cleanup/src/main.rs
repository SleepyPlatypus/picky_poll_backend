use data;
use chrono;
use log::info;
use std::{
    env,
    fmt::{self, Display},
    error::Error,
    time::Duration,
};
use sqlx::{
    Done,
    postgres::PgPoolOptions
};

#[derive(Debug)]
struct BasicError(String);

impl Display for BasicError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}
impl Error for BasicError {}

#[tokio::main]
async fn main() {
    env_logger::init();
    verify()
        .await
        .expect("Failed to run cleanup.")
}

async fn verify() -> Result<(), Box<dyn Error>> {
    let db_url = &env::var(&data::ENV_KEY)
        .map_err(|_| BasicError(format!("Key {} not found in environment", &data::ENV_KEY)))?;

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect_timeout(Duration::from_secs(2))
        .test_before_acquire(true)
        .connect(db_url)
        .await?;

    let db = data::PickyDb::new(pool);

    let now = chrono::Utc::now();

    let mut tx = db.new_transaction().await?;
    let done = tx.delete_expired(&now).await?;
    tx.commit().await?;

    info!("Deleted {} rows", done.rows_affected());

    Ok(())
}

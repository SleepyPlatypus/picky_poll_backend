use chrono;
use log::info;
use std::{
    env,
    fmt::{self, Display},
    error::Error,
};
use sqlx::{Connection, Done, PgConnection};

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

const DB_ENV_KEY: &str ="PICKYPOLL_DB_URL";

async fn verify() -> Result<(), Box<dyn Error>> {
    let db_url = &env::var(&DB_ENV_KEY)
        .map_err(|_| BasicError(format!("Key {} not found in environment", &DB_ENV_KEY)))?;

    let mut conn = PgConnection::connect(db_url)
        .await?;

    let now = chrono::Utc::now();

    let done = sqlx::query("delete from poll where expires <= $1")
        .bind(now)
        .execute(&mut conn)
        .await?;
        
    info!("Deleted {} rows", done.rows_affected());

    Ok(())
}

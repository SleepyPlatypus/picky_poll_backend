use std::env;

use actix_web::{App, HttpServer};
use sqlx::postgres::PgPoolOptions;

use db::PickyDb;
use operations::PollOperationsImpl;

mod model;
mod paths;
mod db;
mod operations;

const DB_URL: &str = "PICKYPOLL_DB_URL";

#[actix_web::main]
async fn main() {
    let db_url = &env::var(&DB_URL).unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(db_url)
        .await
        .unwrap();

    let app = move || {
        let db = PickyDb::new(pool.clone());
        let ops = operations::PollOperationsImpl::new(db);
        App::new()
            .data(ops)
            .configure(paths::config::<PollOperationsImpl>)
    };
    HttpServer::new(app).bind(("127.0.0.1", 8080))
        .unwrap()
        .run()
        .await
        .expect("HTTP Server failed to run.");
}

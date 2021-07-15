#[macro_use]
extern crate log;

use std::env;

use actix_web::{App, HttpServer};
use sqlx::postgres::PgPoolOptions;

use data as db;
use data::PickyDb;
use operations::PollOperations;
use std::time::Duration;

mod model;
mod service;
mod util;
mod operations;

#[actix_web::main]
async fn main() {
    env_logger::init();
    let db_url = &env::var(&data::ENV_KEY)
        .expect(format!("Failed to get {} from environment", data::ENV_KEY).as_str());
    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_timeout(Duration::from_secs(2))
        .test_before_acquire(true)
        .connect(db_url)
        .await
        .expect("Failed to create database pool");
    pool.acquire().await.expect("Failed to connect to database");

    let app = move || {
        let db = PickyDb::new(pool.clone());
        let ops = PollOperations::new(db);
        App::new()
            .data(ops)
            .configure(service::config::<PollOperations>)
    };
    HttpServer::new(app).bind(("0.0.0.0", 8080))
        .expect("HTTP server failed to bind to 8080")
        .run()
        .await
        .expect("HTTP Server failed to run");
}

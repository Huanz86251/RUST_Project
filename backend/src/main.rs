mod auth;
mod services;
mod routes;
use sqlx::{postgres::PgPoolOptions};
use dotenvy;

use crate::routes::AppState;
// use crate::{auth, services};
#[tokio::main]
async fn main()-> anyhow::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();
    if std::env::var("ENV").ok().as_deref() != Some("prod") {
        dotenvy::dotenv().ok();
    }
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let state = AppState { pool };

    // build our application with a route
    let app = routes::app().with_state(state);

    // run our app with hyper, listening globally on port 8080
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    // println!("Listening on {}", listener.local_addr().unwrap() );
    println!("Listening on 8080");
    Ok(())
}

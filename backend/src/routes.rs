use axum::{
    routing::{get, post},
    http::StatusCode,
    Json,extract::State, Router,
    middleware::from_fn,
};
use crate::{auth, services};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres,PgPool};
use dotenvy;

pub fn app() -> axum::Router<AppState>{
    let protected = Router::<AppState>::new()
        .route("/", get(services::root))
        .route("/accounts", post(services::create_account_handler))
        .route("/accounts",get(services::list_accounts))
        .layer(from_fn(auth::auth_middleware));
    Router::new()
        // `GET /` goes to `root`
        //auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        
        .merge(protected)
        // //accounts

        // // categories
        // .route("/categories", post(create_category).get(list_categories))
        // // transactions
        // .route("/transactions", post(create_transaction).get(list_transactions));
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
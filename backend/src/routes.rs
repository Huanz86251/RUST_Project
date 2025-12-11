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
        //accounts
        .route("/accounts", post(services::create_account_handler))
        .route("/accounts",get(services::list_accounts_handler))
        // categories
        .route("/categories", post(services::create_category_handler))
        .route("/categories", get(services::list_categories_handler))
        .route("/transactions", post(services::create_transaction_handler))
        // transactions

        .layer(from_fn(auth::auth_middleware));
    Router::new()
        // `GET /` goes to `root`
        //auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        
        .merge(protected)

        // .route("/transactions", post(create_transaction).get(list_transactions));
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
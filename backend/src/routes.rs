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
        // transactions
        .route("/transactions", post(services::create_transaction_handler))
        .route("/transactions", get(services::list_transactions_handler))
        //ledger
        .route("/ledger",get(services::get_ledger_snapshot_handler))
        

        .layer(from_fn(auth::auth_middleware));
    Router::new()
        //auth
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .merge(protected)
}
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
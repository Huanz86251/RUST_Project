use axum::{
    routing::{get, post,delete},

    Router,
    middleware::from_fn,
};
use crate::{auth, services};
use sqlx::{PgPool};

pub fn app() -> axum::Router<AppState>{
    let protected = Router::<AppState>::new()
        .route("/", get(services::root))
        // accounts
        .route("/accounts", post(services::create_account_handler))
        .route("/accounts",get(services::list_accounts_handler))
        .route("/accounts/{id}", delete(services::delete_account_handler))
        // categories
        .route("/categories", post(services::create_category_handler))
        .route("/categories", get(services::list_categories_handler))
        .route("/categories/{id}", delete(services::delete_category_handler))
        // transactions
        .route("/transactions", post(services::create_transaction_handler))
        .route("/transactions", get(services::list_transactions_handler))
        .route("/transactions/{id}", delete(services::delete_transaction_handler))
        // ledger
        .route("/ledger",get(services::get_ledger_snapshot_handler))
        // entries
        .route("/entries/{id}", delete(services::delete_entry_handler))
        

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
use crate::auth::AuthUser;
use crate::AppState;
use rust_decimal::Decimal;
use axum::{
    Extension,
    extract::{Request, Json,State,Query},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::error::ErrorKind;
use sqlx::{Postgres, QueryBuilder, Transaction};
use sqlx::types::chrono::NaiveDate;
pub async fn root(Extension(user): Extension<AuthUser>) -> String {
    format!("Hello, user_id={}", user.user_id)
}

pub async fn create_account_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateAccountReq>,
) -> Result<Json<AccountDto>, (StatusCode, String)> {
    let currency = req.currency.as_deref().unwrap_or("CAD");
    let opening_balance = req.opening_balance.unwrap_or(Decimal::ZERO);

    let acc_row = create_account(
        &state.pool,
        user.user_id,
        &req.name,
        &req.account_type,
        currency,
        opening_balance,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?; 

    Ok(Json(acc_row.into()))
}
pub async fn create_account(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    account_type: &str,
    currency: &str,
    opening_balance: Decimal,
) -> Result<AccountRow, sqlx::Error> {
    let acc = sqlx::query_as!(
        AccountRow,
        r#"
        INSERT INTO accounts (user_id, name, account_type, currency, opening_balance)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, name, account_type, currency, opening_balance, created_at
        "#,
        user_id,
        name,
        account_type,
        currency,
        opening_balance
    )
    .fetch_one(pool)
    .await?;
    Ok(acc)
}

#[derive(Debug, Deserialize)]
pub struct AccountQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    #[serde(rename = "type")]
    pub account_type: Option<String>, // checking/cash/credit/other
    pub currency: Option<String>,      // CAD

    pub query: Option<String>,         // 搜索 name
    pub sort: Option<String>,          // name/created_at
    pub order: Option<String>,         // asc/desc

    pub include_balance: Option<bool>, // true/false
}
pub async fn list_accounts_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<AccountQuery>,
) -> Result<Json<Vec<AccountDto>>, (StatusCode, String)> {
    let rows = list_accounts_db(&state.pool, user.user_id, &q)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    Ok(Json(rows))
}
pub async fn list_accounts_db(
    pool: &PgPool,
    user_id: Uuid,
    q: &AccountQuery,
) -> Result<Vec<AccountDto>, sqlx::Error> {
    let limit = q.limit.unwrap_or(50).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);

    let sort_col = match q.sort.as_deref() {
        Some("name") => "name",
        Some("created_at") => "created_at",
        _ => "created_at",
    };
    let sort_order = match q.order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let include_balance = q.include_balance.unwrap_or(false);
    let mut qb = QueryBuilder::new("");
    if include_balance {
        qb.push(
            r#"
            SELECT
                a.id, a.name, a.account_type, a.currency, a.opening_balance, a.created_at,
                (a.opening_balance + COALESCE(SUM(e.amount), 0)) AS balance
            FROM accounts a
            LEFT JOIN entries e ON e.account_id = a.id
            LEFT JOIN transactions t ON t.id = e.transaction_id
            WHERE a.user_id =
            "#,
        );
        qb.push_bind(user_id);
        qb.push(" GROUP BY a.id ");
    } else {
        qb.push(
            r#"
            SELECT
                a.id, a.name, a.account_type, a.currency, a.opening_balance, a.created_at,
                NULL::numeric AS balance
            FROM accounts a
            WHERE a.user_id =
            "#,
        );
        qb.push_bind(user_id);
    }
    // filters
    if let Some(t) = q.account_type.as_deref() {
        qb.push(" AND a.account_type = ");
        qb.push_bind(t);
    }
    if let Some(c) = q.currency.as_deref() {
        qb.push(" AND a.currency = ");
        qb.push_bind(c);
    }
    if let Some(s) = q.query.as_deref() {
        qb.push(" AND a.name ILIKE ");
        qb.push_bind(format!("%{}%", s));
    }

    // ordering
    qb.push(format!(" ORDER BY a.{} {}", sort_col, sort_order).as_str());

    // pagination
    qb.push(" LIMIT ");
    qb.push_bind(limit);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows: Vec<AccountDto> = qb
        .build_query_as::<AccountDto>()
        .fetch_all(pool)
        .await?;

    Ok(rows)
}
pub async fn create_category_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateCategoriesReq>,
) -> Result<Json<CategoriesDto>, (StatusCode, String)> {


    let acc_row = create_category(
        &state.pool,
        user.user_id,
        &req.name,
        req.parent_id,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?; 

    Ok(Json(acc_row.into()))
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    #[serde(rename = "type")]
    pub account_type: Option<String>, // checking/cash/credit/other
    pub currency: Option<String>,      // CAD

    pub query: Option<String>,         // 搜索 name
    pub sort: Option<String>,          // name/created_at
    pub order: Option<String>,         // asc/desc

    pub include_balance: Option<bool>, // true/false
}
pub async fn create_category(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    parent_id: Option<i64>,
) -> Result<CategoriesRow, sqlx::Error> {
    let acc = sqlx::query_as!(
        CategoriesRow,
        r#"
        INSERT INTO categories (user_id, name, parent_id)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, parent_id, name
        "#,
        user_id,
        name,
        parent_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(acc)
}
pub async fn list_categories_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Vec<CategoriesDto>>, (StatusCode, String)> {
    let rows = list_categories_db(&state.pool, user.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    Ok(Json(rows))
}
pub async fn list_categories_db(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CategoriesDto>, sqlx::Error> {
    let rows = sqlx::query_as!(
        CategoriesDto,
        r#"
        SELECT
            id, parent_id, name
        FROM categories 
        WHERE user_id = $1
        ORDER BY parent_id NULLS FIRST, name
        "#,
        user_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
pub async fn create_transaction_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateTransactionsReq>,
) -> Result<Json<TransactionsDto>, (StatusCode, String)> {
    let tran_dto =create_transaction_with_entries_db(
        &state.pool,
        user.user_id,
        req
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;
    Ok(Json(tran_dto))


}

pub async fn create_transaction_with_entries_db( 
    pool: &PgPool, 
    user_id: Uuid, 
    req: CreateTransactionsReq, 
) -> Result<TransactionsDto, sqlx::Error>  {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?; // if error, return all
    let tx_row = sqlx::query_as!(
        TransactionsRow,
        r#"
        INSERT INTO transactions (user_id, occurred_at, payee,memo, created_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, occurred_at, payee, memo, created_at
        "#,
        user_id,
        req.occurred_at,
        req.payee,
        req.memo,
        Utc::now(),
    )
    .fetch_one(&mut *tx)
    .await?;
    let mut entry_dtos = Vec::new();
    for entry in &req.entries {
        let row = sqlx::query_as!(
            EntriesRow,
            r#"
            INSERT INTO entries (tx_id, account_id, category_id, amount, note, user_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, tx_id, account_id, category_id, amount, note
            "#,
            
            tx_row.id,
            entry.account_id,
            entry.category_id,
            &entry.amount,
            entry.note,
            user_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        entry_dtos.push(row.into());
    }

    tx.commit().await?;
    Ok(TransactionsDto {
        id: tx_row.id,
        occurred_at: tx_row.occurred_at,
        payee: tx_row.payee,
        memo: tx_row.memo,
        created_at: tx_row.created_at,
        entries: entry_dtos,
    })
}



#[derive(serde::Deserialize)]
pub struct CreateEntryReq {
    pub account_id: i64,
    pub category_id: Option<i64>,
    pub amount: Decimal,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EntriesRow {
    pub id: i64,              // BIGSERIAL -> i64
    pub user_id: Uuid,        // UUID
    pub tx_id: Uuid, // UUID
    pub account_id: i64,      // BIGSERIAL -> i64
    pub category_id: Option<i64>,
    pub amount: Decimal,   // NUMERIC(14,2)
    pub note: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct EntriesDto {
    pub id: i64,
    pub tx_id: Uuid,
    pub account_id: i64,
    pub category_id: Option<i64>,
    pub amount: Decimal,
    pub note: Option<String>,
}

impl From<EntriesRow> for EntriesDto {
    fn from(t: EntriesRow) -> Self {
        Self {
            id: t.id,
            tx_id: t.tx_id,
            account_id: t.account_id,
            category_id: t.category_id,
            amount: t.amount,
            note: t.note,
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TransactionsRow {
    pub id: Uuid,              
    pub user_id: Uuid,        // UUID
    pub occurred_at: NaiveDate, // TIMESTAMPTZ
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ

}
#[derive(serde::Deserialize)]
pub struct CreateTransactionsReq {
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub occurred_at: NaiveDate, // TIMESTAMPTZ
    pub entries: Vec<CreateEntryReq>,
}

#[derive(Debug, serde::Serialize)]
pub struct TransactionsDto {
    pub id: Uuid,              // BIGSERIAL -> i64
    pub occurred_at: NaiveDate, // TIMESTAMPTZ
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ
    pub entries: Vec<EntriesDto>,
    
}

// impl From<TransactionsRow> for TransactionsDto {
//     fn from(t: TransactionsRow) -> Self {
//         Self {
//             id: t.id,
//             occurred_at: t.occurred_at,
//             payee: t.payee,
//             memo: t.memo,
//             created_at: t.created_at,
//         }
//     }
// }


#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoriesRow {
    pub id: i64,              // BIGSERIAL -> i64
    pub user_id: Uuid,        // UUID
    pub parent_id: Option<i64>,
    pub name: String,         // TEXT

}
#[derive(serde::Deserialize)]
pub struct CreateCategoriesReq {
    pub parent_id: Option<i64>,
    pub name: String,         // TEXT
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CategoriesDto {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,         // TEXT
}

impl From<CategoriesRow> for CategoriesDto {
    fn from(r: CategoriesRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            parent_id: r.parent_id,
        }
    }
}



#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AccountRow {
    pub id: i64,              // BIGSERIAL -> i64
    pub user_id: Uuid,        // UUID
    pub name: String,         // TEXT
    pub account_type: String, // TEXT
    pub currency: String,     // CHAR(3) 
    pub opening_balance: Decimal, // NUMERIC(14,2)
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ
}
#[derive(serde::Deserialize)]
pub struct CreateAccountReq {
    pub name: String,
    pub account_type: String,        // "checking" | "credit" | "cash"...
    pub currency: Option<String>,    
    pub opening_balance: Option<Decimal>, 
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct AccountDto {
    pub id: i64,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub opening_balance: Decimal,
    pub created_at: DateTime<Utc>,
}

impl From<AccountRow> for AccountDto {
    fn from(r: AccountRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            account_type: r.account_type,
            currency: r.currency,
            opening_balance: r.opening_balance,
            created_at: r.created_at,
        }
    }
}


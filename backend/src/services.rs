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
use sqlx::{Postgres, QueryBuilder};
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
pub async fn list_accounts(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Query(q): Query<AccountQuery>,
) -> Result<Json<Vec<AccountDto>>, (StatusCode, String)> {
    let limit =q.limit.unwrap_or(50).clamp(1,100);
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
        qb.push_bind(user.user_id);
        qb.push(" GROUP BY a.id ");
        } 
        else {
        qb.push(
            r#"
            SELECT
                a.id, a.name, a.account_type, a.currency, a.opening_balance, a.created_at,
                NULL::numeric AS balance
            FROM accounts a
            WHERE a.user_id =
            "#,
        );
        qb.push_bind(user.user_id);
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
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    Ok(Json(rows))
}
// pub fn db_err(e: sqlx::Error) -> (StatusCode, String) {
//     match &e {
//         sqlx::Err
//     }
// }

pub async fn create_categories_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(req): Json<CreateCategoriesReq>,
) -> Result<Json<CategoriesDto>, (StatusCode, String)> {
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
#[derive(Debug, Serialize, sqlx::FromRow))]
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



#[derive(Debug, Serialize, sqlx::FromRow))]
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





// pub async fn create_transaction_with_entries(
//     State(state): State<AppState>,
//     occured_at: NaiveDate,
//     payee:Option<&str>,
//     memo: Option<&str>,
//     entries: &[EntryInput],
//     Extension(user): Extension<AuthUser>,
// ) -> Result<(), sqlx::Error> {
//     let tx_id =Uuid::new_v4();
//     sqlx::query(
//         r#"INSERT INTO transactions (id, account_type, currency, opening_balance, user_id)
//         VALUES ($1, $2, $3, $4, $5)"#)
//         .bind(tx_id)
//         .bind(occured_at)
//         .bind(payee)
//         .bind(memo)
//         .bind(user.user_id)
//         .execute(&state.pool)
//         .await?;

//     for entry in entries {
//         sqlx::query(
//             r#"INSERT INTO entries (tx_id, account_id, category_id, amount, note， user_id)
//             VALUES ($1, $2, $3, $4, $5, $6)"#)
//             .bind(entry.tx_id)
//             .bind(account_id)
//             .bind(category_id)
//             .bind(amount)
//             .bind(note)
//             .bind(user.user_id)
//             .execute(&state.pool)
//             .await?;
//     }
//     tx.commit().await?;
//     println!("Transaction committed successfully.");
//     ok(tx_id);

// }
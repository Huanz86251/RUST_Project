use axum::{
    body::Body,
    response::Response,
    extract::{Request, Json,State},
    http,
    http::{ StatusCode,header},
    // http::{Response, StatusCode,header},
    middleware::Next,
};
use axum::Extension;
use chrono::{Duration, Utc};
use sqlx::FromRow;
use crate::AppState;
// db / types
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
// password hashing (argon2)
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;
pub async fn register(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    State(state): State<AppState>,
    Json(payload): Json<RegisterReq>,
) -> Result<(StatusCode, Json<RegisterResp>), (StatusCode, String)> {
    // insert your application logic here
    let user_id =Uuid::new_v4();
    let password_hash = hash_password(&payload.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hash error: {e}")))?;

    let res = sqlx::query(
        r#"INSERT INTO users (id, email, password_hash)
        VALUES ($1, $2, $3)"#)
        .bind(user_id)
        .bind(payload.email)
        .bind(password_hash) // Assume a function to hash passwords
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;
    
    
    // this will be converted into a JSON response
    // with a status code of `201 Created`
    Ok((StatusCode::CREATED, Json(RegisterResp { user_id })))
}
pub async fn login(

    State(state): State<AppState>,
    Json(payload): Json<LoginReq>,
) -> Result<(StatusCode, Json<LoginResp>), (StatusCode, String)> {
    let row: Option<UserRow> = sqlx::query_as(
        r#"SELECT id, password_hash FROM users WHERE email = $1"#
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    let Some(row) = row else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()));
    };
    // Verify password
    let is_valid = verify_password(&payload.password, &row.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("verify error: {e}")))?;
    if !is_valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()));
    }
    // let token = encode_jwt(payload.email)?; // Handle JWT encoding errors
    let token = encode_jwt(row.id)?;
    Ok((
        StatusCode::OK,
        Json(LoginResp {
            user_id: row.id,  
            token:token,
        }),
    ))
}

pub fn hash_password(password: &str) ->Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}
pub fn verify_password(
    password: &str,
    stored_hash: &str,
) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(stored_hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn encode_jwt(user_id: Uuid) -> Result<String, (StatusCode, String)> {
    let secret = std::env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set");
    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24);
    let exp: usize = (now + expire).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;

    let claim = Claims {
        sub: user_id.to_string(),
        iat,
        exp,
    };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("jwt encode error: {e}")))
}
pub fn decode_jwt(token: &str) -> Result<Claims, (StatusCode, String)> {
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "JWT_SECRET not set".into()))?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| (StatusCode::UNAUTHORIZED, format!("invalid token: {e}")))?;

    Ok(data.claims)
}
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".into()))?;

    let token = auth
        .strip_prefix("Bearer ")
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid Authorization scheme".into()))?;

    let claims = decode_jwt(token)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid sub in token".into()))?;

    req.extensions_mut().insert(AuthUser { user_id });

    Ok(next.run(req).await)
}
#[derive(Serialize, Deserialize)]
// Define a structure for holding claims data used in JWT tokens
pub struct Claims {
    pub exp: usize,  // Expiry time of the token
    pub iat: usize,  // Issued at time of the token
    // pub email: String,  // Email associated with the token
    pub sub: String,   // user_id（uuid string）
}

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegisterResp {
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub token: String,
    pub user_id: Uuid,
}
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
}
#[derive(Debug, FromRow)]
struct UserRow {
    id: Uuid,
    password_hash: String,
}

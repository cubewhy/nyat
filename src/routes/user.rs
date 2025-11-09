use actix_web::{
    HttpResponse, ResponseError,
    http::StatusCode,
    web::{self, Json},
};
use anyhow::Context;
use serde_json::{Value, json};
use sqlx::PgPool;
use tracing::{Level, instrument};

use crate::{
    auth::{Credentials, CredentialsVerifyError, generate_token, hash_password, verify_password},
    error::response_error,
    startup::{TokenExpireInterval, TokenSecret},
    telemetry::spawn_blocking_with_tracing,
};

#[derive(serde::Deserialize)]
pub struct RegisterModel {
    username: String,
    password: String,
}

#[instrument(
    name = "Register account",
    skip(payload, pool, token_expire_interval, token_secret)
)]
pub async fn register(
    payload: Json<RegisterModel>,
    pool: web::Data<PgPool>,
    token_expire_interval: web::Data<TokenExpireInterval>,
    token_secret: web::Data<TokenSecret>,
) -> Result<Json<Value>, RegisterError> {
    let payload = payload.into_inner();
    let credentials = Credentials::parse(payload.username, payload.password)?;

    // insert the credentials into the database
    let user_id = register_user(credentials, &pool).await?;

    let token = generate_token(user_id, token_expire_interval.0, &token_secret.0)
        .context("Failed to generate token")?;

    Ok(Json(json!({
        "token": token
    })))
}

#[instrument(name = "Insert credentials into database", skip(credentials, pool))]
async fn register_user(credentials: Credentials, pool: &PgPool) -> Result<i64, RegisterError> {
    // find the user with the same username exists
    if sqlx::query!(
        "SELECT (username) FROM users WHERE username LIKE $1 LIMIT 1",
        credentials.username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to find exist user")?
    .is_some()
    {
        return Err(RegisterError::UsernameExists);
    }

    tracing::event!(Level::INFO, "Register new user: {}", credentials.username);

    let hashed_password = spawn_blocking_with_tracing(move || hash_password(&credentials.password))
        .await
        .context("Failed to spawn password hash task")?
        .context("Failed to hash password")?;

    let res = sqlx::query!(
        "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING (id);",
        credentials.username,
        hashed_password,
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert new user")?;

    Ok(res.id)
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("Username was taken")]
    UsernameExists,
    #[error("Credentials error")]
    CredentialsError(#[from] CredentialsVerifyError),
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

impl ResponseError for RegisterError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            RegisterError::UnknownError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RegisterError::UsernameExists => StatusCode::BAD_REQUEST,
            RegisterError::CredentialsError(credentials_error) => match credentials_error {
                CredentialsVerifyError::BadPasswordLength
                | CredentialsVerifyError::InvalidCharacter => StatusCode::BAD_REQUEST,
            },
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let error_msg = match self {
            RegisterError::UnknownError(_) => "Unknown error",
            RegisterError::UsernameExists => "Username was taken",
            RegisterError::CredentialsError(credentials_error) => match credentials_error {
                CredentialsVerifyError::BadPasswordLength => {
                    "Password length not match the requirement: only length in the range 8-256 is acceptable"
                }
                CredentialsVerifyError::InvalidCharacter => {
                    "Invalid characters found in username or password"
                }
            },
        };

        response_error(self.status_code(), error_msg)
    }
}

#[derive(serde::Deserialize)]
pub struct LoginModel {
    username: String,
    password: String,
}

#[instrument(
    name = "Login",
    skip(payload, pool, token_expire_interval, token_secret)
)]
pub async fn login(
    payload: Json<LoginModel>,
    pool: web::Data<PgPool>,
    token_expire_interval: web::Data<TokenExpireInterval>,
    token_secret: web::Data<TokenSecret>,
) -> Result<Json<Value>, LoginError> {
    let payload = payload.into_inner();
    // convert payload into credentials
    let credentials = Credentials {
        username: payload.username,
        password: payload.password,
    };

    // process auth flow for user
    let user_id: i64 = authorize_user(&credentials, &pool).await?;

    // generate token
    let token = generate_token(
        user_id,
        token_expire_interval.into_inner().0,
        &token_secret.into_inner().0,
    )?;

    Ok(Json(json!({
        "token": token
    })))
}

#[instrument(
    name = "Authorize user"
    skip(credentials, pool)
)]
async fn authorize_user(credentials: &Credentials, pool: &PgPool) -> Result<i64, LoginError> {
    // find the user in the users table
    let user = sqlx::query!(
        "SELECT id, password FROM users WHERE username = $1",
        credentials.username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to query user")?
    .ok_or_else(|| LoginError::BadCredentials)?;

    let hashed_password = user.password;

    if !verify_password(&credentials.password, &hashed_password)
        .context("Failed to verify password")?
    {
        // Password didn't match
        return Err(LoginError::BadCredentials);
    }

    Ok(user.id)
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Bad credentials")]
    BadCredentials,
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::BadCredentials => StatusCode::UNAUTHORIZED,
            LoginError::UnknownError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let error_msg: &'static str = match self {
            LoginError::BadCredentials => "Bad credentials",
            LoginError::UnknownError(_) => "Internal Server Error",
        };

        response_error(self.status_code(), error_msg)
    }
}

use std::future::Ready;
use std::future::ready;

use actix_web::FromRequest;
use actix_web::error::ErrorBadRequest;
use actix_web::error::ErrorUnauthorized;
use actix_web::web;
use argon2::Argon2;
use argon2::PasswordHash;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use chrono::Utc;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::Validation;
use tracing::event;

use crate::error::create_error_json;
use crate::startup::TokenSecret;

pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    /// Parse credentials with checks
    ///
    /// Use for register propose
    pub fn parse(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, CredentialsVerifyError> {
        let username = username.into();
        let password = password.into();

        // check the chars in username and password
        if !username.is_ascii() || !password.is_ascii() {
            return Err(CredentialsVerifyError::InvalidCharacter);
        }

        // check the password length
        let password_length = password.len();
        if !(8..=256).contains(&password_length) {
            return Err(CredentialsVerifyError::BadPasswordLength);
        }

        Ok(Self { username, password })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsVerifyError {
    #[error(
        "Password length not match the requirement: only length in the range 8-256 is acceptable"
    )]
    BadPasswordLength,

    #[error("Invalid character")]
    InvalidCharacter,
}

pub fn hash_password(raw_pwd: &str) -> Result<String, argon2::password_hash::Error> {
    // generate a salt
    let salt = SaltString::generate(&mut OsRng);
    let hasher = Argon2::default();

    Ok(hasher.hash_password(raw_pwd.as_bytes(), &salt)?.to_string())
}

pub fn verify_password(
    raw_pwd: &str,
    hashed_pwd: &str,
) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hashed_pwd)?;
    let res = Argon2::default()
        .verify_password(raw_pwd.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(res)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Claims {
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)

    user_id: i64,
}

pub fn generate_token(
    user_id: i64,
    expire_intenval: usize,
    token_secret: &[u8],
) -> Result<String, anyhow::Error> {
    let current_timestamp = Utc::now().timestamp();

    let claims = Claims {
        iat: current_timestamp as usize,
        exp: current_timestamp as usize + expire_intenval,
        user_id,
    };

    // serialize token
    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(token_secret),
    )?;
    Ok(token)
}

pub struct BearerAuth {
    pub user_id: i64,
}

impl FromRequest for BearerAuth {
    type Error = actix_web::Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let Some(token) = req.headers().get("Authorization") else {
            let error_json = create_error_json("No token provided");
            return ready(Err(ErrorUnauthorized(error_json)));
        };

        let Ok(token) = token.to_str() else {
            return ready(Err(ErrorBadRequest(create_error_json(
                "Authorization header is not an valid string",
            ))));
        };
        // cut Bearer from token
        if !token.starts_with("Bearer ") {
            return ready(Err(ErrorBadRequest(create_error_json(
                "Authorization header must starts with \"Bearer\"",
            ))));
        }
        let token = token.trim_start_matches("Bearer ");

        let jwt_secret = req
            .app_data::<web::Data<TokenSecret>>()
            .map(|secret| &secret.0)
            .unwrap();

        // parse the token
        let jwt_key = DecodingKey::from_secret(jwt_secret);
        let claims = match jsonwebtoken::decode::<Claims>(&token, &jwt_key, &Validation::default())
        {
            Ok(data) => data,
            Err(err) => {
                event!(tracing::Level::WARN, "Failed to valid jwt: {err}");
                return ready(Err(ErrorUnauthorized(create_error_json("Unauthorized"))));
            }
        };

        ready(Ok(Self {
            user_id: claims.claims.user_id,
        }))
    }
}

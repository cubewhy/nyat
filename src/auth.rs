use argon2::Argon2;
use argon2::PasswordHasher;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;

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
    ) -> Result<Self, CredentialsError> {
        let username = username.into();
        let password = password.into();

        // check the chars in username and password
        if !username.is_ascii() || !password.is_ascii() {
            return Err(CredentialsError::InvalidCharacter);
        }

        // check the password length
        let password_length = password.len();
        if password_length < 8 || password_length > 256 {
            return Err(CredentialsError::BadPasswordLength);
        }

        Ok(Self { username, password })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
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

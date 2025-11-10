use actix_web::{
    HttpResponse, ResponseError,
    http::StatusCode,
    web::{self, Json},
};
use anyhow::Context;
use serde_json::json;
use sqlx::PgPool;
use tracing::instrument;

use crate::{auth::BearerAuth, error::response_error, routes::user::load_user_by_username};

#[derive(serde::Deserialize)]
pub struct CreatePMModel {
    peer_username: String,
}

#[instrument(name = "Create private message", skip(payload, pool, credentials))]
pub async fn create_pm(
    payload: Json<CreatePMModel>,
    pool: web::Data<PgPool>,
    credentials: BearerAuth,
) -> Result<HttpResponse, CreatePMError> {
    // find the peer user
    let Some(peer_id) = load_user_by_username(&payload.peer_username, &pool)
        .await
        .context("Failed to load peer user")?
    else {
        return Err(CreatePMError::PeerNotFound);
    };

    // find the exist pm
    let chat_id: i64 = match sqlx::query!(
        r#"
SELECT
    cp.chat_id
FROM
    chat_participants AS cp
JOIN
    chats AS c ON cp.chat_id = c.id
WHERE
    c.type = 'private'
    AND cp.user_id IN ($1, $2)
GROUP BY
    cp.chat_id
HAVING
    COUNT(cp.user_id) = 2
LIMIT 1;
    "#,
        // user 1 id
        credentials.user_id,
        // user 2 id
        peer_id,
    )
    .fetch_optional(pool.as_ref())
    .await
    .context("Failed to query exist chat")?
    .map(|chat| chat.chat_id)
    {
        Some(id) => id,
        None => create_pm_chat(credentials.user_id, peer_id, &pool).await?,
    };

    Ok(HttpResponse::Created().json(json!({
        "chat_id": chat_id,
    })))
}

async fn create_pm_chat(user_id: i64, peer_id: i64, pool: &PgPool) -> Result<i64, CreatePMError> {
    let new_chat_id = sqlx::query_scalar!(
        r#"
        WITH new_chat AS (
            INSERT INTO chats (type) VALUES ('private')
            RETURNING id
        )
        INSERT INTO chat_participants (chat_id, user_id, role)
        SELECT id, user_id, 'member'
        FROM new_chat, (VALUES ($1::bigint), ($2::bigint)) AS users(user_id)
        RETURNING (SELECT id FROM new_chat) AS "id!"
        "#,
        user_id,
        peer_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert chat entity")?;

    Ok(new_chat_id)
}

#[derive(Debug, thiserror::Error)]
pub enum CreatePMError {
    #[error("Peer user not found")]
    PeerNotFound,
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

impl ResponseError for CreatePMError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            CreatePMError::UnknownError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CreatePMError::PeerNotFound => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let msg = match self {
            CreatePMError::UnknownError(_) => "Internal Server Error",
            CreatePMError::PeerNotFound => "Peer user not found",
        };
        response_error(self.status_code(), msg)
    }
}

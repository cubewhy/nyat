use actix_web::{
    ResponseError,
    http::StatusCode,
    web::{self, Json},
};
use sqlx::PgPool;
use tracing::instrument;

use crate::{auth::BearerAuth, error::response_error};

#[derive(serde::Deserialize)]
pub struct CreatePMModel {
    peer_username: String,
}

#[instrument(name = "Create private message", skip(payload, pool, credentials))]
pub async fn create_pm(
    payload: Json<CreatePMModel>,
    pool: web::Data<PgPool>,
    credentials: BearerAuth,
) -> Result<Json<serde_json::Value>, CreatePMError> {
    // find the exist pm

    Ok(todo!())
}

#[derive(Debug, thiserror::Error)]
pub enum CreatePMError {
    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

impl ResponseError for CreatePMError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            CreatePMError::UnknownError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        let msg = match self {
            CreatePMError::UnknownError(_) => "Internal Server Error",
        };
        response_error(self.status_code(), msg)
    }
}

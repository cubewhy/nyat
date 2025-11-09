use actix_web::{HttpResponse, http::StatusCode, web::Json};
use serde_json::json;

pub fn response_error(status: StatusCode, msg: &str) -> HttpResponse {
    HttpResponse::build(status).json(Json(create_error_json(msg)))
}

pub fn create_error_json(msg: &str) -> serde_json::Value {
    json!({ "error": msg })
}

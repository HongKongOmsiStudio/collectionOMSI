use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error_code: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    InternalServerError(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("找不到指定的資源".to_string()),
            _ => AppError::DatabaseError(err),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_code, message) = match self {
            AppError::DatabaseError(e) => {
                eprintln!("🔥 資料庫嚴重錯誤: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "資料庫處理請求時發生異常".to_string())
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR", msg),
        };

        let body = Json(ErrorResponse {
            success: false,
            error_code: error_code.to_string(),
            message,
        });

        (status_code, body).into_response()
    }
}

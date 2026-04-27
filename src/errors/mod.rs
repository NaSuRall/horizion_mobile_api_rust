use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Non autorisé")]
    Unauthorized,
    #[error("Accès interdit")]
    Forbidden,
    #[error("Ressource introuvable")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Erreur base de données")]
    Database(#[from] sqlx::Error),
    #[error("Erreur interne du serveur")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::Unauthorized  => (StatusCode::UNAUTHORIZED,            self.to_string()),
            Self::Forbidden     => (StatusCode::FORBIDDEN,               self.to_string()),
            Self::NotFound      => (StatusCode::NOT_FOUND,               self.to_string()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST,             m.clone()),
            Self::Conflict(m)   => (StatusCode::CONFLICT,                m.clone()),
            Self::Database(e)   => {
                if e.to_string().contains("Duplicate entry") {
                    (StatusCode::CONFLICT, "Cet email est déjà utilisé".to_string())
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Erreur base de données".to_string())
                }
            }
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne du serveur".to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
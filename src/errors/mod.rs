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
            Self::Unauthorized  => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden     => (StatusCode::FORBIDDEN,    self.to_string()),
            Self::NotFound      => (StatusCode::NOT_FOUND,    self.to_string()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST,  m.clone()),
            Self::Conflict(m)   => (StatusCode::CONFLICT,     m.clone()),
            Self::Database(e) => {
                if matches!(e, sqlx::Error::RowNotFound) {
                    (StatusCode::NOT_FOUND, "Ressource introuvable".to_string())
                } else if e.to_string().contains("Duplicate entry") {
                    (StatusCode::CONFLICT, "Cet email est déjà utilisé".to_string())
                } else {
                    tracing::error!("Erreur DB : {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Erreur base de données".to_string())
                }
            }
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne du serveur".to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(e: validator::ValidationErrors) -> Self {
        let msg = e
            .field_errors()
            .values()
            .flat_map(|v| v.iter())
            .next()
            .and_then(|err| err.message.as_ref())
            .map(|cow| cow.to_string())
            .unwrap_or_else(|| "Données invalides".to_string());
        ApiError::BadRequest(msg)
    }
}

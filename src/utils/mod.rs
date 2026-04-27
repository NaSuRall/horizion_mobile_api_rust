use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use axum::response::IntoResponse;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_json::json;
use uuid::Uuid;
use crate::models::Claims;

pub fn calculate_rank(points: i32) -> &'static str {
    match points {
        p if p >= 5000 => "Diamond",
        p if p >= 2500 => "Platine",
        p if p >= 1000 => "Gold",
        p if p >= 500  => "Silver",
        _              => "Bronze",
    }
}

pub fn extract_user_id(headers: &HeaderMap, secret: &str) -> Result<Uuid, axum::response::Response> {
    let claims = decode_claims(headers, secret)?;
    Uuid::parse_str(&claims.sub).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "UUID invalide dans le token" })),
    ).into_response())
}

pub fn extract_admin_user_id(headers: &HeaderMap, secret: &str) -> Result<Uuid, axum::response::Response> {
    let claims = decode_claims(headers, secret)?;

    if claims.role.as_deref() != Some("admin") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Accès refusé — droits administrateur requis" })),
        ).into_response());
    }

    Uuid::parse_str(&claims.sub).map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "UUID invalide dans le token" })),
    ).into_response())
}

fn decode_claims(headers: &HeaderMap, secret: &str) -> Result<Claims, axum::response::Response> {
    let auth_header = match headers.get("Authorization") {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Token manquant" })),
        ).into_response()),
    };

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Format de token invalide" })),
        ).into_response()),
    };

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Token invalide ou expiré" })),
    ).into_response())
}
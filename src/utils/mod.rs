use axum::http::HeaderMap;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use uuid::Uuid;
use crate::errors::ApiError;
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

pub fn extract_user_id(headers: &HeaderMap, secret: &str) -> Result<Uuid, ApiError> {
    let claims = decode_claims(headers, secret)?;
    Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)
}

pub fn extract_admin_user_id(headers: &HeaderMap, secret: &str) -> Result<Uuid, ApiError> {
    let claims = decode_claims(headers, secret)?;

    if claims.role.as_deref() != Some("admin") {
        return Err(ApiError::Forbidden);
    }

    Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)
}

fn decode_claims(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized)
}

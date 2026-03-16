use chrono::{NaiveDate, NaiveDateTime};
use jsonwebtoken::signature::digest::typenum::Integer;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;


#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub last_name: String,
    pub first_name: String,
    pub pseudo: String,
    pub email: String,
    pub password: String,
    pub birth_date: Option<NaiveDate>,
    pub phone: Option<String>,
    pub pp: Option<String>,
    pub point: u64,
    pub created_at: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}


#[derive(Deserialize)]
pub struct RegisterUser {
    pub last_name: String,
    pub first_name: String,
    pub pseudo: String,
    pub email: String,
    pub password: String,
    pub phone: String,
    pub point: u64
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
}

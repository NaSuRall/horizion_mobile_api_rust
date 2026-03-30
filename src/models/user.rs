use chrono::{NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use sqlx::Type;
#[derive(Debug, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ENUM")]
#[sqlx(rename_all = "PascalCase")]
pub enum Rank {
    Bronze,
    Silver,
    Gold,
    Platine,
    Diamond,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub last_name: String,
    pub first_name: String,
    pub pseudo: String,
    pub email: String,
    pub password: String,
    pub birth_date: Option<NaiveDate>,
    pub phone: Option<String>,
    pub pp: Option<String>,
    pub point: Option<i32>,
    pub rank: Rank,
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
    pub phone: String
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
}

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum Rank {
    Bronze,
    Silver,
    Gold,
    Platine,
    Diamond,
}

// MySQL envoie les ENUM comme des strings sur le wire — on décode manuellement
impl sqlx::Type<sqlx::MySql> for Rank {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }
    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
            || <&str as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for Rank {
    fn decode(value: sqlx::mysql::MySqlValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<'r, sqlx::MySql>>::decode(value)?;
        Ok(match s.as_str() {
            "Silver"  => Rank::Silver,
            "Gold"    => Rank::Gold,
            "Platine" => Rank::Platine,
            "Diamond" => Rank::Diamond,
            _         => Rank::Bronze,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub last_name: String,
    pub first_name: String,
    pub pseudo: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub phone: Option<String>,
    pub pp: Option<String>,
    pub point: Option<i32>,
    pub rank: Rank,
    pub role: String,
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
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password: Option<String>,
}

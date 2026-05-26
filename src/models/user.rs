use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

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
    pub email_verified: bool,
}

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
    #[validate(length(min = 1, message = "Le mot de passe ne peut pas être vide"))]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct RegisterUser {
    #[validate(length(min = 1, max = 50, message = "Le nom doit contenir entre 1 et 50 caractères"))]
    pub last_name: String,
    #[validate(length(min = 1, max = 50, message = "Le prénom doit contenir entre 1 et 50 caractères"))]
    pub first_name: String,
    #[validate(length(min = 1, max = 50, message = "Le pseudo doit contenir entre 1 et 50 caractères"))]
    pub pseudo: String,
    #[validate(email(message = "Adresse email invalide"))]
    pub email: String,
    #[validate(length(min = 8, max = 72, message = "Le mot de passe doit contenir entre 8 et 72 caractères"))]
    pub password: String,
    #[validate(length(min = 10, max = 20, message = "Le numéro de téléphone doit contenir entre 10 et 20 chiffres"))]
    pub phone: String,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password: Option<String>,
    pub email_verified: bool,
    pub login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

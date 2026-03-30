use crate::config::AppState;
use axum::Json;
use axum::extract::State;
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use jsonwebtoken::{encode, Header, EncodingKey};
use uuid::Uuid;
use crate::models::claim::Claims;
use crate::models::user::{AuthUser, LoginRequest, User, Rank};

pub async fn login(State(state): State<AppState>, Json(body): Json<LoginRequest>) -> Json<Value> {

    let user = sqlx::query_as!(
        AuthUser,
        r#"
        SELECT
            id as "id: Uuid",
            email,
            password
        FROM users
        WHERE email = ?
        "#,
        body.email,
    )
        .fetch_optional(&state.db)
        .await;

    match user {
        Ok(Some(user)) => {

            if user.password != body.password {
                return Json(json!({
                    "status": "error",
                    "message": "Email ou mot de passe incorrect"
                }));
            }


            let full_user = sqlx::query_as!(
                User,
                r#"
                SELECT
                    id as "id: Uuid",
                    last_name,
                    first_name,
                    pseudo,
                    email,
                    password,
                    birth_date,
                    phone,
                    pp,
                    point,
                    `rank` as "rank: Rank"
                FROM users
                WHERE id = ?
                "#,
                user.id
            )
                .fetch_one(&state.db)
                .await;

            match full_user {
                Ok(full_user) => {
                    // 4. Générer le token
                    let token = generate_token(user.id.to_string());

                    Json(json!({
                        "token": token,
                        "user": full_user
                    }))
                }

                Err(_) => Json(json!({
                    "status": "error",
                    "message": "Impossible de récupérer l'utilisateur complet"
                })),
            }
        }

        Ok(None) => Json(json!({
            "status": "error",
            "message": "Utilisateur non trouvé"
        })),

        Err(_) => Json(json!({
            "status": "error",
            "message": "Une erreur est survenue"
        })),
    }
}

fn generate_token(user_id: String) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET manquant dans .env");

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    )
        .unwrap()
}
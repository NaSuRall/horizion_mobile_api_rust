use crate::{config::AppState};
use axum::{Router, routing::get};
use axum::routing::post;
use crate::handlers::{login, register};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user", get(list_users))
        .route("/user/{id}", get(get_user))
        .route("/register", post(register::register))
        .route("/login", post(login::login))
}

// Utilisation du Model User avec le <User>
async fn list_users() -> &'static str {
    "Liste des utilisateurs"
}

async fn get_user() -> &'static str {
    "Détails d’un utilisateur"
}



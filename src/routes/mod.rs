use crate::config::AppState;
use axum::Router;
use axum::routing::post;
use crate::handlers::oauth;
pub mod user;
pub mod point;
pub mod reward;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/google", post(oauth::google_auth))
        .nest("/api", reward::routes())
        .nest("/api", user::routes())
        .nest("/api", point::routes())
}

/// Routeur de test sans rate-limiting ni ServeDir (plus léger pour les tests d'intégration).
pub fn create_test_router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/google", post(oauth::google_auth))
        .nest("/api", reward::routes())
        .nest("/api", user::routes_test())
        .nest("/api", point::routes())
}

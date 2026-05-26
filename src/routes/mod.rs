use std::sync::Arc;
use crate::config::AppState;
use axum::Router;
use axum::routing::post;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use crate::handlers::{oauth, password_reset};
pub mod user;
pub mod point;
pub mod reward;

pub fn create_router() -> Router<AppState> {
    // 3 demandes de reset par 5 min par IP — limite l'énumération d'emails et le spam d'envois.
    let reset_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(100)
            .burst_size(3)
            .finish()
            .unwrap(),
    );

    let reset_request_route = Router::new()
        .route("/api/password-reset/request", post(password_reset::request_password_reset))
        .layer(GovernorLayer::new(reset_governor));

    Router::new()
        .route("/api/auth/google", post(oauth::google_auth))
        .merge(reset_request_route)
        .route("/api/password-reset/verify",  post(password_reset::verify_reset_code))
        .route("/api/password-reset/confirm", post(password_reset::confirm_password_reset))
        .nest("/api", reward::routes())
        .nest("/api", user::routes())
        .nest("/api", point::routes())
}

/// Routeur de test sans rate-limiting ni ServeDir (plus léger pour les tests d'intégration).
pub fn create_test_router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/google", post(oauth::google_auth))
        .route("/api/password-reset/request", post(password_reset::request_password_reset))
        .route("/api/password-reset/verify",  post(password_reset::verify_reset_code))
        .route("/api/password-reset/confirm", post(password_reset::confirm_password_reset))
        .nest("/api", reward::routes())
        .nest("/api", user::routes_test())
        .nest("/api", point::routes())
}

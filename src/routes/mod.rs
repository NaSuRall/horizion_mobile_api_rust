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

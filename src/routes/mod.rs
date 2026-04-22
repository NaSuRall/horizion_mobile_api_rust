use crate::config::AppState;
use axum::Router;
pub mod user;
pub mod point;
pub mod reward;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest("/api", reward::routes())
        .nest("/api", user::routes())
        .nest("/api", point::routes())
}

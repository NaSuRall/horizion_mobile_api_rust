use crate::{config::AppState};
use axum::{Router, routing::get};
use axum::routing::post;
use crate::handlers::{point};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/point", post(point::send_point))
        .route("/point/{id}", get(point::get_point))
}
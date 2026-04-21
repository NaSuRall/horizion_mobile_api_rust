use crate::config::AppState;
use axum::{Router, routing::get, routing::post};
use crate::handlers::point;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/point", post(point::send_point))
        .route("/user/points", get(point::get_user_points))
        .route("/user/qrcode-token", get(point::get_qrcode_token))
}

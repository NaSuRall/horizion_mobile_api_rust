use crate::config::AppState;
use axum::{Router, routing::{get, post}};
use crate::handlers::{reward, admin};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rewards",                       get(reward::get_rewards))
        .route("/rewards/{id}/redeem",           post(reward::redeem_reward))
        .route("/user/redemptions",              get(reward::get_user_redemptions))
        .route("/admin/scan",                    post(admin::admin_scan))
        .route("/admin/validate-redemption",     post(admin::admin_validate_redemption))
}

use std::sync::Arc;
use crate::config::AppState;
use axum::{Router, routing::{get, post, put}};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use crate::handlers::{login, register, user, transaction, event};

/// Routeur sans rate-limiting — utilisé dans les tests d'intégration.
pub fn routes_test() -> Router<AppState> {
    Router::new()
        .route("/login",             post(login::login))
        .route("/register",          post(register::register))
        .route("/user/{id}",         get(user::get_user))
        .route("/user/{id}",         put(user::put_user))
        .route("/user/transactions", get(transaction::get_user_transactions))
        .route("/events",            get(event::get_events))
}

pub fn routes() -> Router<AppState> {
    // 5 tentatives par minute par IP, puis 1 toutes les 12s
    let login_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    // 3 inscriptions par 5 minutes par IP
    let register_governor = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(100)
            .burst_size(3)
            .finish()
            .unwrap(),
    );

    let login_route = Router::new()
        .route("/login", post(login::login))
        .layer(GovernorLayer::new(login_governor));

    let register_route = Router::new()
        .route("/register", post(register::register))
        .layer(GovernorLayer::new(register_governor));

    Router::new()
        .merge(login_route)
        .merge(register_route)
        .route("/user/{id}",         get(user::get_user))
        .route("/user/{id}",         put(user::put_user))
        .route("/user/transactions", get(transaction::get_user_transactions))
        .route("/events",            get(event::get_events))
}
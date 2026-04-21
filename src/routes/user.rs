use crate::config::AppState;
use axum::{Router, routing::{get, post, put}};
use crate::handlers::{login, register, user, transaction, event};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register",           post(register::register))
        .route("/login",              post(login::login))
        .route("/user/{id}",          get(user::get_user))
        .route("/user/{id}",          put(user::put_user))
        .route("/user/transactions",  get(transaction::get_user_transactions))
        .route("/events",             get(event::get_events))
}

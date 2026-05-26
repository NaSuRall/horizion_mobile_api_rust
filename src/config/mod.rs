use axum::http::{HeaderValue, Method, header};
use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{EnvFilter, fmt};

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::Pool<sqlx::MySql>,
    pub jwt_secret: String,
}

#[tokio::main]
pub async fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL non trouvé");

    let db = MySqlPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Impossible de se connecter à MySQL");

    tracing::info!("Connexion MySQL établie");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Impossible d'appliquer les migrations");

    tracing::info!("Migrations appliquées");

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET non trouvé dans .env");
    let state = AppState { db, jwt_secret };

    let address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "4000".to_string());
    let server_addr = format!("{}:{}", address, port);

    let cors = build_cors_layer();

    let app = routes::create_router()
        .nest_service("/admin", ServeDir::new("static/admin"))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&server_addr).await.unwrap();

    tracing::info!("Serveur démarré sur {}", server_addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

fn build_cors_layer() -> CorsLayer {
    let methods = [Method::GET, Method::POST, Method::PUT, Method::DELETE];
    let headers = [header::AUTHORIZATION, header::CONTENT_TYPE];

    let raw = env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();
    let origins: Vec<HeaderValue> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();

    if origins.is_empty() {
        tracing::warn!("CORS_ALLOWED_ORIGINS vide — CORS ouvert à toutes origines (dev uniquement)");
        CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(methods)
            .allow_headers(headers)
    } else {
        tracing::info!("CORS restreint aux origines : {:?}", raw);
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(methods)
            .allow_headers(headers)
            .allow_credentials(true)
    }
}

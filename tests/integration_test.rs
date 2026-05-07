use std::env;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use dotenvy::dotenv;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::mysql::MySqlPoolOptions;
use tower::ServiceExt;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use horizon_api_rust::config::AppState;
use horizon_api_rust::routes::create_test_router;

// ---------------------------------------------------------------------------
// Infrastructure partagée
// ---------------------------------------------------------------------------

async fn build_app() -> axum::Router {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL manquant dans .env");
    let db = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Impossible de se connecter à MySQL pour les tests");

    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "horizon_test_secret_key".to_string());
    let state = AppState { db, jwt_secret };

    create_test_router()
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

async fn body_json(body: Body) -> Value {
    let bytes = body.collect().await.expect("Lecture du body impossible").to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// Email unique à chaque appel pour éviter les conflits entre tests.
fn unique_email() -> String {
    format!("test_{}@horizon-test.com", Uuid::new_v4().simple())
}

/// Helper : inscrit un utilisateur puis le connecte, retourne le JWT.
async fn register_and_login(app: &axum::Router, email: &str, password: &str) -> String {
    let pseudo = format!("user_{}", Uuid::new_v4().simple().to_string().split_at(8).0.to_string());

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Test",
                        "first_name": "Intégration",
                        "pseudo":     pseudo,
                        "email":      email,
                        "password":   password,
                        "phone":      "0600000000"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(res.into_body()).await;
    body["token"]
        .as_str()
        .expect("Pas de token dans la réponse de login")
        .to_string()
}

// ---------------------------------------------------------------------------
// REGISTER
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_register_success() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Dupont",
                        "first_name": "Jean",
                        "pseudo":     "jeandupont",
                        "email":      unique_email(),
                        "password":   "motdepasse123",
                        "phone":      "0600000001"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res.into_body()).await;
    assert_eq!(body["status"], "success");
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let email = unique_email();
    let payload = json!({
        "last_name":  "Dupont",
        "first_name": "Marie",
        "pseudo":     "mariedupont",
        "email":      email,
        "password":   "motdepasse123",
        "phone":      "0600000002"
    })
    .to_string();

    // Première inscription — doit réussir
    let app = build_app().await;
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Deuxième inscription — même email → 409
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = body_json(res.into_body()).await;
    assert!(body["error"].as_str().unwrap_or("").contains("déjà utilisé"));
}

#[tokio::test]
async fn test_register_invalid_email() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Test",
                        "first_name": "User",
                        "pseudo":     "testuser",
                        "email":      "pas-un-email",
                        "password":   "motdepasse123",
                        "phone":      "0600000003"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_password_too_short() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Test",
                        "first_name": "User",
                        "pseudo":     "testuser2",
                        "email":      unique_email(),
                        "password":   "court",
                        "phone":      "0600000004"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// LOGIN
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_login_success() {
    let app = build_app().await;
    let email = unique_email();

    // Inscription préalable
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Login",
                        "first_name": "Test",
                        "pseudo":     "logintest",
                        "email":      email,
                        "password":   "motdepasse123",
                        "phone":      "0600000006"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "motdepasse123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res.into_body()).await;
    assert!(body["token"].is_string(), "Le champ 'token' doit être présent");
    assert_eq!(body["user"]["email"], email);
}

#[tokio::test]
async fn test_login_wrong_password() {
    let app = build_app().await;
    let email = unique_email();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "last_name":  "Wrong",
                        "first_name": "Pass",
                        "pseudo":     "wrongpass",
                        "email":      email,
                        "password":   "motdepasse123",
                        "phone":      "0600000007"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "mauvaispassword" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_unknown_email() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "email":    "inconnu_xyz_@horizon-test.com",
                        "password": "motdepasse123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_invalid_body() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email": "pas-un-email", "password": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// POINTS
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_points_without_token() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/points")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_points_with_token() {
    let app = build_app().await;
    let email = unique_email();
    let token = register_and_login(&app, &email, "motdepasse123").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/points")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res.into_body()).await;
    assert!(body["points"].is_number(), "Le champ 'points' doit être présent");
    assert!(body["rank"].is_string(), "Le champ 'rank' doit être présent");
}

#[tokio::test]
async fn test_get_points_invalid_token() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/points")
                .header(header::AUTHORIZATION, "Bearer token_completement_invalide")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// QRCODE TOKEN
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_qrcode_token_success() {
    let app = build_app().await;
    let email = unique_email();
    let token = register_and_login(&app, &email, "motdepasse123").await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/qrcode-token")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res.into_body()).await;
    assert!(body["token"].is_string(), "Le champ 'token' QR doit être présent");
    assert_eq!(body["expires_in"], 600);
}

#[tokio::test]
async fn test_qrcode_token_without_auth() {
    let app = build_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/qrcode-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

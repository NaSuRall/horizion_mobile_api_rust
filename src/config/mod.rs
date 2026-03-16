use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::env;

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::Pool<sqlx::MySql>,
}

#[tokio::main]
pub async fn main() {
    dotenv().ok();

    // Charger la DATABASE_URL
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL non trouvé");

    // Créer le pool SQLx
    let db = MySqlPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Impossible de se connecter à MySQL");

    // Appliquer les migrations automatiquement
    //sqlx::migrate!()
    //  .run(&db)
    //.await
    //s.expect("Impossible d'appliquer les migrations");

    // Créer le state global
    let state = AppState { db };

    // Charger les variables serveur
    let address = env::var("SERVER_ADDRESS").unwrap();
    let port = env::var("SERVER_PORT").unwrap();
    let server_addr = format!("{}:{}", address, port);

    // Construire le routeur avec le state
    let app = routes::create_router().with_state(state);

    // Lancer le serveur
    let listener = tokio::net::TcpListener::bind(&server_addr).await.unwrap();

    println!("Server running on {}", server_addr);

    axum::serve(listener, app).await.unwrap();
}
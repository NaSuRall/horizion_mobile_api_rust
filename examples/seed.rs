/// Seed de la base de données Horizon Moto
/// Usage : cargo run --example seed
///
/// Comptes créés :
///   Admin   → admin@horizon.fr    / Admin123!         (0 pts · Bronze · role=admin)
///   Client1 → client@horizon.fr   / Client123!        (1 500 pts · Gold)
///   Client2 → client2@horizon.fr  / Client123!        (4 800 pts · Diamond)
///
/// Boutique — récompenses supplémentaires :
///   Remise 10% sur prochain achat  (1 000 pts)
///   Révision offerte               (3 000 pts)
///   Veste moto textile             (6 500 pts)
///   Formation pilotage 1 journée   (8 000 pts)
///
/// Codes de rachat testables dans la PWA caissier :
///   HRZ-TEST-A1 (pending) → client@horizon.fr    / Kit nettoyage
///   HRZ-TEST-B2 (pending) → client2@horizon.fr   / Gants cuir été
///   HRZ-DEMO-USED (used)  → client@horizon.fr    / Bon d'achat 20€

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL manquant dans .env");
    let pool   = MySqlPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Impossible de se connecter à MySQL");

    println!("🌱 Démarrage du seed Horizon Moto…\n");

    // ── 1. Utilisateurs ──────────────────────────────────────────────────────

    let admin_hash  = hash("Admin123!");
    let client_hash = hash("Client123!");

    insert_user(&pool, Uuid::new_v4(), "Admin",  "Horizon", "admin_hrz", "admin@horizon.fr",   &admin_hash,  "0600000001",    0, "Bronze",  "admin").await;
    insert_user(&pool, Uuid::new_v4(), "Dupont", "Jean",    "jdupont",   "client@horizon.fr",   &client_hash, "0611223344", 1500, "Gold",    "user").await;
    insert_user(&pool, Uuid::new_v4(), "Martin", "Sophie",  "sophiem",   "client2@horizon.fr",  &client_hash, "0622334455", 4800, "Diamond", "user").await;

    // Récupère les vrais IDs (INSERT IGNORE : peut ignorer si déjà existants)
    let client1_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind("client@horizon.fr")
        .fetch_one(&pool)
        .await
        .expect("client@horizon.fr introuvable");

    let client2_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind("client2@horizon.fr")
        .fetch_one(&pool)
        .await
        .expect("client2@horizon.fr introuvable");

    println!("✅ Utilisateurs   → admin / client / client2");

    // ── 2. Récompenses supplémentaires ───────────────────────────────────────

    let extra_rewards: Vec<(&str, &str, i32, i32)> = vec![
        // (name, description, point_cost, stock)
        ("Remise 10% sur prochain achat", "Bon de réduction 10% valable sur tout achat en magasin",    1000,  -1),
        ("Révision offerte",              "Révision complète de votre moto (main-d'œuvre offerte)",    3000,   8),
        ("Veste moto textile",            "Veste textile CE niveau 2, toutes saisons",                 6500,   6),
        ("Formation pilotage 1 journée",  "Stage sur circuit homologué, niveau débutant ou confirmé",  8000,   3),
    ];

    for (name, desc, cost, stock) in &extra_rewards {
        sqlx::query(
            "INSERT IGNORE INTO rewards (name, description, point_cost, stock, active)
             VALUES (?, ?, ?, ?, 1)",
        )
        .bind(name)
        .bind(desc)
        .bind(cost)
        .bind(stock)
        .execute(&pool)
        .await
        .expect("Erreur insertion reward");
    }
    println!("✅ Récompenses    → 4 nouvelles (remise, révision, veste, formation)");

    // ── 3. Transactions ───────────────────────────────────────────────────────

    let tx1: Vec<(i32, &str)> = vec![
        ( 500,  "Achat en magasin : 500€"),
        ( 300,  "Achat en magasin : 300€"),
        ( 800,  "Achat en magasin : 800€"),
        (-100,  "Échange : Kit nettoyage moto"),
        ( 200,  "Achat en magasin : 200€"),
    ];
    for (pts, label) in &tx1 {
        insert_transaction(&pool, client1_id, *pts, label).await;
    }

    let tx2: Vec<(i32, &str)> = vec![
        (2000,  "Achat en magasin : 2 000€"),
        (1500,  "Achat en magasin : 1 500€"),
        ( 800,  "Achat en magasin : 800€"),
        ( 500,  "Achat en magasin : 500€"),
        (-800,  "Échange : Gants cuir été"),
        ( 800,  "Achat en magasin : 800€"),
    ];
    for (pts, label) in &tx2 {
        insert_transaction(&pool, client2_id, *pts, label).await;
    }
    println!("✅ Transactions   → 5 lignes client1 · 6 lignes client2");

    // ── 4. Codes de rachat testables ─────────────────────────────────────────

    // Récupère quelques IDs de récompenses
    let rid_kit: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM rewards WHERE name LIKE '%nettoyage%' LIMIT 1"
    ).fetch_optional(&pool).await.unwrap_or(None);

    let rid_gants: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM rewards WHERE name LIKE '%Gants%' LIMIT 1"
    ).fetch_optional(&pool).await.unwrap_or(None);

    let rid_bon: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM rewards WHERE name LIKE '%Bon%' LIMIT 1"
    ).fetch_optional(&pool).await.unwrap_or(None);

    // Pending : peut être validé dans la PWA caissier (/admin)
    if let Some(rid) = rid_kit {
        sqlx::query(
            "INSERT IGNORE INTO redemptions (id, user_id, reward_id, code, status)
             VALUES (?, ?, ?, 'HRZ-TEST-A1', 'pending')"
        )
        .bind(Uuid::new_v4()).bind(client1_id).bind(rid)
        .execute(&pool).await.ok();
        println!("✅ Code rachat    → HRZ-TEST-A1  (pending) · Kit nettoyage · client1");
    }

    if let Some(rid) = rid_gants {
        sqlx::query(
            "INSERT IGNORE INTO redemptions (id, user_id, reward_id, code, status)
             VALUES (?, ?, ?, 'HRZ-TEST-B2', 'pending')"
        )
        .bind(Uuid::new_v4()).bind(client2_id).bind(rid)
        .execute(&pool).await.ok();
        println!("✅ Code rachat    → HRZ-TEST-B2  (pending) · Gants cuir    · client2");
    }

    // Used : déjà consommé (pour tester le message d'erreur dans la PWA)
    if let Some(rid) = rid_bon {
        sqlx::query(
            "INSERT IGNORE INTO redemptions (id, user_id, reward_id, code, status, used_at)
             VALUES (?, ?, ?, 'HRZ-DEMO-USED', 'used', NOW())"
        )
        .bind(Uuid::new_v4()).bind(client1_id).bind(rid)
        .execute(&pool).await.ok();
        println!("✅ Code rachat    → HRZ-DEMO-USED (used)   · Bon d'achat   · client1");
    }

    // ── Résumé ────────────────────────────────────────────────────────────────

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║              🎉 Seed terminé avec succès !           ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  COMPTES                                              ║");
    println!("║  Admin    admin@horizon.fr    Admin123!               ║");
    println!("║  Client1  client@horizon.fr   Client123!  1 500 pts   ║");
    println!("║  Client2  client2@horizon.fr  Client123!  4 800 pts   ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║  CODES DE RACHAT TESTABLES (PWA /admin)               ║");
    println!("║  HRZ-TEST-A1   → pending  (valider pour tester ✓)    ║");
    println!("║  HRZ-TEST-B2   → pending  (valider pour tester ✓)    ║");
    println!("║  HRZ-DEMO-USED → used     (déjà utilisé → erreur)    ║");
    println!("╚══════════════════════════════════════════════════════╝");
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Erreur hachage mot de passe")
        .to_string()
}

async fn insert_user(
    pool: &sqlx::MySqlPool, id: Uuid,
    last_name: &str, first_name: &str, pseudo: &str, email: &str,
    password: &str, phone: &str, points: i32, rank: &str, role: &str,
) {
    sqlx::query(
        "INSERT IGNORE INTO users
         (id, last_name, first_name, pseudo, email, password, phone, point, `rank`, `role`, email_verified)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(id).bind(last_name).bind(first_name).bind(pseudo).bind(email)
    .bind(password).bind(phone).bind(points).bind(rank).bind(role)
    .execute(pool)
    .await
    .expect("Erreur insertion utilisateur");
}

async fn insert_transaction(pool: &sqlx::MySqlPool, user_id: Uuid, points: i32, label: &str) {
    sqlx::query(
        "INSERT INTO transactions (id, user_id, points, label) VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4()).bind(user_id).bind(points).bind(label)
    .execute(pool)
    .await
    .expect("Erreur insertion transaction");
}

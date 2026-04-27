# Horizon Moto — Pistes d'amélioration

> Analyse de l'API Rust/Axum + front React Native/Expo
> Dernière mise à jour : 2026-04-24

Les items sont classés par **priorité** : 🔴 Critique · 🟠 Important · 🟡 Moyen · 🟢 Futur

---

## SÉCURITÉ — API Rust

### ~~🔴 Validation des inputs manquante~~ ✅ FAIT (2026-04-27)

**Problème** : Aucune crate de validation n'est utilisée. Les données entrantes (email, mot de passe, téléphone, noms) arrivent directement en base sans contrôle.

**Risques** :
- Adresses email invalides enregistrées en base
- Mots de passe vides ou d'un seul caractère acceptés
- Champs texte sans limite de longueur → attaque par remplissage de base
- `POST /api/point` accepte n'importe quel `i32`, y compris des valeurs négatives énormes

**Correction recommandée** :
```toml
# Cargo.toml
validator = { version = "0.18", features = ["derive"] }
```
```rust
#[derive(Deserialize, Validate)]
struct RegisterRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8, max = 72))]
    password: String,
    #[validate(length(min = 1, max = 50))]
    first_name: String,
}
```

---

### ~~🔴 Absence de rate limiting / protection brute-force~~ ✅ FAIT (2026-04-27)

**Problème** : Les endpoints `/api/login` et `/api/register` n'ont aucune limitation de débit. Un attaquant peut tester des milliers de combinaisons sans blocage.

**Correction appliquée** :
- `tower_governor 0.8` — 5 req/min sur `/api/login`, 3 req/5min sur `/api/register`, par IP
- Réponse automatique `429 Too Many Requests` + header `Retry-After`

---

### ~~🔴 `POST /api/point` — points arbitraires sans contrainte~~ ✅ FAIT (2026-04-27)

**Problème** : L'endpoint n'impose aucune limite sur le montant. Il est possible d'envoyer une valeur négative massive ou irréaliste, vidant le solde d'un utilisateur instantanément.

**Correction appliquée** :
- Validation `1 ≤ amount ≤ 10 000` sur `POST /api/point` et `POST /api/admin/scan`
- Réponse `400 Bad Request` si la valeur est hors plage

---

### 🟠 CORS complètement ouvert

**Problème** : La configuration actuelle autorise toutes les origines, toutes les méthodes et tous les headers.

```rust
// Code actuel — trop permissif
CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any)
```

**Correction** :
```rust
CorsLayer::new()
    .allow_origin("https://horizonmoto.fr".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT])
    .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
```

---

### 🟠 Tokens JWT à durée de vie trop longue sans refresh

**Problème** : Les tokens de session durent **30 jours** sans possibilité de révocation. Un token volé reste valide un mois entier.

**Corrections recommandées** :
- Réduire la durée du token d'accès à **15–60 minutes**
- Implémenter un **refresh token** (durée 30 jours, stocké en base avec possibilité de révocation)
- Ajouter une table `revoked_tokens` ou utiliser Redis pour invalider des tokens manuellement

---

### ~~🟠 Panics en production — `.unwrap()` et `.expect()` non gérés~~ ✅ FAIT (2026-04-27)

**Problème** : De nombreux appels pouvaient **crasher le serveur** en production.

**Correction appliquée** :
- `ApiError` unifié via `thiserror` (`src/errors/mod.rs`) avec `IntoResponse` propre
- `JWT_SECRET` chargé une seule fois dans `AppState.jwt_secret` au démarrage — plus aucun `.expect()` à l'exécution dans `login.rs`, `point.rs`, `admin.rs`
- Tous les handlers retournent `Result<impl IntoResponse, ApiError>`

---

### 🟡 Pas de logging structuré / audit trail

**Problème** : Aucun système de logs en place. Impossible de détecter des attaques, des abus ou des bugs en production.

**Corrections recommandées** :
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```
- Logger chaque requête : méthode, path, user_id, status, durée
- Logger les événements de sécurité : tentatives de connexion échouées, accès refusés
- Logger chaque opération de points (qui a modifié quoi et quand)

---

### 🟡 Tokens QR non révocables après usage

**Problème** : Un token QR est valide 10 minutes. S'il est intercepté (screenshot partagé, etc.), il peut être réutilisé jusqu'à expiration.

**Correction recommandée** :
- Inclure un `jti` (JWT ID unique) dans chaque token QR
- Stocker ce `jti` en base et le marquer "utilisé" dès le premier scan
- Rejeter les tokens déjà utilisés même s'ils ne sont pas expirés

---

### 🟡 Pas de verrouillage de compte

**Problème** : Aucune limite sur les tentatives de connexion par compte. Attaque par dictionnaire possible sur un compte ciblé.

**Correction** :
- Compter les échecs dans une table `login_attempts` ou un cache Redis
- Verrouiller temporairement après 5 échecs consécutifs (ex : 15 minutes)
- Notifier l'utilisateur par email en cas de verrouillage

---

### 🟡 Événements et récompenses sans authentification

**Problème** : `GET /api/events` et `GET /api/rewards` sont entièrement publics, permettant l'énumération du catalogue par des bots.

**Correction** : Protéger ces endpoints avec un JWT valide (même un token utilisateur suffit).

---

## SÉCURITÉ — Frontend React Native

### 🟠 AsyncStorage non chiffré

**Problème** : Le token JWT et les données utilisateur sont stockés en clair dans AsyncStorage. Sur un appareil rooté/jailbreaké, ces données sont lisibles.

**Correction** :
```bash
expo install expo-secure-store
```
`SecureStore` utilise le Keychain iOS / Keystore Android — chiffrement natif garanti.

---

### 🟠 Pas de certificate pinning (HTTPS)

**Problème** : L'application utilise `fetch()` standard sans vérification du certificat SSL. Une attaque MITM sur un réseau non sécurisé peut intercepter le token JWT.

**Correction** : Utiliser `react-native-ssl-pinning` pour épingler le certificat de l'API en production.

---

### 🟡 Validation côté client insuffisante

**Problème** : La validation front (email avec `@`, mot de passe min. 6 chars, téléphone ≥ 10 chiffres) est trop basique.

**Améliorations** :
- Email : regex RFC 5322 complète
- Mot de passe : min. 8 chars, 1 majuscule, 1 chiffre, 1 caractère spécial
- Téléphone : format E.164
- Longueur max sur tous les champs texte (éviter les inputs abusifs)

---

### 🟡 Pas de timeout de session inactive

**Problème** : Un utilisateur laissant l'application ouverte reste connecté indéfiniment.

**Correction** :
- Détecter l'inactivité avec `AppState` (passage en background)
- Déconnecter automatiquement après X minutes sans interaction (ex : 30 min)

---

### 🟢 Client IDs Google hardcodés

**Problème** : Les `webClientId` et `androidClientId` Google sont écrits en dur dans les fichiers sources.

**Correction** : Les déplacer dans `.env` sous `EXPO_PUBLIC_GOOGLE_WEB_CLIENT_ID` / `EXPO_PUBLIC_GOOGLE_ANDROID_CLIENT_ID`.

---

## QUALITÉ & ROBUSTESSE

### 🟠 Aucun test (API ni front)

**Problème** : Aucun test unitaire, d'intégration ni end-to-end n'est en place.

**Plan recommandé** :
- **API Rust** : `tests/integration_test.rs` — tester login, register, points, qrcode-token, redeem
- **Front** : Jest + React Native Testing Library — tester AuthContext, fetchWithAuth, calculs de rang
- **E2E** : Detox pour les parcours critiques (login → QR → achat boutique)

---

### 🟠 Gestion d'erreurs Rust non unifiée

**Problème** : Chaque handler retourne des tuples `(StatusCode, Json(...))` différents. Code difficile à maintenir et messages d'erreur inconsistants.

**Correction** :
```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Unauthorized")]          Unauthorized,
    #[error("Forbidden")]             Forbidden,
    #[error("Not found")]             NotFound,
    #[error("Database error: {0}")]   Database(#[from] sqlx::Error),
    #[error("Validation: {0}")]       Validation(String),
}

impl IntoResponse for ApiError { /* mapping StatusCode */ }
```

---

### 🟡 Pas d'`ErrorBoundary` React Native

**Problème** : Une exception JavaScript non capturée crashe l'application sans message utilisateur.

**Correction** :
```jsx
import { ErrorBoundary } from 'react-error-boundary';

<ErrorBoundary fallback={<ErrorScreen />}>
  <App />
</ErrorBoundary>
```

---

### 🟡 Faute de frappe dans le nom du fichier Profile

**Problème** : `ProfilScrenn.js` (double `n`) importé partout sous ce nom. Sans impact fonctionnel mais source de confusion.

**Correction** : Renommer en `ProfileScreen.js` et mettre à jour les imports dans `TabNavigator.js` et `App.js`.

---

## NOUVELLES FONCTIONNALITÉS SUGGÉRÉES

### 🟠 Mot de passe oublié
- Endpoint `POST /api/password-reset/request` → envoi d'un email avec lien tokenisé (durée 1h)
- Endpoint `POST /api/password-reset/confirm` → validation du token + nouveau mot de passe
- Écran dédié dans le front avec lien depuis LoginScreen

### 🟠 Notifications push
- `expo-notifications` pour alerter l'utilisateur quand des points sont ajoutés
- Notification lors d'un passage de rang
- Rappel avant expiration d'un code de rachat

### 🟡 Vérification email à l'inscription
- Token de confirmation envoyé par email après `POST /api/register`
- Compte inactif jusqu'à validation (champ `email_verified` en base)

### 🟡 Mode offline
- Mise en cache des données (points, transactions récentes) pour consultation hors connexion
- Synchronisation à la reconnexion réseau

### 🟡 Expiration automatique des codes de rachat
- Les codes `HRZ-XXXX-XXXX` en statut `pending` n'expirent jamais actuellement
- Ajouter un job planifié qui passe les codes non utilisés après 90 jours en `expired`

### 🟢 Historique des changements de rang
- Nouvelle table `rank_history` loggant chaque passage de rang
- Affichage dans le profil : "Passé Diamond le 12/04/2026"

---

## INFRASTRUCTURE & DÉPLOIEMENT

### 🟠 Pas de Docker / CI/CD

**Plan recommandé** :
```yaml
# docker-compose.yml
services:
  api:
    build: ./horizion_mobile_api_rust
    env_file: .env
    ports: ["3000:3000"]
  db:
    image: mysql:8
    volumes: [db_data:/var/lib/mysql]
```
- **GitHub Actions** : build Rust + tests automatiques sur chaque push `main`
- **EAS Build** : `eas.json` pour les builds Expo (APK Android / IPA iOS)

### 🟡 Secret management

**Problème** : `JWT_SECRET` et `DATABASE_URL` dans un `.env` local. En production, utiliser un gestionnaire de secrets.

**Options** : Doppler · HashiCorp Vault · AWS Secrets Manager · variables CI/CD injectées

### 🟡 Monitoring API

- Intégrer `tracing` + export vers Sentry ou Grafana Loki
- Alertes sur les erreurs 5xx en production
- Dashboard : points distribués, rachats, inscriptions par période

---

## RÉSUMÉ PRIORISÉ

| # | Item | Impact | Effort | Priorité |
|---|------|--------|--------|----------|
| 1 | ~~Validation inputs API (`validator`)~~ | Élevé | Moyen | ✅ |
| 2 | ~~Rate limiting sur `/login` et `/register`~~ | Élevé | Faible | ✅ |
| 3 | ~~Contrainte montant sur `POST /api/point`~~ | Élevé | Très faible | ✅ |
| 4 | ~~Supprimer les panics Rust (type ApiError)~~ | Élevé | Moyen | ✅ |
| 5 | Refresh token + réduction durée JWT | Élevé | Élevé | 🟠 |
| 6 | Restreindre le CORS | Moyen | Très faible | 🟠 |
| 7 | AsyncStorage → SecureStore | Moyen | Faible | 🟠 |
| 8 | Tests API + front | Élevé | Élevé | 🟠 |
| 9 | Mot de passe oublié | Élevé (UX) | Moyen | 🟠 |
| 10 | Logging structuré (`tracing`) | Moyen | Faible | 🟡 |
| 11 | Notifications push | Moyen (UX) | Moyen | 🟡 |
| 12 | Docker + CI/CD | Élevé (ops) | Moyen | 🟡 |
| 13 | Certificate pinning | Moyen | Moyen | 🟡 |
| 14 | Verrouillage de compte | Moyen | Moyen | 🟡 |
| 15 | Tokens QR révocables après usage | Faible | Moyen | 🟡 |
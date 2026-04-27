# Horizon — Suivi du Projet

> Application mobile de fidélité (Horizon Moto)
> Dernière mise à jour : 2026-04-27 — Phase 1 ✅ · Phase 2 ✅ · Phase 2.5 ✅ · Phase 2.6 ✅ · Phase 2.7 ✅ · Phase 2.8 ✅ · Phase 2.9 ✅ · Phase 3 (partiel) 🔄

---

## Stack Technique

| Partie | Tech |
|--------|------|
| API | Rust · Axum 0.8 · SQLx 0.7 · MySQL · JWT (HS256) · Tokio · argon2 · tower-http · tower_governor · validator · thiserror |
| Mobile | React Native 0.81 · Expo 54 · React 19 · Axios · AsyncStorage · EAS Build |

---

## Commits livrés

| Dépôt | Commit | Date | Description |
|-------|--------|------|-------------|
| API | `ed28aea` | 2026-04-21 | Phase 1 — Sécurité |
| Front | `2c41da0` | 2026-04-21 | Phase 1 — Sécurité |
| API | Phase 2.5 | 2026-04-21 | Boutique récompenses |
| Front | Phase 2.5+2.6 | 2026-04-21 | Boutique + Récompenses |
| API | `8a2f1d3` | 2026-04-27 | Phase 3 — Validation, ApiError, JWT AppState |
| API | `bb94717` | 2026-04-27 | Phase 3 — Rate limiting login/register |
| Front | `bb7fa78` | 2026-04-27 | Fix Google OAuth — Development Build + redirectUri |
| Front | `22ba8fc` | 2026-04-27 | Fix Google OAuth — access_token params + EXPO_PUBLIC_API_URL EAS |

---

## Phases et Tâches

### PHASE 1 — Sécurité ✅ TERMINÉE

- ✅ **Hash des mots de passe** — `argon2 v0.5`. `register.rs` hash, `login.rs` vérifie. Password exclu de la réponse JSON (`serde skip_serializing`).
- ✅ **`send_point()` corrigé** — Vrai `UPDATE users SET point = point + ?` + auth JWT obligatoire + calcul automatique du rang + mise à jour `rank` en DB.
- ✅ **QR code sécurisé** — Endpoint `GET /api/user/qrcode-token` (JWT signé 10 min). `QrcodeScreen.js` fetch le token + auto-refresh toutes les 9 min via `useFocusEffect`.
- ✅ **Migrations SQLx activées** — `sqlx::migrate!()` actif dans `config/mod.rs`.
- ✅ **CORS configuré** — `tower-http v0.6` + `CorsLayer::new().allow_origin(Any)`.
- ✅ **URL API externalisée** — `axios.js` + `HomeScreen.js` utilisent `EXPO_PUBLIC_API_URL`. `.env.example` créés (front + API). `.env` ajouté au `.gitignore` front.
- ✅ **Doublons email détectés** — `register.rs` retourne `"Cet email est déjà utilisé"` sur erreur Duplicate Entry MySQL.
- ✅ **Fix MySQL ENUM** — Impl manuelle `sqlx::Decode` + `sqlx::Type` pour `Rank` (contournement du bug wire protocol MySQL ENUM → Rust enum).
- ✅ **Rang retourné par `/api/user/points`** — L'endpoint renvoie `{ points, rank }` pour synchroniser le contexte front.
- ✅ **`AuthContext.updatePoints()`** — Accepte désormais `(points, rank)` et met à jour les deux dans le state et AsyncStorage.
- ✅ **Progress bar dynamique** — Calcul en temps réel basé sur `user.point` et les seuils de rang (Bronze 0→500, Silver 500→1000, Gold 1000→2500, Platine 2500→5000, Diamond max).
- ✅ **`ProfilScrenn.handleSave`** — Désormais branchée sur `PUT /api/user/{id}` (Phase 2).

---

### PHASE 2 — Fonctionnalités incomplètes ✅ TERMINÉE

- ✅ **`PUT /api/user/{id}`** — Modification email et/ou password. Vérification du `current_password` si changement de mot de passe. Protection JWT (profil personnel uniquement).
- ✅ **`GET /api/user/{id}`** — Vraie requête SQLx + protection JWT (profil personnel uniquement).
- ✅ **Table `transactions`** — Migration créée. `send_point()` insère un enregistrement à chaque ajout de points.
- ✅ **`GET /api/user/transactions`** — Retourne l'historique trié par date DESC. Branché sur `GainScreen.js`.
- ✅ **Table `events`** — Migration créée + 3 événements de seed. Endpoint `GET /api/events` retourne `{ id, title, location, badge, badgeType, day, month, year }`.
- ✅ **`CalendarScreen.js`** — Fetch dynamique depuis `GET /api/events`. Points sur le calendrier reflètent les vrais événements DB.
- ✅ **`GainScreen.js`** — Fetch `GET /api/user/transactions` via `useFocusEffect`. Affiche spinner pendant le chargement.
- ✅ **`ProfilScrenn.js`** — `handleSave()` appelle `PUT /api/user/{id}`. Affiche "Mot de passe actuel" seulement si nouveau mot de passe saisi.
- ✅ **`src/utils/mod.rs`** — `extract_user_id()` et `calculate_rank()` centralisées et partagées entre handlers.

---

### PHASE 2.5 — Boutique récompenses ✅ TERMINÉE

- ✅ **Champ `role`** — Migration `20260421130000` : `ENUM('user','admin') DEFAULT 'user'` sur `users`. Inclus dans le JWT à la connexion.
- ✅ **Table `rewards`** — Migration `20260421130001` : catalogue (name, description, point_cost, stock, active). 5 récompenses seedées.
- ✅ **Table `redemptions`** — Migration `20260421130001` : code unique `HRZ-XXXX-XXXX`, statut pending/used/expired.
- ✅ **`GET /api/rewards`** — Liste des récompenses actives triées par coût.
- ✅ **`POST /api/rewards/:id/redeem`** — Vérifie les points, déduit, insère la rédemption + transaction négative, décrémente le stock.
- ✅ **`GET /api/user/redemptions`** — Historique des rachats de l'utilisateur connecté.
- ✅ **`POST /api/admin/scan`** — Caissier : décode le QR JWT client + ajoute les points (1€=1pt) + insère transaction.
- ✅ **`POST /api/admin/validate-redemption`** — Caissier : valide un code HRZ-XXXX-XXXX, marque `used`.
- ✅ **PWA Caissier** — Interface HTML/JS servie par Axum sur `/admin`. Login admin, scanner QR caméra (jsQR), saisie montant, validation code.
- ✅ **`ShopScreen.js`** — Boutique mobile : liste récompenses, achat par points, modal code de rachat + QR code.
- ✅ **Navigation** — `InnerStack` dans `App.js` wrappant `TabNavigator` + `ShopScreen`. Accessible depuis HomeScreen.
- ✅ **`HomeScreen.js`** — Section "Boutique récompenses" → bouton navigant vers `ShopScreen`.

---

### PHASE 2.6 — Mes Récompenses ✅ TERMINÉE

- ✅ **`GainScreen.js` — deux onglets** — "Historique" (transactions) + "Récompenses" (rachats). Sélecteur en haut, chargement indépendant pour chaque onglet.
- ✅ **Liste des rachats** — Fetch `GET /api/user/redemptions`. Affiche : nom récompense, code `HRZ-XXXX-XXXX`, date, badge statut (En attente / Utilisé).
- ✅ **Modal code QR** — Appui sur un rachat "En attente" → modal plein écran avec le code en grand + QR scannable par le caissier.
- ✅ **`ShopScreen.js` — bouton post-achat** — Après un échange réussi, le modal affiche "Voir mes récompenses" → navigue vers `GainScreen` onglet Récompenses.
- ✅ **Navigation par paramètre** — `navigation.navigate("Gain", { tab: "recompenses" })` + `useEffect` sur `route.params.tab` dans GainScreen pour auto-sélectionner l'onglet.
- ✅ **Navbar redesignée** — QR code centré via deux groupes `flex:1`. Labels sous les icônes. Point actif. 6 onglets : Accueil · Agenda · QR · Boutique · Gains · Profil.
- ✅ **Seed étendu** — `cargo run --example seed` : 3 comptes (admin/client/client2), 9 récompenses, transactions, codes testables `HRZ-TEST-A1` + `HRZ-TEST-B2` (pending), `HRZ-DEMO-USED` (used).

---

### PHASE 2.7 — Login Google ✅ TERMINÉE

- ✅ **Migration `20260422000000`** — `password` nullable + colonne `google_id VARCHAR(255) NULL`.
- ✅ **`POST /api/auth/google`** — Vérifie l'`access_token` via Google userinfo, crée ou retrouve l'user (liaison email existant), retourne JWT.
- ✅ **`models/user.rs`** — `password: Option<String>` dans `User` et `AuthUser`. Champ `role: String` ajouté et retourné dans toutes les réponses user.
- ✅ **`handlers/login.rs`** — Gère `password = None` (compte Google → mot de passe incorrect).
- ✅ **`reqwest 0.12`** — Ajouté à `Cargo.toml` pour les appels HTTP vers Google userinfo.
- ✅ **`LoginScreen.js` + `RegisterScreen.js`** — Bouton "Continuer avec Google" (AntDesign icon). `expo-auth-session` + `expo-web-browser` installés.
- ✅ **`app.json`** — `scheme: "horizonmoto"` + plugin `expo-web-browser`.

---

### PHASE 2.8 — Admin QR Scanner + Fixes ✅ TERMINÉE

- ✅ **`POST /api/admin/customer-info`** — Décode le JWT QR, retourne infos client (nom, email, points, rang) sans modifier la DB.
- ✅ **`QrcodeScreen.js` — double branche** — `user.role === "admin"` → scanner caméra. Sinon → QR code client habituel.
- ✅ **Scanner admin** — `expo-camera` · demande permission · viewfinder avec coins de cadrage · scan automatique.
- ✅ **Modal infos client** — Nom, email, points actuels, rang + saisie du montant en € (1€ = 1 pt) + bouton confirmation.
- ✅ **Modal succès** — Affiche les points ajoutés + nouveau solde + rang mis à jour.
- ✅ **Fix clavier Android** — `KeyboardAvoidingView behavior="padding"` sur `LoginScreen` et `RegisterScreen`.
- ✅ **Fix crash QrcodeScreen** — `user?.first_name` garanti non-null pour les comptes Google.
- ✅ **Fix `role` manquant** — `role` inclus dans le `SELECT` de tous les handlers retournant un `User` (login, oauth, get_user).

---

### PHASE 3 — Qualité & robustesse 🔄 EN COURS

- ✅ **Custom error type `ApiError`** — `src/errors/mod.rs` avec `thiserror`. `IntoResponse` unifié.
- ✅ **JWT_SECRET dans AppState** — Plus de `.expect("JWT_SECRET")` à chaque requête. Chargé une fois au démarrage dans `AppState.jwt_secret`. Propagé à `extract_user_id`, `extract_admin_user_id`, `generate_token`, `get_qrcode_token`.
- ✅ **Validation inputs API** — `validator 0.18` avec `#[derive(Validate)]` sur `RegisterUser` (email, password 8-72 chars, noms 1-50, phone 10-20) et `LoginRequest`. Validation inline sur `PUT /api/user/{id}` (email format, password 8-72).
- ✅ **Contrainte montant `POST /api/point`** — `1 ≤ point ≤ 10 000` validé. Idem `POST /api/admin/scan`.
- ✅ **Réponse HTTP correcte register** — `201 Created` au lieu de `200 OK`.
- ✅ **Rate limiting** — `tower_governor 0.8` sur `/api/login` (5 req/min) et `/api/register` (3 req/5min) par IP. Réponse `429` + header `Retry-After` automatique.
- [ ] **Logging structuré** — Intégrer `tracing` + `tracing-subscriber` dans l'API.
- [ ] **`ErrorBoundary` React** — Aucun composant `ErrorBoundary` dans le front.
- [ ] **Tests API Rust** — `tests/integration_test.rs` : login, register, points, qrcode-token.
- [ ] **Tests Frontend** — Jest + React Native Testing Library : AuthContext + screens.

---

### PHASE 4 — Nouvelles fonctionnalités

- [ ] **Écran "Mot de passe oublié"** — `POST /api/password-reset` + envoi email + écran dédié.
- [ ] **Vérification email à l'inscription** — Token de confirmation par email.
- [ ] **Push notifications** — Alertes quand des points sont ajoutés.
- [ ] **Mode offline** — Sync des points au retour de connexion.

---

### PHASE 5 — Production ready

- [ ] **Docker Compose** — Conteneuriser API Rust + MySQL.
- [ ] **CI/CD GitHub Actions** — Build + tests automatiques sur push.
- [ ] **EAS Build** — Configurer `eas.json` pour les builds Expo.
- [ ] **README.md** — Instructions de setup pour les deux parties.
- [ ] **Secret management** — `JWT_SECRET` via vault / env sécurisé en prod.

---

## État des fichiers critiques

| Fichier | Statut |
|---------|--------|
| `handlers/login.rs` | ✅ argon2 verify + password masqué |
| `handlers/register.rs` | ✅ argon2 hash + doublon email |
| `handlers/point.rs` | ✅ UPDATE réel + JWT auth + rang calculé + qrcode-token + insert transaction |
| `handlers/user.rs` | ✅ GET + PUT /user/{id} avec JWT auth |
| `handlers/transaction.rs` | ✅ GET /user/transactions |
| `handlers/event.rs` | ✅ GET /events |
| `models/user.rs` | ✅ Rank decode custom MySQL ENUM |
| `models/transaction.rs` | ✅ FromRow Transaction |
| `models/event.rs` | ✅ FromRow Event |
| `utils/mod.rs` | ✅ extract_user_id + calculate_rank partagés |
| `config/mod.rs` | ✅ Migrations + CORS |
| `routes/user.rs` | ✅ Toutes les routes branchées |
| `routes/point.rs` | ✅ Route qrcode-token |
| `migrations/20260421120000` | ✅ Table transactions |
| `migrations/20260421120001` | ✅ Table events + seed |
| `src/api/axios.js` | ✅ EXPO_PUBLIC_API_URL |
| `src/context/AuthContext.js` | ✅ updatePoints(points, rank) + fetchWithAuth |
| `src/screens/HomeScreen.js` | ✅ URL fixée + progress bar dynamique + rang sync |
| `src/screens/QrcodeScreen.js` | ✅ JWT 10 min + auto-refresh |
| `src/screens/ProfilScrenn.js` | ✅ PUT /api/user/{id} branché |
| `src/screens/GainScreen.js` | ✅ Deux onglets : Historique + Récompenses · Modal QR code |
| `src/screens/ShopScreen.js` | ✅ Boutique · achat par points · modal code · bouton vers Récompenses |
| `src/screens/CalendarScreen.js` | ✅ GET /api/events branché |
| `handlers/reward.rs` | ✅ GET /rewards · POST /rewards/:id/redeem · GET /user/redemptions |
| `handlers/admin.rs` | ✅ POST /admin/scan · POST /admin/validate-redemption |
| `static/admin/index.html` | ✅ PWA caissier : login · scanner QR · valider code |
| `migrations/20260421130000` | ✅ Champ role sur users |
| `migrations/20260421130001` | ✅ Tables rewards + redemptions + seed 5 récompenses |
| `examples/seed.rs` | ✅ Seed complet : 3 comptes · 9 récompenses · tx · codes testables |
| `src/navigation/TabNavigator.js` | ✅ 6 onglets · QR centré · labels · point actif |

---

## Légende

| Icône | Signification |
|-------|---------------|
| 🔴 | Bloquant — ne pas déployer sans fix |
| 🟠 | Important — à faire rapidement |
| 🟡 | Moyen — à planifier |
| ✅ | Terminé |

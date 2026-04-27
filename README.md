# Horizon Moto — Application de fidélité

Application mobile de fidélité pour [horizonmoto.fr](https://horizonmoto.fr).

| Partie | Stack |
|--------|-------|
| API | Rust · Axum 0.8 · SQLx 0.7 · MySQL · JWT · Argon2 |
| Mobile | React Native 0.81 · Expo 54 · EAS Build |

---

## Prérequis

| Outil | Version | Lien |
|-------|---------|------|
| Rust | stable (≥ 1.80) | https://rustup.rs |
| MySQL | 8.x | |
| Node.js | ≥ 20 | https://nodejs.org |
| Expo CLI | dernière | `npm install -g expo-cli` |
| EAS CLI | dernière | `npm install -g eas-cli` |

---

## 1. Lancer l'API Rust

### Configuration

```bash
cd horizion_mobile_api_rust
cp .env.example .env
```

Édite `.env` :

```env
DATABASE_URL=mysql://user:password@localhost:3306/horizon
JWT_SECRET=un-secret-long-et-aleatoire
SERVER_ADDRESS=0.0.0.0
SERVER_PORT=4000
```

### Créer la base de données MySQL

```sql
CREATE DATABASE horizon CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER 'horizon'@'localhost' IDENTIFIED BY 'ton_mot_de_passe';
GRANT ALL PRIVILEGES ON horizon.* TO 'horizon'@'localhost';
FLUSH PRIVILEGES;
```

### Lancer l'API

```bash
# Développement (recompile à chaque changement)
cargo run

# Ou build de production
cargo build --release
./target/release/horizon_api_rust
```

L'API démarre sur `http://0.0.0.0:4000`.  
Les migrations SQLx s'appliquent automatiquement au démarrage.

### Peupler la base (optionnel)

```bash
cargo run --example seed
```

Crée 3 comptes de test :
- `admin@horizonmoto.fr` / `Admin1234!` (rôle admin)
- `client@horizonmoto.fr` / `Client1234!`
- `client2@horizonmoto.fr` / `Client1234!`

### Interface admin caissier (PWA)

Accessible sur `http://localhost:4000/admin` depuis n'importe quel navigateur.

---

## 2. Lancer le front React Native

### Configuration

```bash
cd horizon_mobile_front
npm install
cp .env.example .env
```

Édite `.env` :

```env
EXPO_PUBLIC_API_URL=http://TON_IP_LOCALE:4000/api
```

Pour connaître ton IP locale :
```bash
# Windows
ipconfig | findstr "IPv4"

# macOS / Linux
ifconfig | grep "inet "
```

### Lancer en développement (Expo Go — sans Google OAuth)

```bash
npx expo start
```

> **Attention** : Google OAuth ne fonctionne pas avec Expo Go depuis mai 2024.  
> Pour tester Google OAuth, utilise le Development Build (voir ci-dessous).

### Lancer avec le Development Build (Google OAuth fonctionnel)

```bash
# Connexion EAS (une seule fois)
eas login

# Build Android APK de développement (cloud EAS ~10-15 min)
eas build --profile development --platform android
```

L'APK est téléchargeable via QR code à la fin du build.  
Installe-le sur ton téléphone Android, puis :

```bash
npx expo start --dev-client
```

---

## 3. Variables d'environnement

### API (`horizion_mobile_api_rust/.env`)

| Variable | Description | Exemple |
|----------|-------------|---------|
| `DATABASE_URL` | URL de connexion MySQL | `mysql://user:pass@localhost:3306/horizon` |
| `JWT_SECRET` | Clé secrète JWT (min. 32 chars) | `super-secret-key-change-in-prod` |
| `SERVER_ADDRESS` | Adresse d'écoute | `0.0.0.0` |
| `SERVER_PORT` | Port d'écoute | `4000` |

### Front (`horizon_mobile_front/.env`)

| Variable | Description | Exemple |
|----------|-------------|---------|
| `EXPO_PUBLIC_API_URL` | URL de l'API accessible depuis le téléphone | `http://192.168.1.10:4000/api` |

---

## 4. Endpoints API

### Auth
| Méthode | Route | Description |
|---------|-------|-------------|
| POST | `/api/register` | Créer un compte |
| POST | `/api/login` | Connexion email/password |
| POST | `/api/auth/google` | Connexion Google OAuth |

### Utilisateur (JWT requis)
| Méthode | Route | Description |
|---------|-------|-------------|
| GET | `/api/user/{id}` | Profil utilisateur |
| PUT | `/api/user/{id}` | Modifier email/password |
| GET | `/api/user/points` | Points et rang |
| GET | `/api/user/transactions` | Historique des points |
| GET | `/api/user/redemptions` | Codes de rachat |
| GET | `/api/user/qrcode-token` | Token QR (10 min) |

### Boutique (JWT requis)
| Méthode | Route | Description |
|---------|-------|-------------|
| GET | `/api/rewards` | Catalogue des récompenses |
| POST | `/api/rewards/{id}/redeem` | Échanger des points |

### Événements (JWT requis)
| Méthode | Route | Description |
|---------|-------|-------------|
| GET | `/api/events` | Liste des événements calendrier |

### Admin (JWT admin requis)
| Méthode | Route | Description |
|---------|-------|-------------|
| POST | `/api/admin/scan` | Scanner QR client + ajouter points |
| POST | `/api/admin/customer-info` | Infos client depuis token QR |
| POST | `/api/admin/validate-redemption` | Valider un code HRZ-XXXX-XXXX |

---

## 5. Système de rangs

| Rang | Points | Couleur |
|------|--------|---------|
| Bronze | 0 — 499 | `#CD7F32` |
| Silver | 500 — 999 | `#C0C0C0` |
| Gold | 1 000 — 2 499 | `#CFB53B` |
| Platine | 2 500 — 4 999 | `#00FFD4` |
| Diamond | 5 000+ | `#A78BFA` |

---

## 6. Build de production

```bash
# APK Android (distribution directe)
eas build --profile preview --platform android

# AAB Android (Google Play Store)
eas build --profile production --platform android
```

> Avant un build de production, pense à :
> - Changer `EXPO_PUBLIC_API_URL` vers l'URL publique de l'API
> - Mettre à jour le `JWT_SECRET` vers une valeur sécurisée
> - Configurer le SHA-1 du keystore de production dans Google Cloud Console
-- Vérification email à l'inscription
ALTER TABLE users
    ADD COLUMN email_verified TINYINT(1) NOT NULL DEFAULT 0;

-- Comptes existants : considérés comme vérifiés (rétrocompatibilité, pas
-- de blocage du login pour les utilisateurs déjà créés).
UPDATE users SET email_verified = 1;

CREATE TABLE otp_codes (
    id          BINARY(16)                          NOT NULL PRIMARY KEY,
    user_id     BINARY(16)                          NOT NULL,
    purpose     ENUM('verify_email')                NOT NULL DEFAULT 'verify_email',
    code_hash   CHAR(64)                            NOT NULL,
    expires_at  DATETIME                            NOT NULL,
    used_at     DATETIME                            NULL,
    created_at  DATETIME                            NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_otp_code_hash    (code_hash),
    INDEX idx_otp_user_purpose (user_id, purpose),
    CONSTRAINT fk_otp_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE refresh_tokens (
    id           BINARY(16)   NOT NULL PRIMARY KEY,
    user_id      BINARY(16)   NOT NULL,
    token_hash   CHAR(64)     NOT NULL UNIQUE,
    issued_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at   DATETIME     NOT NULL,
    revoked_at   DATETIME     NULL,
    INDEX idx_rt_user (user_id),
    CONSTRAINT fk_rt_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

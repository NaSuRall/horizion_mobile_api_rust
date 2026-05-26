CREATE TABLE password_reset_tokens (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    user_id     BINARY(16)  NOT NULL,
    token_hash  CHAR(64)    NOT NULL,
    expires_at  DATETIME    NOT NULL,
    used_at     DATETIME    NULL,
    created_at  DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_prt_token_hash (token_hash),
    INDEX idx_prt_user_id    (user_id),
    CONSTRAINT fk_prt_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE used_qr_tokens (
    jti        CHAR(36)  NOT NULL PRIMARY KEY,
    used_at    DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_used_qr_used_at (used_at)
);

CREATE TABLE transactions (
    id       BINARY(16)   NOT NULL PRIMARY KEY,
    user_id  BINARY(16)   NOT NULL,
    points   INT          NOT NULL,
    label    VARCHAR(255) NOT NULL,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_tx_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

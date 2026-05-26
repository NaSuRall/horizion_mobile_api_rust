ALTER TABLE users
    ADD COLUMN login_attempts INT      NOT NULL DEFAULT 0,
    ADD COLUMN locked_until   DATETIME NULL;

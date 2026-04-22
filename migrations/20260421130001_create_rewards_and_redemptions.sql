CREATE TABLE rewards (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    description TEXT,
    image_url   VARCHAR(255),
    point_cost  INT          NOT NULL,
    stock       INT          NOT NULL DEFAULT -1,
    active      TINYINT(1)   NOT NULL DEFAULT 1,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE redemptions (
    id         BINARY(16)                              NOT NULL PRIMARY KEY,
    user_id    BINARY(16)                              NOT NULL,
    reward_id  INT                                     NOT NULL,
    code       VARCHAR(20)                             NOT NULL,
    status     ENUM('pending', 'used', 'expired')      NOT NULL DEFAULT 'pending',
    created_at DATETIME                                NOT NULL DEFAULT CURRENT_TIMESTAMP,
    used_at    DATETIME,
    UNIQUE KEY uk_redemptions_code (code),
    CONSTRAINT fk_redemptions_user   FOREIGN KEY (user_id)   REFERENCES users(id),
    CONSTRAINT fk_redemptions_reward FOREIGN KEY (reward_id) REFERENCES rewards(id)
);

INSERT INTO rewards (name, description, point_cost, stock, active) VALUES
('Casque moto basique',    'Casque homologué ECE 22.06, idéal pour la ville',         2000, 10,  1),
('Gants cuir été',         'Gants cuir protection CE niveau 1',                         800, 20,  1),
('Gilet airbag',           'Protection dorsale et thoracique intégrée',                5000,  5,  1),
('Kit nettoyage moto',     'Produits entretien complet pour votre moto',                300, 50,  1),
('Bon d''achat 20€',       'Bon d''achat valable en magasin',                           500, -1,  1);

CREATE TABLE events (
    id         INT AUTO_INCREMENT NOT NULL PRIMARY KEY,
    title      VARCHAR(255) NOT NULL,
    location   VARCHAR(255) NOT NULL,
    badge      VARCHAR(100) NOT NULL,
    badge_type VARCHAR(20)  NOT NULL DEFAULT 'green',
    event_date DATE         NOT NULL
);

INSERT INTO events (title, location, badge, badge_type, event_date) VALUES
('Journée portes ouvertes', 'Magasin Horizon Moto - 10h à 18h', '3x points',    'red',   '2026-05-05'),
('Balade printanière',      'Départ parking - 8h00',            'Accès gratuit', 'green', '2026-05-12'),
('Essai nouveaux modèles',  'Piste intérieure - 14h à 17h',     'Accès VIP',     'green', '2026-06-20');

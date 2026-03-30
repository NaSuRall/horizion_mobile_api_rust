-- Add migration script here
ALTER TABLE users
    ADD COLUMN `rank` ENUM('Bronze', 'Silver', 'Gold', 'Platine', 'Diamond')
NOT NULL DEFAULT 'Bronze';

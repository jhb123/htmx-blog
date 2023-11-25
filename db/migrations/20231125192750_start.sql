-- Add migration script here
CREATE TABLE IF NOT EXISTS users 
(
    id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    session INT,
    privilege INT,
    name TEXT
);

INSERT INTO users (id, privilege, name) VALUES (0, 1, "admin")

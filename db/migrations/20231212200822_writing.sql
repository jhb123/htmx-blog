-- Add migration script here
CREATE TABLE IF NOT EXISTS writing 
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    creation_date DATETIME DEFAULT CURRENT_TIMESTAMP,
    published_date DATETIME,
    is_published INTEGER,
    visits INT,
    title TEXT,
    title_image TEXT,
    blurb TEXT
);
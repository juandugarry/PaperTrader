CREATE TABLE asx_directory (
    id INTEGER PRIMARY KEY CHECK(id=1),
    body TEXT NOT NULL,
    fetched_at TEXT NOT NULL
);

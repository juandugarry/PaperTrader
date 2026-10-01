ALTER TABLE settings ADD COLUMN profile_id TEXT CHECK(profile_id IS NULL OR length(profile_id)=32);
UPDATE settings SET profile_id=lower(hex(randomblob(16)));
CREATE TABLE notes (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 120),
    body TEXT NOT NULL CHECK(length(body)<=50000),
    version INTEGER NOT NULL CHECK(version>0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

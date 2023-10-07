-- Add migration script here
CREATE TABLE IF NOT EXISTS eve_stargates (
    id INTEGER PRIMARY KEY,
    source_system_id INTEGER NOT NULL REFERENCES eve_system(id) ON DELETE CASCADE,
    target_system_id INTEGER NOT NULL REFERENCES eve_system(id) ON DELETE CASCADE
);
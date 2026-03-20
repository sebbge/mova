-- single row, id 1
CREATE TABLE IF NOT EXISTS gtfs_metadata (
    id        INTEGER     PRIMARY KEY DEFAULT 1,
    version   TEXT        NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL
);

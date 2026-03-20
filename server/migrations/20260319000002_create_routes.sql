CREATE TABLE IF NOT EXISTS routes (
    route_id         TEXT PRIMARY KEY,
    route_short_name TEXT NOT NULL,
    route_long_name  TEXT,
    agency_id        TEXT
);

CREATE INDEX IF NOT EXISTS idx_routes_short_name ON routes (route_short_name);

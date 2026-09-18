CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS stops (
    stop_id   TEXT PRIMARY KEY,
    stop_name TEXT NOT NULL,
    geom      GEOGRAPHY(Point, 4326) NOT NULL,
    bearing_rad FLOAT8
);

CREATE INDEX IF NOT EXISTS idx_stops_geom ON stops USING GIST (geom);

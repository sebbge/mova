CREATE TABLE IF NOT EXISTS shapes (
    shape_id TEXT PRIMARY KEY,
    geom     GEOMETRY(LineString, 4326) NOT NULL
);

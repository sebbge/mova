CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- `region` issues the id, `area` is where the point lies
CREATE TABLE areas (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    geom GEOMETRY(Geometry, 4326) NOT NULL
);

CREATE INDEX idx_areas_geom ON areas USING GIST (geom);

-- same bounds in small pieces, for the index
CREATE TABLE area_parts (
    code TEXT NOT NULL REFERENCES areas(code),
    geom GEOMETRY(Geometry, 4326) NOT NULL
);

CREATE INDEX idx_area_parts_geom ON area_parts USING GIST (geom);

-- bus terminal outlines, OpenStreetMap (ODbL)
CREATE TABLE terminals (
    osm_id   TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    geom     GEOMETRY(Polygon, 4326) NOT NULL,
    region   TEXT,
    provider TEXT,
    stop_id  TEXT
);

CREATE INDEX idx_terminals_geom ON terminals USING GIST (geom);
CREATE INDEX idx_terminals_stop ON terminals (region, provider, stop_id);

CREATE TABLE places (
    id       TEXT PRIMARY KEY,
    kind     TEXT NOT NULL,
    name     TEXT NOT NULL,
    category TEXT NOT NULL,
    context  TEXT,
    key      TEXT NOT NULL,
    crossing TEXT,
    rank     SMALLINT NOT NULL,
    geom     GEOGRAPHY(Point, 4326) NOT NULL,
    area     TEXT REFERENCES areas(code)
);

CREATE INDEX idx_places_key ON places USING GIN (key gin_trgm_ops);
CREATE INDEX idx_places_crossing ON places USING GIN (crossing gin_trgm_ops) WHERE crossing IS NOT NULL;
CREATE INDEX idx_places_geom ON places USING GIST (geom);

CREATE TABLE places_metadata (
    source    TEXT NOT NULL,
    loaded_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE gtfs_metadata (
    provider  TEXT        PRIMARY KEY,
    version   TEXT        NOT NULL,
    synced_at TIMESTAMPTZ NOT NULL
);

-- `cluster` leading stop of a shared shelter, `zonal` zone marker
-- `station`, `platform` for stops the provider marks as platforms
CREATE TABLE stops (
    region      TEXT NOT NULL,
    provider    TEXT NOT NULL,
    stop_id     TEXT NOT NULL,
    stop_name   TEXT NOT NULL,
    geom        GEOGRAPHY(Point, 4326) NOT NULL,
    bearing_rad FLOAT8,
    area        TEXT REFERENCES areas(code),
    cluster     TEXT,
    zonal       BOOLEAN NOT NULL DEFAULT FALSE,
    station     TEXT,
    platform    TEXT,
    PRIMARY KEY (region, provider, stop_id)
);

CREATE INDEX idx_stops_geom ON stops USING GIST (geom);
CREATE INDEX idx_stops_provider ON stops (provider);
CREATE INDEX idx_stops_area ON stops (area);
CREATE INDEX idx_stops_cluster ON stops (region, cluster);
CREATE INDEX idx_stops_zonal ON stops (provider) WHERE zonal;

CREATE TABLE routes (
    region           TEXT NOT NULL,
    provider         TEXT NOT NULL,
    route_id         TEXT NOT NULL,
    route_short_name TEXT NOT NULL,
    route_long_name  TEXT,
    agency_id        TEXT,
    area             TEXT REFERENCES areas(code),
    PRIMARY KEY (region, provider, route_id)
);

CREATE INDEX idx_routes_short_name ON routes (region, route_short_name);
CREATE INDEX idx_routes_provider ON routes (provider);
CREATE INDEX idx_routes_area ON routes (area);

CREATE TABLE shapes (
    region   TEXT NOT NULL,
    provider TEXT NOT NULL,
    shape_id TEXT NOT NULL,
    geom     GEOMETRY(LineString, 4326) NOT NULL,
    PRIMARY KEY (region, provider, shape_id)
);

CREATE INDEX idx_shapes_provider ON shapes (provider);

CREATE TABLE calendar (
    region     TEXT    NOT NULL,
    provider   TEXT    NOT NULL,
    service_id TEXT    NOT NULL,
    monday     BOOLEAN NOT NULL,
    tuesday    BOOLEAN NOT NULL,
    wednesday  BOOLEAN NOT NULL,
    thursday   BOOLEAN NOT NULL,
    friday     BOOLEAN NOT NULL,
    saturday   BOOLEAN NOT NULL,
    sunday     BOOLEAN NOT NULL,
    start_date DATE    NOT NULL,
    end_date   DATE    NOT NULL,
    PRIMARY KEY (region, provider, service_id)
);

CREATE INDEX idx_calendar_provider ON calendar (provider);

CREATE TABLE trips (
    region        TEXT     NOT NULL,
    provider      TEXT     NOT NULL,
    trip_id       TEXT     NOT NULL,
    route_id      TEXT     NOT NULL,
    service_id    TEXT     NOT NULL,
    shape_id      TEXT,
    direction_id  SMALLINT,
    trip_headsign TEXT,
    starts        INTERVAL,
    ends          INTERVAL,
    PRIMARY KEY (region, provider, trip_id)
);

CREATE INDEX idx_trips_route ON trips (region, provider, route_id);
CREATE INDEX idx_trips_shape ON trips (region, provider, shape_id);
CREATE INDEX idx_trips_provider ON trips (provider);

CREATE TABLE stop_times (
    region         TEXT     NOT NULL,
    provider       TEXT     NOT NULL,
    trip_id        TEXT     NOT NULL,
    stop_id        TEXT     NOT NULL,
    stop_sequence  INTEGER  NOT NULL,
    arrival_time   INTERVAL NOT NULL,
    departure_time INTERVAL NOT NULL,
    zonal          BOOLEAN  NOT NULL DEFAULT FALSE,
    interpolated   BOOLEAN  NOT NULL DEFAULT FALSE,
    PRIMARY KEY (region, provider, trip_id, stop_sequence)
);

CREATE INDEX idx_stop_times_stop ON stop_times (region, provider, stop_id);

CREATE TABLE stop_lines (
    region   TEXT NOT NULL,
    provider TEXT NOT NULL,
    stop_id  TEXT NOT NULL,
    line     TEXT NOT NULL,
    PRIMARY KEY (region, provider, stop_id, line)
);

CREATE INDEX idx_stop_lines_stop ON stop_lines (region, provider, stop_id);

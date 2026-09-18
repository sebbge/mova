CREATE TABLE IF NOT EXISTS trips (
    trip_id       TEXT     PRIMARY KEY,
    route_id      TEXT     NOT NULL REFERENCES routes(route_id),
    service_id    TEXT     NOT NULL REFERENCES calendar(service_id),
    shape_id      TEXT     REFERENCES shapes(shape_id),
    direction_id  SMALLINT,
    trip_headsign TEXT
);

CREATE INDEX IF NOT EXISTS idx_trips_route_id ON trips (route_id);

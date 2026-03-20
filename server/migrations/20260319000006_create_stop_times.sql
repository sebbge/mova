-- INTERVAL for values past 24h
CREATE TABLE IF NOT EXISTS stop_times (
    trip_id        TEXT     NOT NULL REFERENCES trips(trip_id),
    stop_id        TEXT     NOT NULL REFERENCES stops(stop_id),
    stop_sequence  INTEGER  NOT NULL,
    arrival_time   INTERVAL NOT NULL,
    departure_time INTERVAL NOT NULL,
    PRIMARY KEY (trip_id, stop_sequence)
);

CREATE INDEX IF NOT EXISTS idx_stop_times_stop_id ON stop_times (stop_id);

ALTER TABLE trips ADD COLUMN starts INTERVAL, ADD COLUMN ends INTERVAL;

UPDATE trips t SET starts = s.starts, ends = s.ends
FROM (
  SELECT region, provider, trip_id, min(departure_time) AS starts, max(arrival_time) AS ends
  FROM stop_times
  GROUP BY region, provider, trip_id
) s
WHERE t.region = s.region AND t.provider = s.provider AND t.trip_id = s.trip_id;

CREATE INDEX idx_trips_shape ON trips (region, provider, shape_id);

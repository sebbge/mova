ALTER TABLE stop_times ADD COLUMN interpolated BOOLEAN NOT NULL DEFAULT FALSE;

-- forces a resync
DELETE FROM gtfs_metadata;

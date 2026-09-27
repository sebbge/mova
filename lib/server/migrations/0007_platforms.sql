-- `station`, `platform` for stops the provider marks as platforms
-- drops the STM version to re-ingest it
ALTER TABLE stops ADD COLUMN station TEXT, ADD COLUMN platform TEXT;

DELETE FROM gtfs_metadata WHERE provider = 'stm';

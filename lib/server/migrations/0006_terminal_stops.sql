ALTER TABLE terminals ADD COLUMN region TEXT, ADD COLUMN provider TEXT, ADD COLUMN stop_id TEXT;

CREATE INDEX idx_terminals_stop ON terminals (region, provider, stop_id);

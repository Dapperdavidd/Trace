-- Metadata is evidence with an explicit lookup state, not a guessed label.
ALTER TABLE tokens ADD COLUMN metadata_status TEXT NOT NULL DEFAULT 'unknown'
  CHECK (metadata_status IN ('unknown','complete','unavailable'));
ALTER TABLE tokens ADD COLUMN metadata_checked_at TIMESTAMPTZ;

-- One watched-wallet event may be scanned repeatedly. Delivery stays idempotent.
ALTER TABLE alerts ADD COLUMN dedupe_key TEXT UNIQUE;

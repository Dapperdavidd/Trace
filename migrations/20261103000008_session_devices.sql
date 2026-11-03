-- A coarse device label helps users identify and revoke sessions without
-- storing full user-agent strings or client IP addresses.
ALTER TABLE app_sessions ADD COLUMN device_label TEXT NOT NULL DEFAULT 'Unknown device';

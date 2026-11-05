-- Support bounded retention scans without full-table sorts.
CREATE INDEX raw_chain_events_retention
    ON raw_chain_events (ingested_at, id) WHERE fixture = FALSE;
CREATE INDEX job_attempts_retention
    ON job_attempts (finished_at, id) WHERE finished_at IS NOT NULL;
CREATE INDEX audit_events_retention ON audit_events (created_at, id);

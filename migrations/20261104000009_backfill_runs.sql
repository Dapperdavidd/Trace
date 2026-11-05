-- Bounded, resumable historical EVM ingestion. This progress is deliberately
-- separate from ingestion_cursors so a backfill cannot move the live cursor.

CREATE TABLE backfill_runs (
    id            UUID PRIMARY KEY,
    chain         TEXT NOT NULL REFERENCES chains(chain),
    protocol      TEXT NOT NULL,
    from_height   BIGINT NOT NULL CHECK (from_height >= 0),
    to_height     BIGINT NOT NULL CHECK (to_height >= from_height),
    next_height   BIGINT NOT NULL CHECK (next_height >= from_height),
    status        TEXT NOT NULL CHECK (status IN ('queued','running','completed','cancelled')),
    last_error    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at  TIMESTAMPTZ,
    UNIQUE (chain, protocol, from_height, to_height),
    CHECK (next_height <= to_height + 1)
);

CREATE INDEX backfill_runs_pending
    ON backfill_runs (status, updated_at)
    WHERE status IN ('queued','running');

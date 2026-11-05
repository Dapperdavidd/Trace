-- Bounded, resumable Solana signature backfill, separate from the live cursor.

CREATE TABLE solana_backfill_runs (
    id                UUID PRIMARY KEY,
    protocol          TEXT NOT NULL,
    before_signature  TEXT NOT NULL,
    until_signature   TEXT NOT NULL,
    next_before       TEXT NOT NULL,
    max_signatures    BIGINT NOT NULL CHECK (max_signatures BETWEEN 1 AND 1000000),
    processed         BIGINT NOT NULL DEFAULT 0 CHECK (processed >= 0),
    status            TEXT NOT NULL CHECK (status IN ('queued','running','completed','limit_reached','cancelled')),
    last_error        TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at      TIMESTAMPTZ,
    UNIQUE (protocol, before_signature, until_signature)
);

CREATE INDEX solana_backfill_runs_pending
    ON solana_backfill_runs (status, updated_at)
    WHERE status IN ('queued','running');

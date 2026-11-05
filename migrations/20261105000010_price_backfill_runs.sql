-- Bounded and resumable historical token-price intake.

CREATE TABLE price_backfill_runs (
    id               UUID PRIMARY KEY,
    chain            TEXT NOT NULL,
    asset            TEXT NOT NULL,
    from_time        TIMESTAMPTZ NOT NULL,
    to_time          TIMESTAMPTZ NOT NULL CHECK (to_time >= from_time),
    next_time        TIMESTAMPTZ NOT NULL CHECK (next_time >= from_time),
    interval_seconds BIGINT NOT NULL CHECK (interval_seconds BETWEEN 300 AND 86400),
    status           TEXT NOT NULL CHECK (status IN ('queued','running','completed','cancelled')),
    stored_points    BIGINT NOT NULL DEFAULT 0 CHECK (stored_points >= 0),
    rejected_points  BIGINT NOT NULL DEFAULT 0 CHECK (rejected_points >= 0),
    last_error       TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at     TIMESTAMPTZ,
    FOREIGN KEY (chain, asset) REFERENCES tokens(chain, contract_or_mint),
    UNIQUE (chain, asset, from_time, to_time, interval_seconds)
);

CREATE INDEX price_backfill_runs_pending
    ON price_backfill_runs (status, updated_at)
    WHERE status IN ('queued','running');

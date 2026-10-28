-- TRACE foundation schema. Forward-only. Exact numerics, chain-aware identities.

CREATE TABLE chains (
    chain            TEXT PRIMARY KEY CHECK (chain IN ('solana','ethereum','base','bnb')),
    family           TEXT NOT NULL CHECK (family IN ('svm','evm')),
    evm_chain_id     BIGINT UNIQUE,
    display_name     TEXT NOT NULL,
    finality_policy  TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO chains (chain, family, evm_chain_id, display_name, finality_policy) VALUES
  ('solana',   'svm', NULL,  'Solana',    'finalized commitment only; confirmed treated as tentative'),
  ('ethereum', 'evm', 1,     'Ethereum',  'finalized block tag; unfinalized treated as tentative'),
  ('base',     'evm', 8453,  'Base',      'safe/finalized tag; unfinalized treated as tentative'),
  ('bnb',      'evm', 56,    'BNB Chain', 'N confirmations configured; below N treated as tentative');

CREATE TABLE protocols (
    chain                    TEXT NOT NULL REFERENCES chains(chain),
    protocol                 TEXT NOT NULL,
    support_status           TEXT NOT NULL CHECK (support_status IN ('enabled','partial','disabled','unsupported')),
    limitations              TEXT[] NOT NULL DEFAULT '{}',
    blocker                  TEXT,
    last_successful_scan_at  TIMESTAMPTZ,
    lookback_blocks          BIGINT,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chain, protocol)
);

CREATE TABLE coverage_snapshots (
    id            BIGSERIAL PRIMARY KEY,
    chain         TEXT NOT NULL REFERENCES chains(chain),
    protocol      TEXT,
    captured_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    data_status   TEXT NOT NULL CHECK (data_status IN ('live','stale','unavailable','partial','demo')),
    detail        JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX coverage_snapshots_chain_time ON coverage_snapshots (chain, captured_at DESC);

CREATE TABLE provider_quotas (
    provider           TEXT NOT NULL,
    chain              TEXT REFERENCES chains(chain),
    window_start       TIMESTAMPTZ NOT NULL,
    window_seconds     INTEGER NOT NULL CHECK (window_seconds > 0),
    requests_made      BIGINT NOT NULL DEFAULT 0,
    request_budget     BIGINT,
    last_status        TEXT,
    last_failure_class TEXT CHECK (last_failure_class IN ('transient','permanent')),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, window_start)
);

CREATE TABLE audit_events (
    id          UUID PRIMARY KEY,
    actor       TEXT NOT NULL,
    action      TEXT NOT NULL,
    subject     TEXT,
    detail      JSONB NOT NULL DEFAULT '{}',
    request_id  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

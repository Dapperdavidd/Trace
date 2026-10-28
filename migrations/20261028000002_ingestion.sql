-- Raw evidence, canonical swap ledger, cursors and jobs.

CREATE TABLE tokens (
    chain            TEXT NOT NULL REFERENCES chains(chain),
    contract_or_mint TEXT NOT NULL,
    symbol           TEXT,
    decimals         SMALLINT,
    first_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chain, contract_or_mint)
);

CREATE TABLE wallets (
    chain       TEXT NOT NULL REFERENCES chains(chain),
    address     TEXT NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chain, address)
);

-- Raw provider payloads kept for replay. One row per natural event identity.
CREATE TABLE raw_chain_events (
    id               BIGSERIAL PRIMARY KEY,
    chain            TEXT NOT NULL REFERENCES chains(chain),
    tx_id            TEXT NOT NULL,
    event_index      INTEGER NOT NULL,
    finality_revision INTEGER NOT NULL DEFAULT 0,
    source           TEXT NOT NULL,
    payload          JSONB NOT NULL,
    payload_sha256   TEXT NOT NULL,
    fixture          BOOLEAN NOT NULL DEFAULT FALSE,
    observed_at      TIMESTAMPTZ NOT NULL,
    ingested_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (chain, tx_id, event_index, finality_revision)
);

CREATE TABLE normalized_swaps (
    id                BIGSERIAL PRIMARY KEY,
    chain             TEXT NOT NULL REFERENCES chains(chain),
    tx_id             TEXT NOT NULL,
    event_index       INTEGER NOT NULL,
    finality_revision INTEGER NOT NULL DEFAULT 0,
    wallet            TEXT NOT NULL,
    protocol          TEXT NOT NULL,
    pool              TEXT NOT NULL,
    token_in          TEXT NOT NULL,
    amount_in         NUMERIC(40,0) NOT NULL CHECK (amount_in > 0),
    token_out         TEXT NOT NULL,
    amount_out        NUMERIC(40,0) NOT NULL CHECK (amount_out > 0),
    block_height      BIGINT,
    block_hash        TEXT,
    observed_at       TIMESTAMPTZ NOT NULL,
    ingested_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    finality          TEXT NOT NULL CHECK (finality IN ('tentative','final','reorged')),
    attribution       TEXT NOT NULL CHECK (attribution IN ('direct','routed','ambiguous','infrastructure')),
    parser_version    TEXT NOT NULL,
    raw_ref           TEXT NOT NULL,
    demo              BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (chain, tx_id, event_index, finality_revision),
    FOREIGN KEY (chain, wallet) REFERENCES wallets(chain, address),
    FOREIGN KEY (chain, token_in) REFERENCES tokens(chain, contract_or_mint),
    FOREIGN KEY (chain, token_out) REFERENCES tokens(chain, contract_or_mint)
);
CREATE INDEX normalized_swaps_wallet_time ON normalized_swaps (chain, wallet, observed_at);
CREATE INDEX normalized_swaps_token_time ON normalized_swaps (chain, token_out, observed_at);
CREATE INDEX normalized_swaps_pool_time ON normalized_swaps (chain, pool, observed_at);

CREATE TABLE ingestion_cursors (
    chain        TEXT NOT NULL REFERENCES chains(chain),
    protocol     TEXT NOT NULL,
    cursor       TEXT NOT NULL,
    height       BIGINT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (chain, protocol)
);

CREATE TABLE worker_jobs (
    id            UUID PRIMARY KEY,
    kind          TEXT NOT NULL,
    dedupe_key    TEXT UNIQUE,
    payload       JSONB NOT NULL DEFAULT '{}',
    status        TEXT NOT NULL CHECK (status IN ('queued','running','succeeded','failed','dead')),
    attempts      INTEGER NOT NULL DEFAULT 0,
    max_attempts  INTEGER NOT NULL DEFAULT 5,
    run_after     TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until  TIMESTAMPTZ,
    last_error    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX worker_jobs_due ON worker_jobs (status, run_after);

CREATE TABLE job_attempts (
    id           BIGSERIAL PRIMARY KEY,
    job_id       UUID NOT NULL REFERENCES worker_jobs(id),
    started_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at  TIMESTAMPTZ,
    outcome      TEXT,
    failure_class TEXT CHECK (failure_class IN ('transient','permanent')),
    error        TEXT
);

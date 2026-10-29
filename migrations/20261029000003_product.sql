-- Product schema: accounts, sessions, watchlists, alerts, market snapshots,
-- prices, signals and shadow strategies. Forward-only.

ALTER TABLE tokens ADD COLUMN name TEXT;
ALTER TABLE tokens ADD COLUMN is_stable BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE users (
    id          UUID PRIMARY KEY,
    email       TEXT NOT NULL,
    name        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ
);
CREATE UNIQUE INDEX users_email_live ON users (lower(email)) WHERE deleted_at IS NULL;

CREATE TABLE oauth_identities (
    provider  TEXT NOT NULL,
    subject   TEXT NOT NULL,
    user_id   UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (provider, subject)
);

-- Only a SHA-256 of the bearer token is stored.
CREATE TABLE app_sessions (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ
);
CREATE INDEX app_sessions_user ON app_sessions (user_id);

CREATE TABLE watchlist_wallets (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chain            TEXT NOT NULL,
    address          TEXT NOT NULL,
    alias            TEXT,
    paused           BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_checked_at  TIMESTAMPTZ,
    UNIQUE (user_id, chain, address),
    FOREIGN KEY (chain, address) REFERENCES wallets(chain, address)
);

CREATE TABLE alert_preferences (
    user_id     UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    wallet_buy  BOOLEAN NOT NULL DEFAULT TRUE,
    wallet_sell BOOLEAN NOT NULL DEFAULT TRUE,
    risk_event  BOOLEAN NOT NULL DEFAULT TRUE,
    signal      BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE alerts (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type          TEXT NOT NULL CHECK (type IN ('wallet_buy','wallet_sell','risk_event','signal')),
    title         TEXT NOT NULL,
    body          TEXT NOT NULL,
    evidence_tx   TEXT[] NOT NULL DEFAULT '{}',
    event_time    TIMESTAMPTZ NOT NULL,
    published_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    demo          BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX alerts_user_time ON alerts (user_id, published_at DESC, id);

-- Point-in-time prices. Never interpolated: lookups take the nearest point
-- within a stated tolerance or report no price.
CREATE TABLE price_points (
    chain             TEXT NOT NULL,
    asset             TEXT NOT NULL,
    observed_at       TIMESTAMPTZ NOT NULL,
    usd_per_base_unit NUMERIC(60,30) NOT NULL CHECK (usd_per_base_unit > 0),
    source            TEXT NOT NULL,
    demo              BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (chain, asset, observed_at, source),
    FOREIGN KEY (chain, asset) REFERENCES tokens(chain, contract_or_mint)
);

-- One row per token pool and window, written by the market worker.
CREATE TABLE market_snapshots (
    chain                     TEXT NOT NULL,
    token                     TEXT NOT NULL,
    pool                      TEXT NOT NULL,
    win                       TEXT NOT NULL CHECK (win IN ('1h','24h','7d')),
    price_now_usd             NUMERIC(38,18) NOT NULL,
    price_window_start_usd    NUMERIC(38,18) NOT NULL,
    volume_window_usd         NUMERIC(38,6) NOT NULL,
    volume_baseline_usd       NUMERIC(38,6),
    liquidity_usd             NUMERIC(38,6) NOT NULL,
    market_cap_usd            NUMERIC(38,6),
    holders                   INTEGER,
    pool_age_hours            INTEGER,
    independent_smart_buyers  INTEGER NOT NULL DEFAULT 0,
    large_buyer_flows         INTEGER NOT NULL DEFAULT 0,
    sparkline                 JSONB NOT NULL DEFAULT '[]',
    risk_inputs               JSONB NOT NULL DEFAULT '{}',
    last_updated              TIMESTAMPTZ NOT NULL,
    demo                      BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (chain, token, pool, win),
    FOREIGN KEY (chain, token) REFERENCES tokens(chain, contract_or_mint)
);

CREATE TABLE signals (
    id                UUID PRIMARY KEY,
    wallet_chain      TEXT NOT NULL,
    wallet_address    TEXT NOT NULL,
    token_chain       TEXT NOT NULL,
    token             TEXT NOT NULL,
    receipt           JSONB NOT NULL,
    receipt_hash      TEXT NOT NULL,
    state             TEXT NOT NULL,
    published_at      TIMESTAMPTZ NOT NULL,
    demo              BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX signals_time ON signals (published_at DESC, id);

CREATE TABLE signal_events (
    signal_id  UUID NOT NULL REFERENCES signals(id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    state      TEXT NOT NULL,
    at         TIMESTAMPTZ NOT NULL,
    note       TEXT,
    PRIMARY KEY (signal_id, seq)
);

-- Outcomes are append-only: one row per horizon, never replaced.
CREATE TABLE signal_outcomes (
    signal_id     UUID NOT NULL REFERENCES signals(id) ON DELETE CASCADE,
    horizon_secs  BIGINT NOT NULL,
    kind          TEXT NOT NULL,
    price_usd     NUMERIC(38,18),
    return_pct    NUMERIC(20,6),
    evaluated_at  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (signal_id, horizon_secs)
);

CREATE TABLE shadow_strategies (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chain       TEXT NOT NULL,
    address     TEXT NOT NULL,
    params      JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX shadow_strategies_user ON shadow_strategies (user_id, created_at DESC);

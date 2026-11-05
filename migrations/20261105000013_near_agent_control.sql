-- NEAR-native agent mandates and retained dry quote evidence.

CREATE TABLE near_agent_mandates (
    id                   UUID PRIMARY KEY,
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shadow_strategy_id   UUID NOT NULL REFERENCES shadow_strategies(id) ON DELETE CASCADE,
    owner_near_account   TEXT NOT NULL,
    agent_near_account   TEXT NOT NULL,
    mode                 TEXT NOT NULL CHECK (mode IN ('paper','approval_required','delegated')),
    status               TEXT NOT NULL CHECK (status IN ('draft','anchored','paused','revoked','expired')),
    allowed_chains       TEXT[] NOT NULL,
    allowed_assets       JSONB NOT NULL,
    max_trade_usd        NUMERIC(38,6) NOT NULL CHECK (max_trade_usd > 0),
    max_daily_usd        NUMERIC(38,6) NOT NULL CHECK (max_daily_usd >= max_trade_usd),
    max_slippage_bps     INTEGER NOT NULL CHECK (max_slippage_bps BETWEEN 0 AND 2000),
    expires_at           TIMESTAMPTZ NOT NULL,
    nonce                BIGINT NOT NULL CHECK (nonce >= 0),
    commitment           TEXT NOT NULL,
    near_contract_id     TEXT,
    anchor_tx_hash       TEXT,
    anchored_at          TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, shadow_strategy_id)
);
CREATE INDEX near_agent_mandates_user
    ON near_agent_mandates (user_id, created_at DESC, id);

CREATE TABLE near_intent_previews (
    id               UUID PRIMARY KEY,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mandate_id       UUID NOT NULL REFERENCES near_agent_mandates(id) ON DELETE CASCADE,
    correlation_id   TEXT NOT NULL,
    request           JSONB NOT NULL,
    response          JSONB NOT NULL,
    response_signature TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX near_intent_previews_mandate
    ON near_intent_previews (mandate_id, created_at DESC, id);


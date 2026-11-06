-- Revocable, least-privilege credentials for external agents.

CREATE TABLE agent_connections (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    token_hash      TEXT NOT NULL UNIQUE,
    token_hint      TEXT NOT NULL,
    scopes          TEXT[] NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    last_used_at    TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    CHECK (char_length(name) BETWEEN 1 AND 60),
    CHECK (scopes <@ ARRAY['observe','propose']::TEXT[]),
    CHECK (cardinality(scopes) > 0)
);
CREATE INDEX agent_connections_user
    ON agent_connections (user_id, created_at DESC, id);

ALTER TABLE near_intent_previews
    ADD COLUMN agent_connection_id UUID REFERENCES agent_connections(id) ON DELETE SET NULL;

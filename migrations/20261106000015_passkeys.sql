-- Passkey-first accounts. Ceremony state stays server-side and expires quickly.

ALTER TABLE users ALTER COLUMN email DROP NOT NULL;
ALTER TABLE users ADD COLUMN username TEXT;
CREATE UNIQUE INDEX users_username_live
    ON users (lower(username)) WHERE deleted_at IS NULL AND username IS NOT NULL;

CREATE TABLE passkey_credentials (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BYTEA NOT NULL UNIQUE,
    passkey         JSONB NOT NULL,
    label           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);
CREATE INDEX passkey_credentials_user ON passkey_credentials (user_id);

CREATE TABLE passkey_ceremonies (
    id              UUID PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('register','authenticate')),
    user_id         UUID NOT NULL,
    username        TEXT,
    display_name    TEXT,
    state           JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    used_at         TIMESTAMPTZ
);
CREATE INDEX passkey_ceremonies_expiry ON passkey_ceremonies (expires_at)
    WHERE used_at IS NULL;

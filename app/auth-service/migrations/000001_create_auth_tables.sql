CREATE TABLE accounts (
    id                  UUID PRIMARY KEY,
    username            VARCHAR(32) NOT NULL,
    username_normalized VARCHAR(32) NOT NULL UNIQUE,
    status              VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE password_credentials (
    account_id          UUID PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    password_hash       TEXT NOT NULL,
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE refresh_sessions (
    id          UUID PRIMARY KEY,
    family_id   UUID NOT NULL,
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash  VARCHAR(64) NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    replaced_by UUID REFERENCES refresh_sessions(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_sessions_account ON refresh_sessions (account_id, created_at DESC);
CREATE INDEX idx_refresh_sessions_family ON refresh_sessions (family_id);

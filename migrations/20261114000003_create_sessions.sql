-- BFF-owned session and preferences tables.
-- Sessions: server-side, tokens never reach browser.

CREATE TABLE IF NOT EXISTS bff_sessions (
    id              TEXT        PRIMARY KEY,            -- opaque session ID
    subject         TEXT        NOT NULL,               -- IdP sub claim
    display_name    TEXT,
    -- Tokens encrypted at rest (AES-256-GCM); key from env SESSIONS_ENCRYPTION_KEY
    -- For Phase 1: store as text, encryption wrapper added in next iteration
    access_token    TEXT        NOT NULL,
    refresh_token   TEXT,
    access_expires_at TIMESTAMPTZ NOT NULL,
    csrf_token      TEXT        NOT NULL,
    idle_timeout_secs BIGINT   NOT NULL DEFAULT 3600,
    absolute_timeout_secs BIGINT NOT NULL DEFAULT 43200,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    workspace_id    TEXT,
    agent_id        UUID
);

CREATE INDEX IF NOT EXISTS idx_bff_sessions_subject ON bff_sessions (subject);
CREATE INDEX IF NOT EXISTS idx_bff_sessions_expired ON bff_sessions (last_seen_at, created_at);

CREATE TABLE IF NOT EXISTS bff_login_transactions (
    state           TEXT        PRIMARY KEY,
    nonce           TEXT        NOT NULL,
    pkce_verifier   TEXT        NOT NULL,
    return_to       TEXT        NOT NULL,
    browser_binding TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bff_login_txn_expires ON bff_login_transactions (expires_at);

CREATE TABLE IF NOT EXISTS bff_user_preferences (
    subject         TEXT        PRIMARY KEY,
    locale          TEXT        NOT NULL DEFAULT 'id-ID',
    timezone        TEXT        NOT NULL DEFAULT 'Asia/Jakarta',
    theme           TEXT        NOT NULL DEFAULT 'system',
    notification_prefs JSONB   NOT NULL DEFAULT '{}',
    version         BIGINT      NOT NULL DEFAULT 1,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

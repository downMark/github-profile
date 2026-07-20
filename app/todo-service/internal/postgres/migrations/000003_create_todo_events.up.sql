CREATE TABLE todo_event_outbox (
    event_id       UUID PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    event_type     TEXT NOT NULL,
    occurred_at    TIMESTAMPTZ NOT NULL,
    environment    TEXT NOT NULL,
    github_user_id UUID NOT NULL,
    todo_id        UUID NOT NULL,
    payload        JSONB NOT NULL,
    available_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_until   TIMESTAMPTZ,
    locked_by      TEXT,
    attempts       INTEGER NOT NULL DEFAULT 0,
    last_error     TEXT,
    published_at   TIMESTAMPTZ
);

CREATE INDEX idx_todo_event_outbox_pending
    ON todo_event_outbox (available_at, occurred_at)
    WHERE published_at IS NULL;

CREATE TABLE todo_event_audit (
    event_id       UUID PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    event_type     TEXT NOT NULL,
    occurred_at    TIMESTAMPTZ NOT NULL,
    processed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    environment    TEXT NOT NULL,
    github_user_id UUID NOT NULL,
    todo_id        UUID NOT NULL,
    payload        JSONB NOT NULL
);

CREATE INDEX idx_todo_event_audit_user_time
    ON todo_event_audit (github_user_id, occurred_at DESC, event_id DESC);

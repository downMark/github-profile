ALTER TABLE todos RENAME COLUMN user_id TO github_user_id;
DROP INDEX IF EXISTS idx_todos_user_updated;
CREATE INDEX todos_github_user_updated_idx ON todos (github_user_id, updated_at DESC, id DESC);

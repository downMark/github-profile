ALTER TABLE github_users DROP CONSTRAINT github_users_github_id_key;
ALTER TABLE github_users ADD COLUMN owner_account_id UUID NOT NULL;
ALTER TABLE github_users ADD CONSTRAINT github_users_owner_github_unique UNIQUE (owner_account_id, github_id);
CREATE INDEX github_users_owner_updated_idx ON github_users (owner_account_id, updated_at DESC, id DESC);

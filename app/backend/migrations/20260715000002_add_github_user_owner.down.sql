DROP INDEX IF EXISTS github_users_owner_updated_idx;
ALTER TABLE github_users DROP CONSTRAINT github_users_owner_github_unique;
ALTER TABLE github_users DROP COLUMN owner_account_id;
ALTER TABLE github_users ADD CONSTRAINT github_users_github_id_key UNIQUE (github_id);

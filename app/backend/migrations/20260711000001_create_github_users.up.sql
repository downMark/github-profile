-- github_users: 已导入的 GitHub 用户资料 + 加密后的访问 token
CREATE TABLE github_users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_id       BIGINT UNIQUE NOT NULL,
    login           VARCHAR(255) NOT NULL,
    name            VARCHAR(255),
    bio             TEXT,
    avatar_url      TEXT,
    html_url        TEXT,
    public_repos    INT NOT NULL DEFAULT 0,
    followers       INT NOT NULL DEFAULT 0,
    following       INT NOT NULL DEFAULT 0,
    company         VARCHAR(255),
    blog            TEXT,
    location        VARCHAR(255),
    -- AES-256-GCM 加密后的 GitHub token（base64(nonce || ciphertext)），任何 API 响应不得返回该字段
    encrypted_token TEXT NOT NULL,
    -- GitHub 账号注册时间，与本系统入库时间分开保存
    github_created_at TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 列表接口按 updated_at DESC 分页（GET /api/users）
CREATE INDEX idx_github_users_updated_at ON github_users (updated_at DESC);

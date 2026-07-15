# Backend

Rust/Axum API。当前云端部署目标是 ECS Fargate，同时保留本地 HTTP 和 Lambda Runtime 兼容入口。

## 本地运行

按 `.env.example` 设置环境变量并准备 PostgreSQL，然后运行：

```bash
cargo run
```

服务默认监听 `http://localhost:3000`，启动时自动执行 migration。

## 验证

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
```

## 容器构建

在仓库根目录运行：

```bash
docker build --tag github-profile-backend:local app/backend
```

PR 云端环境由根目录 `infra/pr-environment.yaml` 和 `.github/workflows/pr-environment.yml` 管理。数据库凭据与 Token 加密密钥由 ECS 从 Secrets Manager 注入，不写入镜像。

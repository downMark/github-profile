# Backend

Rust/Axum Profile API。当前云端部署目标是 ECS Fargate，同时保留本地 HTTP 和 Lambda Runtime 兼容入口；ECS/本地模式额外提供内部 gRPC 用户查询服务。

## 本地运行

按 `.env.example` 设置环境变量并准备 PostgreSQL，然后运行：

```bash
cargo run
```

HTTP 默认监听 `http://localhost:3000`，gRPC 默认监听 `localhost:50051`，启动时自动执行 migration。Lambda 模式只运行 HTTP Router。

## 验证

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
```

## 容器构建

Docker 构建需要共享 proto，因此在仓库根目录使用 `app` 作为构建上下文：

```bash
docker build --file app/backend/Dockerfile --tag github-profile-profile:local app
```

PR 云端环境由根目录 `infra/pr-environment.yaml` 和 `.github/workflows/pr-environment.yml` 管理。数据库凭据与 Token 加密密钥由 ECS 从 Secrets Manager 注入，不写入镜像。

# GitHub Profile Manager

GitHub Profile Manager 使用 React 前端、Rust Auth/Profile Service 和 Go Todo Service 管理系统账号、GitHub 用户及其 Todo。

## 目录

```text
app/frontend/                 React + Vite 前端
app/auth-service/             Rust + Axum Auth Service（账号密码、JWT、Refresh Session）
app/backend/                  Rust + Axum Profile Service（HTTP + gRPC）
app/todo-service/             Go Todo Service
app/contracts/                跨服务 protobuf 合约
app/compose.yaml              PostgreSQL + 三服务本地环境
infra/pr-environment.yaml     PR、Production 三服务 CloudFormation 模板
buildspec.yml/.codebuild/     CodeBuild Batch 三镜像并行构建
.github/workflows/            GitHub Actions CI/CD
```

## 本地运行

前端：

```bash
cd app/frontend
npm ci
npm run dev
```

复制 `app/.env.example` 为 `app/.env` 并设置本地加密密钥，然后可一次启动 PostgreSQL 和三个服务：

```bash
cd app
docker compose up --build
```

Auth HTTP 默认监听 `3002`，Profile HTTP 监听 `3000`，Profile gRPC 监听 `50051`，Todo HTTP 监听 `3001`。Vite 将认证路径代理到 Auth、Todo 路径代理到 Go，其余 `/api` 代理到 Profile。

首次进入 `http://localhost:5173` 会跳转到注册页。注册并登录后才能导入 GitHub Token；导入的 GitHub 账号只属于当前系统账号，每个 GitHub 账号分别管理自己的 Todo。

## PR 环境

同仓库且源分支以 `jira-` 开头的 Pull Request 会触发独立环境：

1. GitHub Actions 通过 OIDC 获取 AWS 临时凭证。
2. CodeBuild Batch 并行测试并构建 Auth、Profile 和 Todo 镜像，推送到各自 ECR Repository。
3. CloudFormation Stack 创建三个 ECS Service、Task Definition、Target Group 和 ALB Rule，使用 Service Connect 建立 Todo → Profile gRPC 及 Auth JWKS 内部调用。
4. 前端构建后发布到 Cloudflare Pages 的 `pr-<编号>` 预览分支。
5. PR 关闭时先删除 PostgreSQL Schema，再删除 Stack 和 PR 镜像。

完整配置见 [DEPLOYMENT_ARCHITECTURE.md](DEPLOYMENT_ARCHITECTURE.md)。

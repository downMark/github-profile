# GitHub Profile Manager

GitHub Profile Manager 使用 React 前端和 Rust/Axum 后端管理 GitHub 用户资料。

## 目录

```text
app/frontend/                 React + Vite 前端
app/backend/                  Rust + Axum 后端
infra/pr-environment.yaml     每个 PR 的 CloudFormation 模板
.github/workflows/            GitHub Actions CI/CD
```

## 本地运行

前端：

```bash
cd app/frontend
npm ci
npm run dev
```

后端所需环境变量见 `app/backend/README.md`。本地默认监听 `3000`，Vite 将 `/api` 代理到后端。

## PR 环境

同仓库且源分支以 `jira-` 开头的 Pull Request 会触发独立环境：

1. GitHub Actions 通过 OIDC 获取 AWS 临时凭证。
2. 后端镜像构建并推送到共享 ECR。
3. CloudFormation Stack 创建独立 ECS Service、Task Definition、Target Group、ALB Rule 和 Cloud Map Service。
4. 前端构建后发布到 Cloudflare Pages 的 `pr-<编号>` 预览分支。
5. PR 关闭时先删除 PostgreSQL Schema，再删除 Stack 和 PR 镜像。

完整配置见 [DEPLOYMENT_ARCHITECTURE.md](DEPLOYMENT_ARCHITECTURE.md)。

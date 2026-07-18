# GitHub Profile Manager — Frontend

React SPA（Vite + React + TypeScript），部署至 Cloudflare Pages。

## 开发

```bash
npm install
npm run dev        # 本地开发服务器
npm run lint       # oxlint
npm run build      # tsc -b && vite build，产物在 dist/
npm run preview    # 预览构建产物
```

## 环境变量

| 变量                | 说明                                      |
| ------------------- | ----------------------------------------- |
| `VITE_API_BASE_URL` | 后端 API 基础地址，默认 `/api` |

本地可创建 `.env.local` 覆盖（参考 `.env.example`）。

## Cloudflare Pages 部署

- 配置文件：`wrangler.toml`（`pages_build_output_dir = "dist"`）
- SPA 路由回退：`public/_redirects`（`/* /index.html 200`）
- 手动部署：

```bash
npm run build
npx wrangler pages deploy dist --project-name <PROJECT_NAME>
```

- PR 环境由 GitHub Actions 使用规范化后的 `jira-*` PR 源分支发布 Direct Upload Preview，例如 `jira-123` 对应 `jira-123.<PROJECT_NAME>.pages.dev`。
- Cloudflare API Token 保存在 GitHub Environment Secret `CLOUDFLARE_API_TOKEN` 中，不提交到仓库。

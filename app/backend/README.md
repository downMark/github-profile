# Backend

Rust/Axum API，支持本地 HTTP 服务和 AWS Lambda 两种运行模式。

## 本地运行

按 `.env.example` 设置环境变量并准备 PostgreSQL，然后运行 `cargo run`。服务默认监听 `http://localhost:3000`，启动时自动执行 migration。

## 验证与打包

```sh
cargo fmt --check
cargo test
cargo lambda build --release --arm64
sam validate --lint --template-file template.yaml
```

真实部署前必须提供 AWS 凭据、RDS 网络连通性和模板参数。本模板只提供离线配置，不会自动发布云资源。

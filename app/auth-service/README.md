# Auth Service

Rust Auth Service 独占 `accounts`、`password_credentials` 和 `refresh_sessions` 表，提供用户名密码注册/登录、15 分钟 RS256 Access JWT、30 天 Refresh Token 单次轮换和退出。

本地端口为 `3002`（容器内 `3000`）。Refresh Token 只存于 `HttpOnly; SameSite=Lax; Path=/api/auth` Cookie，数据库只保存 SHA-256 哈希；Access Token 由前端保存在内存。

主要接口：

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `GET /.well-known/jwks.json`

本地 Compose 会设置 `DATABASE_URL`、`JWT_ISSUER`、`JWT_AUDIENCE`、Token TTL 和 Cookie Secure 配置，不需要把 JWT 私钥写入仓库；服务每次本地启动时生成临时 RSA 密钥。

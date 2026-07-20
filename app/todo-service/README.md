# Todo Service

Todo 写操作会在同一 PostgreSQL 事务中写入 `todo_event_outbox`。配置 `TODO_EVENTS_TOPIC_ARN` 和 `TODO_EVENTS_QUEUE_URL` 后，后台发布器将版本化事件发送到 SNS，SQS 消费者幂等写入 `todo_event_audit`；两项配置都为空时本地消息功能关闭。

审计查询接口：

```text
GET /api/users/{user_id}/todos/events?page=1&limit=20
```

事件当前使用 `schema_version: 1`，支持 `todo.created`、`todo.updated` 和 `todo.deleted`。

Go 服务负责 GitHub 账号 Todo 的增删改查，只访问 `todos` 表。HTTP 请求先验证 Auth Service 签发的 Access JWT，再把 Bearer Token 通过 gRPC metadata 交给 Profile Service 校验当前系统账号是否拥有目标 GitHub 账号。

## 环境变量

```text
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres?sslmode=disable
DATABASE_MAX_CONNECTIONS=5
PROFILE_GRPC_ADDR=localhost:50051
AUTH_ISSUER=http://localhost:3002
AUTH_AUDIENCE=github-profile
AUTH_JWKS_URL=http://localhost:3002/.well-known/jwks.json
ALLOWED_ORIGIN=http://localhost:5173
API_BASE_PATH=
PORT=3001
```

也可使用与 Rust 服务相同的 `DB_HOST`、`DB_PORT`、`DB_NAME`、`DB_USERNAME`、`DB_PASSWORD` 和 `DB_SSL_MODE` 组件配置。PR 环境额外设置 `DB_SCHEMA=pr_<number>`。

## 本地运行

```bash
go run ./cmd/server
```

## 验证

```bash
./scripts/generate-proto.sh
go test ./...
go vet ./...
```

PostgreSQL 集成测试默认跳过；提供一个可清理的测试数据库即可启用：

```bash
TEST_DATABASE_URL='postgres://postgres:postgres@localhost:5432/postgres?sslmode=disable' go test ./internal/postgres
```

测试会创建并在结束时删除自己的 `pr_<number>` Schema。

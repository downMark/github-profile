# 基础镜像走 ECR Public 官方镜像源，规避 Docker Hub 匿名限流（429）
FROM public.ecr.aws/docker/library/golang:1.24-bookworm AS builder

WORKDIR /workspace/todo-service

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY todo-service/go.mod todo-service/go.sum ./
RUN go mod download

COPY contracts /workspace/contracts
COPY todo-service .
RUN ./scripts/generate-proto.sh \
    && go test ./... \
    && go vet ./... \
    && CGO_ENABLED=0 GOOS=linux go build -trimpath -ldflags="-s -w" -o /todo-service ./cmd/server

FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /todo-service /todo-service

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/todo-service"]

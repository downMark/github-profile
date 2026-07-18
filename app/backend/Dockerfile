# 基础镜像走 ECR Public 官方镜像源，规避 Docker Hub 匿名限流（429）
FROM public.ecr.aws/docker/library/rust:1.94-bookworm AS builder

WORKDIR /workspace/backend
RUN rustup component add rustfmt

# 依赖预编译层：Cargo.lock/contracts 不变即可命中缓存，避免每次全量重编译
COPY contracts /workspace/contracts
COPY backend/Cargo.toml backend/Cargo.lock backend/build.rs ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && cargo test --release --locked --no-run \
    && rm -rf src \
        target/release/github-profile-api* \
        target/release/deps/github_profile_api-* \
        target/release/.fingerprint/github-profile-api-*

COPY backend/migrations ./migrations
COPY backend/src ./src
RUN cargo fmt --check \
    && cargo test --release --locked \
    && cargo build --release --locked

FROM public.ecr.aws/docker/library/debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /workspace/backend/target/release/github-profile-api /usr/local/bin/github-profile-api

ENV PORT=3000
ENV GRPC_PORT=50051
EXPOSE 3000 50051

CMD ["github-profile-api"]

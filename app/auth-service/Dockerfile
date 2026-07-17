FROM rust:1.94-bookworm AS builder
WORKDIR /build
RUN rustup component add rustfmt

# 依赖预编译层：只要 Cargo.lock 不变即可命中缓存，避免每次全量重编译
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && cargo test --release --locked --no-run \
    && rm -rf src \
        target/release/github-profile-auth* \
        target/release/deps/github_profile_auth-* \
        target/release/.fingerprint/github-profile-auth-*

COPY src ./src
COPY migrations ./migrations
RUN cargo fmt --check \
    && cargo test --release --locked \
    && cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/github-profile-auth /usr/local/bin/auth-service
ENV PORT=3000
EXPOSE 3000
CMD ["auth-service"]

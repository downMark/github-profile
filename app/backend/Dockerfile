FROM rust:1.94-bookworm AS builder

WORKDIR /workspace/backend

COPY backend/Cargo.toml backend/Cargo.lock backend/build.rs ./
COPY backend/migrations ./migrations
COPY backend/src ./src
COPY contracts /workspace/contracts

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /workspace/backend/target/release/github-profile-api /usr/local/bin/github-profile-api

ENV PORT=3000
ENV GRPC_PORT=50051
EXPOSE 3000 50051

CMD ["github-profile-api"]

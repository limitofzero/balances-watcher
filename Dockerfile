FROM rust:1.90-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo build --release && rm -rf src

COPY src ./src

RUN cargo build --release \
  && strip target/release/token-balances-updater || true

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -s /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /app/target/release/token-balances-updater /app/token-balances-updater
COPY --from=builder /app/configs ./configs

RUN chown -R appuser:appuser /app

USER appuser

EXPOSE 8080

CMD ["./token-balances-updater"]

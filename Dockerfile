FROM rust:1.90-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY configs ./configs

RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN cargo build --release && rm -rf src

COPY src ./src

RUN touch src/main.rs && cargo build --release --offline

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 appuser

WORKDIR /app

COPY --from=builder /app/target/release/token-balances-updater /app/token-balances-updater
COPY --from=builder /app/configs ./configs

RUN chown appuser:appuser /app/token-balances-updater

USER appuser

EXPOSE 8080

CMD ["./token-balances-updater"]

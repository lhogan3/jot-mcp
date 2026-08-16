FROM rust:1-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:trixie-slim
COPY --from=builder /app/target/release/jot-mcp /usr/local/bin/jot-mcp
ENTRYPOINT ["/usr/local/bin/jot-mcp"]

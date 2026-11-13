# --- Stage 1: Build WASM client + Release server ---
FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends python3 python-is-python3 && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

WORKDIR /app
COPY . .

# Build Leptos WASM bundle into crates/monopoly-client/dist
RUN cd crates/monopoly-client && trunk build --release

# Build authoritative Axum server
RUN cargo build --release -p monopoly-server

# --- Stage 2: Minimal runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=builder /app/target/release/monopoly-server /app/monopoly-server
COPY --from=builder /app/crates/monopoly-client/dist /app/crates/monopoly-client/dist

ENV PORT=8080
EXPOSE 8080
CMD ["./monopoly-server"]

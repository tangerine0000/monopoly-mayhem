# Vietnamese Monopoly ("Cờ Tỷ Phú Sài Gòn")

Full-stack Rust web multiplayer implementation of the Saigon edition of Monopoly ("Cờ Tỷ Phú Sài Gòn").

## Architecture

- **`monopoly-proto`**: Shared wire protocol serialization types, client messages, and server broadcast events.
- **`monopoly-core`**: Deterministic domain logic, Saigon 40-tile catalog, cards, and state transitions. Zero external I/O or DOM dependencies, compiles to native and WASM.
- **`monopoly-server`**: Authoritative game server using Axum, Tokio WebSockets, per-room actor tasks, turn countdown timers, and "Xe Ôm" heuristic fallback bot.
- **`monopoly-client`**: Single Page Application in Leptos (CSR), rendering responsive inline SVG with coordinate mapping, minimap, ledger, and reconnect logic.

## Prerequisites

- Rust 1.80+
- `rustup target add wasm32-unknown-unknown`
- Trunk (`cargo install trunk`)
- Python 3 (for asset sanitizer)

## Build & Test Commands

```bash
# Check shared protocol types
cargo check -p monopoly-proto

# Run core domain test suite
cargo test -p monopoly-core

# Run server test suite
cargo test -p monopoly-server

# Build client WASM application
cd crates/monopoly-client && trunk build --release

# Run entire workspace tests
cargo test --workspace
```

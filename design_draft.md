# Vietnamese Monopoly ("Cờ Tỷ Phú Sài Gòn") Full-Stack Rust Web Application

*   **Author:** Engineering Team
*   **Status:** Draft
*   **Date:** 2026-10-30

--------------------------------------------------------------------------------

## Overview

This project implements a web-based, multiplayer version of the Saigon edition of Monopoly ("Cờ Tỷ Phú Sài Gòn") using an end-to-end Rust architecture. The system combines an authoritative server built with Axum and Tokio WebSockets with a WebAssembly client built with Leptos and vector SVG graphics. The design targets low-bandwidth mobile and desktop web browsers, delivering responsive gameplay, authentic Saigon cultural landmarks, a strict 30-second turn enforcement timer, and an automated "Xe Ôm" heuristic fallback bot to preserve game continuity when players disconnect.

## The Problem

Online multiplayer board games frequently suffer from three primary failure modes:
1. **High Client Resource Usage:** Modern web games often rely on heavy 3D WebGL runtimes (15–30 MB bundles) that drain mobile battery life, generate thermal throttling, and suffer from poor text rendering on low-to-mid-range mobile devices.
2. **Session Fragility and Stallouts:** When a participant disconnects or abandons a match, turn-based games freeze indefinitely or force an abrupt forfeit, degrading the match for remaining players.
3. **Loss of Cultural Flavor:** Existing digital Monopoly adaptations are generic Western localizations that lack Vietnamese street identities, regional transit hubs, and cultural humor.

This design resolves these issues through a lightweight Rust WASM and SVG frontend (targeting a sub-500 KB initial payload), a server-authoritative state machine with automated heuristic bot takeover on timeout/disconnect, and an authentic Saigon board layout.

## Goals

*   Provide a deterministic shared core (`monopoly-core`) running identical state-transition logic natively on the server and inside WebAssembly on the client.
*   Deliver a full 40-tile Saigon street layout featuring 8 distinct district color bands, 4 municipal transit hubs, 2 public utilities, and localized "Cơ Hội" & "Khí Vận" chance cards.
*   Enforce a strict 30-second server-side turn timer with automated "Xe Ôm" heuristic fallback moves to prevent stalled lobbies.
*   Implement reconnection session tokens allowing players on mobile networks to reclaim active seats within 15 seconds of network interruption.
*   Maintain a sub-500 KB compressed WebAssembly and SVG client bundle capable of running smoothly on mobile 4G connections without WebGL overhead.
*   Ensure full numerical precision in financial transactions using 64-bit signed integers (`i64` in thousands of VNĐ) to eliminate floating-point rounding errors.

## Non-Goals

*   3D WebGL graphics, physical dice simulation engines, or heavy mesh rendering in the v1 MVP.
*   Native mobile binary wrappers for iOS App Store or Google Play Store distributions in the initial phase.
*   Real-money gambling, payment gateways, fiat currency cash-outs, or microtransactions.
*   Asynchronous multi-day correspondence play (matches are real-time, live sessions).

## Open Questions

*   **Question 1**: How should the server determine room cleanup when all connected human players disconnect simultaneously during an active game?
*   **Question 2**: For the mobile portrait layout, should tile zoom follow active player movement automatically, or allow manual pan-and-zoom touch controls alongside the mini-map?

## Critical User Journeys (CUJs)

### CUJ 1: Lobby Creation, Invite Link, and Game Start
*   **Given**: A player visits the application homepage on a desktop or mobile browser.
*   **When**: The player clicks "Create Game" and shares the generated room URL (e.g., `?room=SGN-888`) with 3 other players who join the lobby.
*   **Then**: The authoritative server assigns player seats, synchronizes the initial board state, rolls for starting order, and transitions all clients to the active match screen.

### CUJ 2: Normal Turn Flow and Property Acquisition
*   **Given**: An active game session where it is Player 1's turn with an active 30-second timer.
*   **When**: Player 1 clicks "Lắc Xúc Xắc" (Roll Dice), moves to "Nguyễn Huệ" (unowned), and clicks "Mua Bất Động Sản" (Buy Property).
*   **Then**: The client optimistically renders the action, the server validates funds and tile ownership, deducts cash, updates property title deeds, and broadcasts the state delta to all connected players.

### CUJ 3: Disconnect and "Xe Ôm" Bot Takeover
*   **Given**: Player 2 experiences a mobile connection drop while holding an active turn.
*   **When**: The 30-second turn timer expires or the socket remains disconnected past the 15-second grace window.
*   **Then**: The server flags Player 2 as temporarily absent, invokes the heuristic "Xe Ôm" bot to execute the required dice roll or turn pass, and preserves Player 2's session token so they can reclaim their seat seamlessly upon reconnecting.


--------------------------------------------------------------------------------

## Technical Plan (Architecture)

### 1. High-Level Workspace Architecture

The system is organized as a Cargo workspace with four distinct crates that enforce boundary separation between domain rules, wire communication, server infrastructure, and web rendering:

```text
+-------------------------------------------------------------+
|                      monopoly-proto                         |
|     (Serde Action Commands, Event Dockets, Error Enums)     |
+------------------------------+------------------------------+
                               |
            +------------------+------------------+
            |                                     |
            v                                     v
+-----------------------+             +-----------------------+
|     monopoly-core     |             |    monopoly-client    |
| (Deterministic State  |             | (Leptos SPA, WASM,    |
|  Machine, Rules, PRNG)|             |  SVG Viewport, Audio) |
+-----------+-----------+             +-----------+-----------+
            ^                                     ^
            |                                     |
            +------------------+------------------+
                               | (via WebSocket)
            +------------------+------------------+
            |          monopoly-server            |
            | (Axum HTTP/WS, Room Actors, Tokio   |
            |  Timers, "Xe Ôm" Bot Engine)        |
            +-------------------------------------+
```

*   **`monopoly-core`**: A headless domain library containing pure data models and state transitions. It has zero network, filesystem, or DOM dependencies, allowing direct compilation to native machine targets (for server execution) and WebAssembly (for client-side optimistic UI validation).
*   **`monopoly-proto`**: Shared message contract crate defining typed payloads for client actions and server broadcast events using Serde.
*   **`monopoly-server`**: An asynchronous network server built on Axum and Tokio. It routes HTTP requests for lobby setup, establishes WebSocket streams, manages isolated room actors, coordinates the 30-second turn countdown, and executes heuristic bot decisions.
*   **`monopoly-client`**: A single-page WebAssembly application authored in Leptos. It maps room state to a reactive DOM and SVG canvas, renders animations via CSS hardware transforms, and maintains local WebSocket session state.

### 2. State Machine & Domain Logic (`monopoly-core`)

The game state machine operates as a pure reducer: given the current `GameState`, an incoming `PlayerAction`, and a deterministic pseudorandom generator (`rand_chacha::ChaCha8Rng`), it yields either an updated `GameState` and a slice of `GameEvent` occurrences, or a `GameRuleError`.

#### Core Domain Types (Illustrative API)
*   **`Money`**: Represented as `i64` in units of thousands of Vietnamese Đồng (1 unit = 1,000 VNĐ). Starting cash per player is 15,000 units (15,000,000 VNĐ).
*   **`Board`**: An array of 40 static `Tile` records (streets, stations, utilities, taxes, chance/chest, corner stations).
*   **`PlayerState`**: ID, seat index (0..3), cash balance, board position (0..39), in-jail status, turns in jail, and owned property bitmasks.
*   **`TurnPhase`**: Enum tracking turn progression:
    *   `WaitingForRoll`: Player must roll dice or pay bail.
    *   `TileActionPending`: Pawn landed; player must buy property, draw card, pay rent, or go to jail.
    *   `Auction`: Tile declined; open bidding initiated among players.
    *   `TradingOrBuilding`: Optional inter-turn phase for house construction or property trades.
    *   `TurnEnded`: Cleanup completed; ready for next player.

```text
[Client Action] ---> [monopoly-server Validation]
                             |
                             v
              [monopoly-core::apply_action()]
                             |
         +-------------------+-------------------+
         |                                       |
    (Valid Transition)                    (Invalid Action)
         |                                       |
         v                                       v
[New GameState + GameEvents]            [GameRuleError -> Client]
         |
         v
[Broadcast to Room Sockets]
```

### 3. Network Protocol & Synchronization (`monopoly-proto`)

Communication across WebSocket channels uses serialized JSON frames conforming to two tagged envelope families:

*   **`ClientMessage`**:
    *   `JoinRoom { room_id: String, player_name: String, session_token: Option<String> }`
    *   `RollDice`
    *   `BuyProperty`
    *   `DeclineProperty`
    *   `BuildHouse { tile_id: u8 }`
    *   `MortgageProperty { tile_id: u8 }`
    *   `PayJailBail`
    *   `EndTurn`
*   **`ServerMessage`**:
    *   `RoomJoined { player_id: Uuid, seat: u8, session_token: String, initial_state: GameState }`
    *   `GameStarted { turn_order: Vec<Uuid> }`
    *   `TurnStarted { current_player: Uuid, time_remaining_secs: u8 }`
    *   `TimerTick { time_remaining_secs: u8 }`
    *   `DiceRolled { player_id: Uuid, die1: u8, die2: u8, is_doubles: bool, destination: u8 }`
    *   `PropertyAcquired { player_id: Uuid, tile_id: u8, price: Money }`
    *   `RentPaid { payer: Uuid, recipient: Uuid, amount: Money }`
    *   `CardDrawn { player_id: Uuid, deck: DeckType, card_id: u16, title: String, effect_desc: String }`
    *   `PlayerDisconnected { player_id: Uuid, bot_takeover_pending_secs: u8 }`
    *   `BotAssumedControl { player_id: Uuid }`
    *   `SyncSnapshot { state: GameState }`
    *   `ActionRejected { reason: String }`

### 4. Room Actor Lifecycle & Turn Timer (`monopoly-server`)

Each game room runs as an isolated Tokio task actor:
1.  **Actor State**: Holds the current `GameState`, connected client channel senders, session token map, and an active cancellation token for the turn countdown.
2.  **Turn Timer Loop**: Upon `TurnStarted`, a Tokio task emits a `TimerTick` every second.
3.  **Timeout / Bot Fallback**: If the timer reaches 0 before the active player submits a legal terminal action, or if a disconnected player's turn begins, the room actor delegates control to the "Xe Ôm" heuristic routine:
    *   If in `WaitingForRoll`: Invokes auto-roll.
    *   If in `TileActionPending` on an unowned street:
        *   If player's cash balance exceeds twice the property cost: Buys property.
        *   Otherwise: Declines property, triggering normal turn resolution.
    *   If in debt: Sequentially mortgages lowest-tier un-monopolized properties to satisfy liability. If liabilities exceed liquidation capacity, declares bankruptcy.
    *   Calls `EndTurn` and hands control to the next player.

### 5. Responsive SVG Viewport & Rendering (`monopoly-client`)

The Leptos frontend translates `GameState` into an interactive SVG structure with responsive adaptations:
*   **Desktop & Tablet Landscape**: Renders a central 1000x1000 coordinate SVG board framed by player ledger panels, dice controls, and notification feeds.
*   **Mobile Portrait**:
    *   *Minimap View*: A 160x160 overview anchored in the top corner highlighting active player pins.
    *   *Dynamic Quadrant View*: The main SVG viewport applies an animated `viewBox` transformation focusing on the active 10-tile board segment where the current turn's pawn interaction occurs.
*   **Animation Performance**: Pawns and building indicators use SVG group IDs (`#pawn_0` through `#pawn_3`) styled with CSS hardware-accelerated transforms (`transform: translate3d(...)`) rather than recalculating raw SVG node paths.

### 6. Non-Functional Requirements

*   **Security & Anti-Cheat**: All RNG seeds, dice roll outcomes, money arithmetic, and ownership records remain strictly server-authoritative. The client receives outcomes and cannot dictate dice values or balance values.
*   **Latency & Bandwidth**: Event delta frames are compact JSON payloads (<1 KB per turn step). Full state snapshots are sent only on initial join and reconnection resynchronization.
*   **State Reliability**: Players retain their assigned seat and cash balance across mobile network shifts via their unique `session_token` stored in browser `localStorage`.


--------------------------------------------------------------------------------

## Alternatives Considered

### Alternative 1: Bevy Engine (Canvas / WebGL 3D Rendering)

*   **Pros**: Enables 3D mesh rendering, dynamic camera perspectives, physical rigid-body dice collisions, and visual particle effects.
*   **Cons**: Produces a heavy WebAssembly payload (15–30 MB download), requires continuous WebGL context rendering leading to rapid mobile battery drain and thermal throttling, and exhibits blurry typography on high-DPI mobile screens without complex SDF font atlases.
*   **Reason for Rejection**: Accessible web play on mobile devices requires fast initial loading and low power consumption. A Leptos and SVG approach delivers a sub-500 KB bundle that loads immediately over cellular networks while maintaining sharp vector typography across all screen resolutions.

### Alternative 2: Full-State Snapshot Broadcast on Every Action

*   **Pros**: Simplifies client implementation by eliminating local state mutation logic; the client simply overwrites its active state tree on every incoming server packet.
*   **Cons**: Transmits redundant board data on high-frequency updates (such as second-by-second timer countdowns and movement steps), increases JSON serialization overhead, and complicates smooth client-side step animations.
*   **Reason for Rejection**: Replaying discrete event records (`DiceRolled`, `PropertyAcquired`, `RentPaid`) keeps WebSocket messages under 1 KB, conserves bandwidth on cellular networks, and allows the client to animate intermediate movements sequentially. Full snapshots are reserved for room joins and reconnection synchronization.

### Alternative 3: Immediate Match Forfeit on Disconnection

*   **Pros**: Simplifies server architecture by eliminating bot heuristics, timeout timers, and session recovery mappings.
*   **Cons**: Destroys game flow for the remaining three participants whenever a player experiences a transient network drop, background app suspension, or mobile signal interruption.
*   **Reason for Rejection**: Multiplayer board games require 30–60 minutes of continuous engagement. Substituting the automated "Xe Ôm" heuristic bot maintains game momentum and gives disconnected players a grace period to reclaim their seats without stalling the table.


--------------------------------------------------------------------------------

## Detailed Implementation Plan

This section provides the exhaustive implementation plan. Every file to be created or modified is enumerated along with its specific rationale and architectural responsibility.

### Target Files to Create/Modify

#### 1. Root Workspace Configuration & Documentation

*   **`Cargo.toml`** (Create)
    *   **Type / Scope**: Cargo Workspace Virtual Manifest
    *   **Proposed Changes**: Define the top-level workspace containing members `crates/monopoly-core`, `crates/monopoly-proto`, `crates/monopoly-server`, and `crates/monopoly-client`. Set shared workspace dependencies, compilation profiles, and release optimizations (LTO, opt-level 3, and strip symbols for WASM).
*   **`.gitignore`** (Create)
    *   **Type / Scope**: Build Tooling
    *   **Proposed Changes**: Ignore `/target`, `crates/monopoly-client/dist`, `.env`, and local editor files.
*   **`README.md`** (Create)
    *   **Type / Scope**: Project Documentation
    *   **Proposed Changes**: Provide architectural overview, local development prerequisites (Rust 1.80+, `wasm32-unknown-unknown` target, `trunk`), build and run commands for both native server and WebAssembly client, and test instructions.

#### 2. Protocol Crate (`crates/monopoly-proto`)

*   **`crates/monopoly-proto/Cargo.toml`** (Create)
    *   **Type / Scope**: Crate Manifest
    *   **Proposed Changes**: Declare dependencies for `serde` (with derive), `uuid` (with serde feature), and `thiserror`.
*   **`crates/monopoly-proto/src/lib.rs`** (Create)
    *   **Type / Scope**: Rust Protocol Library
    *   **Proposed Changes**: Define the wire protocol contracts: `ClientMessage` (commands such as `JoinRoom`, `RollDice`, `BuyProperty`, `DeclineProperty`, `BuildHouse`, `MortgageProperty`, `PayJailBail`, `EndTurn`) and `ServerMessage` (broadcasts such as `RoomJoined`, `GameStarted`, `TurnStarted`, `TimerTick`, `DiceRolled`, `PropertyAcquired`, `RentPaid`, `CardDrawn`, `BotAssumedControl`, `SyncSnapshot`, and `ActionRejected`).

#### 3. Core Domain State Machine (`crates/monopoly-core`)

*   **`crates/monopoly-core/Cargo.toml`** (Create)
    *   **Type / Scope**: Crate Manifest
    *   **Proposed Changes**: Declare dependencies for `rand_chacha`, `rand_core`, `serde`, `thiserror`, and internal dependency `monopoly-proto`.
*   **`crates/monopoly-core/src/lib.rs`** (Create)
    *   **Type / Scope**: Root Library Entry
    *   **Proposed Changes**: Re-export core modules (`money`, `board`, `cards`, `state`, `rules`) and expose the primary state reducer function.
*   **`crates/monopoly-core/src/money.rs`** (Create)
    *   **Type / Scope**: Domain Type
    *   **Proposed Changes**: Implement the `Money(pub i64)` value object where 1 unit represents 1,000 VNĐ. Implement arithmetic checks preventing negative balances without explicit bankruptcy flags and format functions returning display strings (e.g., `2.000.000 đ`).
*   **`crates/monopoly-core/src/board.rs`** (Create)
    *   **Type / Scope**: Board Configuration
    *   **Proposed Changes**: Define the 40-tile Saigon data array: street titles (Bến Vân Đồn to Nguyễn Huệ), district color bands, tile purchase prices, base rent tables, multi-house multipliers, transit stations (Ga Sài Gòn, Bến Xe Miền Đông, etc.), municipal utilities (Sawaco, EVN HCMC), taxes, and corner tiles.
*   **`crates/monopoly-core/src/cards.rs`** (Create)
    *   **Type / Scope**: Card Deck Logic
    *   **Proposed Changes**: Implement the deck state and card effects for "Cơ Hội" and "Khí Vận". Define effects including monetary penalties (flood repair), windfalls (lottery win), mandatory travel (move to specific station or street), and jail dispatch.
*   **`crates/monopoly-core/src/state.rs`** (Create)
    *   **Type / Scope**: State Structs
    *   **Proposed Changes**: Define `GameState`, `PlayerState`, `PropertyOwnership`, `TurnPhase`, `BankVault`, and `AuctionSession`. Ensure all structs derive `Clone`, `Debug`, `Serialize`, and `Deserialize`.
*   **`crates/monopoly-core/src/rules.rs`** (Create)
    *   **Type / Scope**: Reducer Logic
    *   **Proposed Changes**: Implement the pure transition function `apply_action(state: &GameState, action: &PlayerAction, rng: &mut impl Rng) -> Result<(GameState, Vec<GameEvent>), GameRuleError>`. Enforce movement rules, doubles bonus turns, jail lockup after three consecutive doubles, rent deduction, monopoly detection, and bankruptcy dissolution.
*   **`crates/monopoly-core/tests/rules_test.rs`** (Create)
    *   **Type / Scope**: Integration & Unit Test Suite
    *   **Proposed Changes**: Exhaustive automated tests covering: passing GO cash reward, buying unowned properties, stepping on owned tiles and paying rent, completing color sets and verifying doubled base rent, landing in jail, and handling bankruptcy transitions.

#### 4. Backend Server (`crates/monopoly-server`)

*   **`crates/monopoly-server/Cargo.toml`** (Create)
    *   **Type / Scope**: Crate Manifest
    *   **Proposed Changes**: Declare dependencies for `axum` (with ws feature), `tokio` (full runtime), `tower-http` (cors, trace, fs), `tracing`, `tracing-subscriber`, `serde_json`, `uuid`, and internal crates `monopoly-core` and `monopoly-proto`.
*   **`crates/monopoly-server/src/main.rs`** (Create)
    *   **Type / Scope**: Server Application Entrypoint
    *   **Proposed Changes**: Parse environment variables (port, bind address), configure structured logging with `tracing-subscriber`, initialize the shared room manager registry, bind TCP listener, and attach graceful shutdown signals.
*   **`crates/monopoly-server/src/router.rs`** (Create)
    *   **Type / Scope**: Axum HTTP & WS Router
    *   **Proposed Changes**: Define endpoints: `/healthz` for liveness probes, `/api/rooms` for creating new game rooms, and `/ws` for upgrading incoming client requests to WebSocket connections with room query parameters.
*   **`crates/monopoly-server/src/room.rs`** (Create)
    *   **Type / Scope**: Actor Room Coordination
    *   **Proposed Changes**: Manage per-room Tokio actors holding `GameState`, connected client channel senders, and session token persistence. Dispatch client messages into `monopoly-core`, broadcast event diffs, and handle player disconnects.
*   **`crates/monopoly-server/src/timer.rs`** (Create)
    *   **Type / Scope**: Turn Countdown Task
    *   **Proposed Changes**: Tokio-driven interval timer emitting `TimerTick` every 1,000 milliseconds for active turns. Cancels previous timer tasks on explicit player actions and triggers the bot engine upon reaching zero.
*   **`crates/monopoly-server/src/bot.rs`** (Create)
    *   **Type / Scope**: Heuristic Bot Engine
    *   **Proposed Changes**: Implement the "Xe Ôm" fallback logic: triggers automatic roll on roll timeout, decides property acquisition (buy if liquid cash > 2x property cost, else pass), executes mortgage liquidation on negative balance, and invokes `EndTurn`.
*   **`crates/monopoly-server/tests/ws_integration_test.rs`** (Create)
    *   **Type / Scope**: Integration Test Suite
    *   **Proposed Changes**: Connect mock WebSocket clients, create rooms, simulate full 4-player join, verify starting order broadcast, simulate 30s timeout and confirm bot takeover, and verify reconnection with session tokens.

#### 5. Frontend Client (`crates/monopoly-client`)

*   **`crates/monopoly-client/Cargo.toml`** (Create)
    *   **Type / Scope**: Crate Manifest
    *   **Proposed Changes**: Declare dependencies for `leptos` (CSR mode), `wasm-bindgen`, `web-sys`, `gloo-net`, `serde`, `serde_json`, `monopoly-core`, and `monopoly-proto`.
*   **`crates/monopoly-client/Trunk.toml`** (Create)
    *   **Type / Scope**: Trunk Bundler Config
    *   **Proposed Changes**: Configure public URL, output directory (`dist`), asset copying rules for SVG files and audio clips, and release optimization flags.
*   **`crates/monopoly-client/index.html`** (Create)
    *   **Type / Scope**: Web Entry HTML
    *   **Proposed Changes**: Set mobile-responsive viewport meta tags, link Google Fonts (Be Vietnam Pro for authentic typography), and define the mounting root container `<div id="app"></div>`.
*   **`crates/monopoly-client/src/main.rs`** (Create)
    *   **Type / Scope**: Client Application Entrypoint
    *   **Proposed Changes**: Initialize console error panic hook and mount the Leptos `App` component into the DOM.
*   **`crates/monopoly-client/src/app.rs`** (Create)
    *   **Type / Scope**: Root Leptos Component
    *   **Proposed Changes**: Manage client view states: Lobby view (create room / join room with room code and player name) and Active Match view.
*   **`crates/monopoly-client/src/ws.rs`** (Create)
    *   **Type / Scope**: WebSocket Client Manager
    *   **Proposed Changes**: Maintain WebSocket connection, dispatch incoming `ServerMessage` variants into Leptos reactive signals, manage heartbeat, and persist `session_token` in `window.localStorage`.
*   **`crates/monopoly-client/src/components/board.rs`** (Create)
    *   **Type / Scope**: SVG Board Component
    *   **Proposed Changes**: Render the 1000x1000 SVG board layout, map 40 tiles with authentic Saigon naming and color bands, translate pawn coordinates using hardware-accelerated CSS `transform: translate3d(...)`, and handle mobile dynamic quadrant view transitions.
*   **`crates/monopoly-client/src/components/minimap.rs`** (Create)
    *   **Type / Scope**: Mobile UI Component
    *   **Proposed Changes**: Render a persistent 160x160 overview board map on mobile portrait layouts, displaying current player positions and high-level district ownership.
*   **`crates/monopoly-client/src/components/controls.rs`** (Create)
    *   **Type / Scope**: Turn Controls Component
    *   **Proposed Changes**: Render contextual action buttons ("Lắc Xúc Xắc", "Mua Đất", "Xây Nhà", "Thế Chấp", "Kết Thúc Lượt") and a 30-second animated visual countdown timer bar.
*   **`crates/monopoly-client/src/components/ledger.rs`** (Create)
    *   **Type / Scope**: Player Ledger Component
    *   **Proposed Changes**: Display all 4 players' avatar, VNĐ cash balances, owned property cards, color set monopolies, and jail badges.
*   **`crates/monopoly-client/src/components/log.rs`** (Create)
    *   **Type / Scope**: Activity Feed Component
    *   **Proposed Changes**: Display localized game narrative feed (e.g., "Người chơi 1 đã đổ 5 và 3, di chuyển đến Nguyễn Huệ").

#### 6. Vector Assets & Build Scripts

*   **`assets/svg/board_saigon.svg`** (Create)
    *   **Type / Scope**: Master Board Asset
    *   **Proposed Changes**: Vector artwork for 40 tiles featuring Saigon landmarks (Chợ Bến Thành, Nhà Thờ Đức Bà, Bitexco, Bến Bạch Đằng) with discrete SVG group identifiers (`#tile_0` to `#tile_39`).
*   **`assets/svg/pieces.svg`** (Create)
    *   **Type / Scope**: Pawn and Building Tokens
    *   **Proposed Changes**: Vector markers for 4 player tokens (Vespa scooter, Phin cà phê, Nón lá, Xe xích lô) and building models (nhà phố, biệt thự).
*   **`scripts/sanitize_svg.py`** (Create)
    *   **Type / Scope**: Asset Build Tool
    *   **Proposed Changes**: Python script that cleans raw Inkscape SVG exports by stripping sodipodi/inkscape metadata, removing unused XML namespaces, and optimizing path syntax.

#### 7. CI/CD Automation

*   **`.github/workflows/ci.yml`** (Create)
    *   **Type / Scope**: GitHub Actions Workflow
    *   **Proposed Changes**: Automated test pipeline that checks code formatting (`cargo fmt`), runs linter (`cargo clippy -D warnings`), executes unit and integration tests (`cargo test --workspace`), and validates WASM compilation via `trunk build`.

---

### Verification Plan

#### 1. Automated Test Suite
*   **Unit Tests (`monopoly-core`)**:
    *   Command: `cargo test -p monopoly-core`
    *   Verification: 100% pass on 40-tile indexing, money math edge cases (preventing overflow or fractional VND), monopoly detection across all 8 color sets, rent calculation with 1–4 houses and hotel, and jail escape mechanics.
*   **Server Integration Tests (`monopoly-server`)**:
    *   Command: `cargo test -p monopoly-server`
    *   Verification: Validate room initialization, 4-client WebSocket handshake, authoritative dice roll validation, 30s turn timeout triggering the "Xe Ôm" bot, and reconnection using `session_token`.
*   **Client WASM Build Verification**:
    *   Command: `cd crates/monopoly-client && trunk build --release`
    *   Verification: Ensure successful WebAssembly compilation with total bundle payload (`wasm` + `js` + `assets`) under 500 KB compressed.

#### 2. Manual & Cross-Device Ergonomics Verification
*   **Desktop Browser Verification**:
    *   Launch local server (`cargo run -p monopoly-server`), open 4 browser tabs to `http://localhost:8080`, and verify full 40-tile square view, dice roll animations, ledger updates, and turn handoffs.
*   **Mobile Emulation Verification**:
    *   Using Chrome DevTools device mode (iPhone 14 / Pixel 7 viewports), verify that the dynamic quadrant zoom follows active pawn movement, the minimap displays accurate player pins, and touch buttons have minimum 44px hit areas.
*   **Network Disruption & Bot Takeover Verification**:
    *   Kill one client connection during an active turn; verify that the server waits 15 seconds, announces bot takeover, automatically rolls and ends turn, and allows the reconnected client to resume ownership upon page reload.


--------------------------------------------------------------------------------

<!-- Provenance: Indicates initial structural drafting via EGM. -->

*Drafted with EGM*

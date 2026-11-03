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

## Resolved Architectural Questions

*   **Question 1: Server Room Cleanup Policy**: How should the server determine room cleanup when all connected human players disconnect simultaneously during an active game?
    *   **Decision**: The room actor initiates a 5-minute teardown timer whenever the count of active human WebSocket connections drops to zero. If any player reconnects with a valid `session_token` within 5 minutes, the timer is aborted. If the timer elapses without active connections, the room actor unregisters itself from the central room registry and shuts down to release memory.
*   **Question 2: Mobile Viewport & Camera Control**: For the mobile portrait layout, should tile zoom follow active player movement automatically, or allow manual pan-and-zoom touch controls alongside the mini-map?
    *   **Decision**: The viewport defaults to automatic quadrant auto-follow tracking active player token coordinates, with smooth CSS transforms. An interactive 160x160 mini-map allows manual quadrant inspection on tap; tapping a mini-map quadrant pauses auto-follow until the next turn event or a dedicated "Recenter" tap.

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

Implementation tasks are partitioned into five sequential, verifiable phases. Each phase establishes a stable, buildable, and testable checkpoint.

```
Phase 1: Workspace & Protocol Contracts
   │
   ▼
Phase 2: Deterministic Domain State Machine & Saigon Catalogs
   │
   ▼
Phase 3: Axum WebSocket Server, Actor Runtime & "Xe Ôm" Bot
   │
   ▼
Phase 4: Leptos WASM Client, Inline SVG Board & Coordinate Mapping
   │
   ▼
Phase 5: Asset Tooling, Verification Test Suites & CI/CD Automation
```

---

### Phase 1: Workspace Configuration & Protocol Contracts

**Goal**: Establish the virtual Cargo workspace, compilation profiles, and shared wire protocol serialization models.

#### Target Files to Create

*   **`Cargo.toml`** (Create)
    *   **Scope**: Cargo Workspace Virtual Manifest
    *   **Specification**: Define top-level workspace members: `crates/monopoly-core`, `crates/monopoly-proto`, `crates/monopoly-server`, `crates/monopoly-client`.
    *   **Release Profiles**: Configure `[profile.release]` with `lto = true`, `opt-level = 3`, `codegen-units = 1`, and `strip = "symbols"` for WASM size minimization.
*   **`.gitignore`** (Create)
    *   **Scope**: Version Control Tooling
    *   **Specification**: Ignore `/target`, `crates/monopoly-client/dist`, `.env`, and IDE artifacts.
*   **`README.md`** (Create)
    *   **Scope**: Project Documentation
    *   **Specification**: Document local development prerequisites (Rust 1.80+, `wasm32-unknown-unknown` toolchain target, `trunk`), build and test commands, and architectural summary.
*   **`crates/monopoly-proto/Cargo.toml`** (Create)
    *   **Scope**: Crate Manifest
    *   **Specification**: Declare dependencies: `serde = { version = "1.0", features = ["derive"] }`, `serde_json = "1.0"`, `uuid = { version = "1.10", features = ["v4", "serde"] }`, and `thiserror = "1.0"`.
*   **`crates/monopoly-proto/src/lib.rs`** (Create)
    *   **Scope**: Shared Wire Protocol Types
    *   **Specification**: Enumerate exact client/server communication messages and payloads:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoomId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionToken(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Money(pub i64); // 1 unit = 1,000 VNĐ

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    JoinRoom {
        player_name: String,
        session_token: Option<SessionToken>,
    },
    RollDice,
    BuyProperty,
    DeclineProperty,
    PlaceBid {
        amount: Money,
    },
    PassAuction,
    BuildHouse {
        tile_index: u8,
    },
    SellHouse {
        tile_index: u8,
    },
    MortgageProperty {
        tile_index: u8,
    },
    UnmortgageProperty {
        tile_index: u8,
    },
    PayJailBail,
    UseJailCard,
    DeclareBankruptcy,
    EndTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    RoomJoined {
        room_id: RoomId,
        player_id: PlayerId,
        session_token: SessionToken,
        seat_index: u8,
    },
    GameStarted {
        turn_order: Vec<PlayerId>,
    },
    TurnStarted {
        active_player: PlayerId,
        time_limit_seconds: u32,
    },
    TimerTick {
        seconds_remaining: u32,
    },
    DiceRolled {
        player_id: PlayerId,
        dice: (u8, u8),
        new_tile_index: u8,
        is_doubles: bool,
    },
    PropertyAcquired {
        player_id: PlayerId,
        tile_index: u8,
        cost: Money,
    },
    RentPaid {
        payer: PlayerId,
        recipient: PlayerId,
        tile_index: u8,
        amount: Money,
    },
    AuctionStarted {
        tile_index: u8,
        opening_bid: Money,
    },
    BidPlaced {
        bidder: PlayerId,
        amount: Money,
    },
    AuctionConcluded {
        winner: Option<PlayerId>,
        final_price: Money,
    },
    HouseModified {
        tile_index: u8,
        house_count: u8,
    },
    PropertyMortgageChanged {
        tile_index: u8,
        is_mortgaged: bool,
    },
    CardDrawn {
        player_id: PlayerId,
        deck: String, // "Cơ Hội" | "Khí Vận"
        description: String,
        effect_summary: String,
    },
    PlayerBankrupt {
        player_id: PlayerId,
    },
    BotAssumedControl {
        player_id: PlayerId,
        reason: String, // "Timeout" | "Disconnected"
    },
    PlayerReconnected {
        player_id: PlayerId,
    },
    SyncSnapshot {
        state: serde_json::Value,
    },
    ActionRejected {
        reason: String,
    },
}
```

**Phase 1 Checkpoint**: `cargo check -p monopoly-proto` executes without compiler warnings or errors.

---

### Phase 2: Deterministic Domain State Machine & Saigon Catalogs

**Goal**: Implement the deterministic game rules, money value object, complete 40-tile Saigon street layout, localized card decks, and pure reducer transitions in `crates/monopoly-core`.

#### Target Files to Create

*   **`crates/monopoly-core/Cargo.toml`** (Create)
    *   **Scope**: Crate Manifest
    *   **Specification**: Declare dependencies: `monopoly-proto = { path = "../monopoly-proto" }`, `rand = { version = "0.8", default-features = false }`, `rand_chacha = "0.3"`, `serde = { version = "1.0", features = ["derive"] }`, and `thiserror = "1.0"`.
*   **`crates/monopoly-core/src/lib.rs`** (Create)
    *   **Scope**: Core Library Entry
    *   **Specification**: Re-export modules `money`, `board`, `cards`, `state`, `rules`.
*   **`crates/monopoly-core/src/money.rs`** (Create)
    *   **Scope**: Currency Value Object
    *   **Specification**: Implement `Money(pub i64)` arithmetic operations (addition, subtraction, scaling). Enforce that 1 unit = 1,000 VNĐ.
        *   Methods: `Money::from_vnd(vnd: i64) -> Self` (`Money(vnd / 1000)`), `Money::as_vnd(&self) -> i64` (`self.0 * 1000`).
        *   `Display` implementation: Format as Vietnamese Dong with period thousand separators (e.g., `Money(2_000)` formats as `"2.000.000 đ"`; `Money(-500)` formats as `"-500.000 đ"`).
*   **`crates/monopoly-core/src/board.rs`** (Create)
    *   **Scope**: Tile Data Structures & Static 40-Tile Saigon Catalog
    *   **Specification**: Define concrete types and the static 40-tile array:

```rust
use crate::money::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum District {
    District4,   // Brown / Bình Dân 1
    District8,   // Light Blue / Bình Dân 2
    PhuNhuan,    // Pink / Trung Cấp 1
    TanBinh,     // Orange / Trung Cấp 2
    District5,   // Red / Thương Gia 1
    District3,   // Yellow / Thương Gia 2
    District2,   // Green / Cao Cấp 1 (Thảo Điền)
    District1,   // Dark Blue / Cao Cấp 2 (Bến Nghé)
}

#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub name: &'static str,
    pub district: District,
    pub cost: Money,
    pub house_cost: Money,
    pub mortgage_value: Money,
    pub rent_table: [Money; 6], // Base, 1H, 2H, 3H, 4H, Hotel
}

#[derive(Debug, Clone)]
pub struct StationDef {
    pub name: &'static str,
    pub cost: Money,
    pub mortgage_value: Money,
}

#[derive(Debug, Clone)]
pub struct UtilityDef {
    pub name: &'static str,
    pub cost: Money,
    pub mortgage_value: Money,
}

#[derive(Debug, Clone)]
pub struct TaxDef {
    pub name: &'static str,
    pub amount: Money,
}

#[derive(Debug, Clone)]
pub enum TileKind {
    Go,
    Property(PropertyDef),
    CommunityChest, // Khí Vận
    Tax(TaxDef),
    Station(StationDef),
    Chance,         // Cơ Hội
    Jail,
    Utility(UtilityDef),
    FreeParking,
    GoToJail,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub index: u8,
    pub name: &'static str,
    pub kind: TileKind,
}
```

##### 40-Tile Static Saigon Definition Table

| Index | Name | TileKind | Cost (VNĐ) | Base Rent | Multi-House Rents (1H / 2H / 3H / 4H / Hotel) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **0** | **Khởi Hành (GO)** | `Go` | — | Collect 2.000.000 đ on pass | — |
| **1** | Bến Vân Đồn | `Property(District4)` | 600.000 đ | 20.000 đ | 100k / 300k / 900k / 1.6M / 2.5M |
| **2** | Khí Vận 1 | `CommunityChest` | — | Draw Card | — |
| **3** | Đoàn Văn Bơ | `Property(District4)` | 600.000 đ | 40.000 đ | 200k / 600k / 1.8M / 3.2M / 4.5M |
| **4** | Thuế Thu Nhập | `Tax` | 2.000.000 đ | Flat fee | — |
| **5** | Bến Xe Miền Đông | `Station` | 2.000.000 đ | 250.000 đ | Scaled: 250k / 500k / 1.0M / 2.0M |
| **6** | Phạm Thế Hiển | `Property(District8)` | 1.000.000 đ | 60.000 đ | 300k / 900k / 2.7M / 4.0M / 5.5M |
| **7** | Cơ Hội 1 | `Chance` | — | Draw Card | — |
| **8** | Tạ Quang Bửu | `Property(District8)` | 1.000.000 đ | 60.000 đ | 300k / 900k / 2.7M / 4.0M / 5.5M |
| **9** | Dương Bá Trạc | `Property(District8)` | 1.200.000 đ | 80.000 đ | 400k / 1.0M / 3.0M / 4.5M / 6.0M |
| **10** | **Vào Tù / Thăm Tù** | `Jail` | — | Just Visiting / In Jail | — |
| **11** | Phan Xích Long | `Property(PhuNhuan)` | 1.400.000 đ | 100.000 đ | 500k / 1.5M / 4.5M / 6.25M / 7.5M |
| **12** | Công Ty Cấp Nước Sawaco | `Utility` | 1.500.000 đ | 4x / 10x dice | 4x dice if 1 owned, 10x if both owned |
| **13** | Phan Đăng Lưu | `Property(PhuNhuan)` | 1.400.000 đ | 100.000 đ | 500k / 1.5M / 4.5M / 6.25M / 7.5M |
| **14** | Hoàng Văn Thụ | `Property(PhuNhuan)` | 1.600.000 đ | 120.000 đ | 600k / 1.8M / 5.0M / 7.0M / 9.0M |
| **15** | Ga Sài Gòn | `Station` | 2.000.000 đ | 250.000 đ | Scaled: 250k / 500k / 1.0M / 2.0M |
| **16** | Cộng Hòa | `Property(TanBinh)` | 1.800.000 đ | 140.000 đ | 700k / 2.0M / 5.5M / 7.5M / 9.5M |
| **17** | Khí Vận 2 | `CommunityChest` | — | Draw Card | — |
| **18** | Trường Chinh | `Property(TanBinh)` | 1.800.000 đ | 140.000 đ | 700k / 2.0M / 5.5M / 7.5M / 9.5M |
| **19** | Lê Văn Sỹ | `Property(TanBinh)` | 2.000.000 đ | 160.000 đ | 800k / 2.2M / 6.0M / 8.0M / 10.0M |
| **20** | **Bãi Đỗ Xe Miễn Phí** | `FreeParking` | — | Safe Rest Tile | — |
| **21** | Hải Thượng Lãn Ông | `Property(District5)` | 2.200.000 đ | 180.000 đ | 900k / 2.5M / 7.0M / 8.75M / 10.5M |
| **22** | Cơ Hội 2 | `Chance` | — | Draw Card | — |
| **23** | Châu Văn Liêm | `Property(District5)` | 2.200.000 đ | 180.000 đ | 900k / 2.5M / 7.0M / 8.75M / 10.5M |
| **24** | An Dương Vương | `Property(District5)` | 2.400.000 đ | 200.000 đ | 1.0M / 3.0M / 7.5M / 9.25M / 11.0M |
| **25** | Bến Xe Miền Tây | `Station` | 2.000.000 đ | 250.000 đ | Scaled: 250k / 500k / 1.0M / 2.0M |
| **26** | Nguyễn Thị Minh Khai | `Property(District3)` | 2.600.000 đ | 220.000 đ | 1.1M / 3.3M / 8.0M / 9.75M / 11.5M |
| **27** | Nam Kỳ Khởi Nghĩa | `Property(District3)` | 2.600.000 đ | 220.000 đ | 1.1M / 3.3M / 8.0M / 9.75M / 11.5M |
| **28** | Tổng Công Ty Điện Lực EVN | `Utility` | 1.500.000 đ | 4x / 10x dice | 4x dice if 1 owned, 10x if both owned |
| **29** | Võ Văn Tần | `Property(District3)` | 2.800.000 đ | 240.000 đ | 1.2M / 3.6M / 8.5M / 10.25M / 12.0M |
| **30** | **Vào Tù Ngay Lập Tức** | `GoToJail` | — | Move to Tile 10 (`Jail`) | — |
| **31** | Trần Não | `Property(District2)` | 3.000.000 đ | 260.000 đ | 1.3M / 3.9M / 9.0M / 11.0M / 12.75M |
| **32** | Song Hành | `Property(District2)` | 3.000.000 đ | 260.000 đ | 1.3M / 3.9M / 9.0M / 11.0M / 12.75M |
| **33** | Khí Vận 3 | `CommunityChest` | — | Draw Card | — |
| **34** | Xuân Thủy (Thảo Điền) | `Property(District2)` | 3.200.000 đ | 280.000 đ | 1.5M / 4.5M / 10.0M / 12.0M / 14.0M |
| **35** | Tuyến Metro Số 1 | `Station` | 2.000.000 đ | 250.000 đ | Scaled: 250k / 500k / 1.0M / 2.0M |
| **36** | Cơ Hội 3 | `Chance` | — | Draw Card | — |
| **37** | Đồng Khởi | `Property(District1)` | 3.500.000 đ | 350.000 đ | 1.75M / 5.0M / 11.0M / 13.0M / 15.0M |
| **38** | Thuế Xa Xỉ Phẩm | `Tax` | 1.000.000 đ | Flat fee | — |
| **39** | Nguyễn Huệ | `Property(District1)` | 4.000.000 đ | 500.000 đ | 2.0M / 6.0M / 14.0M / 17.0M / 20.0M |

*   **`crates/monopoly-core/src/cards.rs`** (Create)
    *   **Scope**: Localized Card Deck Catalogs & Effect Logic
    *   **Specification**: Define `Card` struct and `CardEffect` enum:
        *   `CardEffect::CashDelta(Money)`: e.g., Trúng vé số kiến thiết (+1.000.000 đ), Sửa xe ngập nước đường Nguyễn Hữu Cảnh (-500.000 đ).
        *   `CardEffect::MoveTo(u8)`: e.g., Đi thẳng đến Bến Thành (Tile 0), Tiến vào Nguyễn Huệ (Tile 39).
        *   `CardEffect::GoToJail`: Bị giữ xe do vi phạm nồng độ cồn, vào tù ngay lập tức.
        *   `CardEffect::GetOutOfJailFree`: Thẻ cứu trợ được bảo lãnh miễn phí.
        *   `CardEffect::BuildingRepairs { house: Money, hotel: Money }`: Trùng tu nhà cửa sau mùa mưa ngập.
*   **`crates/monopoly-core/src/state.rs`** (Create)
    *   **Scope**: Game Domain Entities
    *   **Specification**: Define `GameState`, `PlayerState`, `TurnPhase`, `PropertyOwnership`, and `AuctionSession`. Ensure all types derive `Clone`, `Debug`, `Serialize`, `Deserialize`.
*   **`crates/monopoly-core/src/rules.rs`** (Create)
    *   **Scope**: Pure State Reducer
    *   **Specification**: Implement `apply_action(state: &GameState, action: &ClientMessage, rng: &mut impl rand::Rng) -> Result<(GameState, Vec<ServerMessage>), GameRuleError>`. Enforce:
        *   Doubles grant bonus roll; 3 consecutive doubles triggers immediate jail dispatch.
        *   Full district monopoly doubles base unimproved rent.
        *   Rent calculation for stations (250k * 2^(stations_owned - 1)) and utilities (4x dice sum or 10x if both utilities owned).
        *   Bankruptcy asset transfer to creditor player, or mortgage liquidation to bank.

**Phase 2 Checkpoint**: `cargo test -p monopoly-core` runs unit tests verifying all 40 tiles, rent tables, card effects, and transition logic.

---

### Phase 3: Axum Server Runtime, Actor Runtime & "Xe Ôm" Bot Engine

**Goal**: Build the authoritative multiplayer server using Axum, per-room Tokio actors, countdown timers, and automated heuristic bot execution.

#### Target Files to Create

*   **`crates/monopoly-server/Cargo.toml`** (Create)
    *   **Scope**: Crate Manifest
    *   **Specification**: Declare dependencies: `axum = { version = "0.7", features = ["ws"] }`, `tokio = { version = "1.40", features = ["full"] }`, `tower-http = { version = "0.5", features = ["cors", "trace"] }`, `dashmap = "6.0"`, `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `serde_json = "1.0"`, `uuid = { version = "1.10", features = ["v4"] }`, `monopoly-proto`, and `monopoly-core`.
*   **`crates/monopoly-server/src/main.rs`** (Create)
    *   **Scope**: Server Application Entrypoint
    *   **Specification**: Bind TCP listener (`0.0.0.0:8080`), initialize structured logging with `tracing-subscriber`, instantiate shared room registry `Arc<DashMap<String, mpsc::Sender<RoomCommand>>>`, and attach Tokio graceful shutdown listeners (`SIGINT`/`SIGTERM`).
*   **`crates/monopoly-server/src/router.rs`** (Create)
    *   **Scope**: HTTP & WebSocket Endpoint Routing
    *   **Specification**:
        *   `GET /healthz`: Returns HTTP 200 `OK`.
        *   `POST /api/rooms`:
            *   Request Payload: `{ "player_name": String }`
            *   Response Payload: `{ "room_id": String, "session_token": String }`
            *   Spawns a new room actor and inserts the channel sender into the `DashMap` registry.
        *   `GET /ws`: Upgrades connection to WebSocket. Extracts query parameter `?room={room_id}`. Rejects connection if `room_id` is missing or not found in registry.
*   **`crates/monopoly-server/src/room.rs`** (Create)
    *   **Scope**: Room Actor Coordination & Lifecycle
    *   **Specification**:
        *   Define `RoomCommand` enum: `ClientConnected(WebSocketSender, SessionToken)`, `ClientDisconnected(PlayerId)`, `ClientMessageReceived(PlayerId, ClientMessage)`, `TickTimer`, `CloseEmptyRoom`.
        *   Actor loop manages room state, seat allocation, connected socket channels, and session recovery mappings (`HashMap<SessionToken, PlayerId>`).
        *   **Empty Room Cleanup**: If all human players disconnect, the room actor initiates a 5-minute countdown. If no player reconnects before 5 minutes elapse, the room actor exits and deregisters itself from the shared registry.
*   **`crates/monopoly-server/src/timer.rs`** (Create)
    *   **Scope**: Turn Timer & Disconnect Arbitration
    *   **Specification**: Implement timer coordination using `tokio::select!`:
        *   When a turn starts for an active player, spawn a 30-second interval emitting `TimerTick` every second.
        *   If an active player's socket disconnects, spawn a concurrent 15-second `GraceTimer`.
        *   Whichever timer reaches zero first triggers the "Xe Ôm" bot takeover.
        *   If the disconnected player reconnects with their valid `session_token` before the 15-second grace window expires, abort `GraceTimer` and restore human control.
        *   If a player's turn begins while already disconnected, trigger bot action after a 3-second simulation delay.
*   **`crates/monopoly-server/src/bot.rs`** (Create)
    *   **Scope**: "Xe Ôm" Fallback Heuristic Bot
    *   **Specification**: Automated decision rules:
        *   Roll phase: Execute `RollDice` immediately.
        *   Unowned property: If liquid cash balance >= 2x tile cost, execute `BuyProperty`; otherwise, execute `DeclineProperty`.
        *   Auction phase: If active bid < 50% of tile property cost and cash >= 3x current bid, execute `PlaceBid(current_bid + 50.000 đ)`; otherwise execute `PassAuction`.
        *   Jail phase: If in jail and holds a Get-Out-of-Jail card, execute `UseJailCard`; if cash > 5.000.000 đ, execute `PayJailBail`; otherwise execute `RollDice`.
        *   Debt liquidation: Mortgage unimproved properties starting from lowest base rent upward until debt is covered.
        *   End turn: Execute `EndTurn` once mandatory actions resolve.

**Phase 3 Checkpoint**: `cargo test -p monopoly-server` passes integration tests verifying room creation, WebSocket message dispatch, timer timeouts, and bot takeover.

---

### Phase 4: Leptos WebAssembly Client & Responsive SVG Board

**Goal**: Deliver the lightweight, mobile-responsive vector frontend in WebAssembly with zero external WebGL runtime dependencies.

#### Target Files to Create

*   **`crates/monopoly-client/Cargo.toml`** (Create)
    *   **Scope**: Client Crate Manifest
    *   **Specification**: Declare dependencies: `leptos = { version = "0.6", features = ["csr"] }`, `wasm-bindgen = "0.2"`, `web-sys = { version = "0.3", features = ["Storage", "Window"] }`, `gloo-net = { version = "0.5", features = ["websocket"] }`, `serde = { version = "1.0", features = ["derive"] }`, `serde_json = "1.0"`, `monopoly-proto`, and `monopoly-core`.
*   **`crates/monopoly-client/Trunk.toml`** (Create)
    *   **Scope**: Trunk WASM Bundler Configuration
    *   **Specification**: Output directory `dist`, asset copying for SVG files and audio clips, and release optimizations. Add pre-build hook:
        ```toml
        [[hooks]]
        stage = "pre_build"
        command = "python"
        command_arguments = ["../../scripts/sanitize_svg.py"]
        ```
*   **`crates/monopoly-client/index.html`** (Create)
    *   **Scope**: HTML Application Container
    *   **Specification**: Viewport meta tag `width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no`, link Google Fonts (`Be Vietnam Pro` for Vietnamese glyph rendering), and mount target `<div id="app"></div>`.
*   **`crates/monopoly-client/src/main.rs`** (Create)
    *   **Scope**: Client Entrypoint
    *   **Specification**: Mount `App` component into `#app` and initialize `console_error_panic_hook`.
*   **`crates/monopoly-client/src/app.rs`** (Create)
    *   **Scope**: Root Leptos Component & Route State
    *   **Specification**: Manage view state between Lobby view (name entry, room code, join/create buttons) and Game Session view.
*   **`crates/monopoly-client/src/ws.rs`** (Create)
    *   **Scope**: Client WebSocket Connection Manager
    *   **Specification**: Maintain `gloo_net::websocket::futures::WebSocket`. Dispatch incoming `ServerMessage` to Leptos signals. Store `session_token` in `window.localStorage`. On network drop, attempt reconnection every 2 seconds with stored token.
*   **`crates/monopoly-client/src/components/board.rs`** (Create)
    *   **Scope**: Inline SVG Board Component & Coordinate Engine
    *   **Specification**: Render an inline `<svg viewBox="0 0 1000 1000">`. Provide mathematical coordinate mapping:
        *   Canvas: 1000x1000 px.
        *   Corner Tiles (Indices 0, 10, 20, 30): 130x130 px.
        *   Regular Perimeter Tiles (9 per side): Width = 82.22 px, Height = 130 px.
        *   Coordinate Lookup Formula for Tile Center `(x, y)`:
            *   Bottom Row (Tiles 0..10): `y = 935.0`; `x = 935.0 - (index * 87.0)`.
            *   Left Column (Tiles 10..20): `x = 65.0`; `y = 935.0 - ((index - 10) * 87.0)`.
            *   Top Row (Tiles 20..30): `y = 65.0`; `x = 65.0 + ((index - 20) * 87.0)`.
            *   Right Column (Tiles 30..39): `x = 935.0`; `y = 65.0 + ((index - 30) * 87.0)`.
        *   **Procedural Fallback Mode**: If SVG asset files are missing, render geometric SVG rectangles with hex color headers for districts, black text labels, and prices.
*   **`crates/monopoly-client/src/components/minimap.rs`** (Create)
    *   **Scope**: Mobile Navigation Minimap
    *   **Specification**: 160x160 px persistent overview map. Displays player pawn dots and district color ownership. Tapping a quadrant navigates the main viewport camera.
*   **`crates/monopoly-client/src/components/controls.rs`** (Create)
    *   **Scope**: Action Button Toolbar
    *   **Specification**: Contextual buttons ("Lắc Xúc Xắc", "Mua Bất Động Sản", "Đấu Giá", "Xây Nhà", "Kết Thúc Lượt"). Buttons adhere to minimum 44x44px touch targets. Animated CSS countdown bar for 30s timer.
*   **`crates/monopoly-client/src/components/ledger.rs`** (Create)
    *   **Scope**: Player Balances & Asset Cards
    *   **Specification**: Display all 4 players' name, cash balance formatted via `Money::to_string()`, owned property deeds, and status badges (In Jail, Disconnected, Xe Ôm Bot Active).
*   **`crates/monopoly-client/src/components/log.rs`** (Create)
    *   **Scope**: Event History Feed
    *   **Specification**: Chronological localized game log messages.

**Phase 4 Checkpoint**: `cd crates/monopoly-client && trunk build --release` compiles client WASM bundle with output size under 500 KB compressed.

---

### Phase 5: Build Tooling, Test Fixtures & CI/CD Pipeline

**Goal**: Establish SVG asset optimization, automated test fixtures with time manipulation and seeded RNGs, and continuous integration workflows.

#### Target Files to Create

*   **`scripts/sanitize_svg.py`** (Create)
    *   **Scope**: Build Asset Optimizer
    *   **Specification**: Python 3 script using `xml.etree.ElementTree` to strip Inkscape/Sodipodi metadata, remove unreferenced layers, round floating point paths to 2 decimal places, and minify SVG output.
*   **`crates/monopoly-core/tests/helpers/mod.rs`** (Create)
    *   **Scope**: Core Test Infrastructure
    *   **Specification**: Provide `create_test_game() -> GameState` and `create_seeded_rng(seed: u64) -> ChaCha8Rng` for deterministic dice rolls and reproducible card draws.
*   **`crates/monopoly-server/tests/helpers/mod.rs`** (Create)
    *   **Scope**: Server Test Infrastructure
    *   **Specification**: Provide mock WebSocket client connected via in-process TCP listener on port `127.0.0.1:0`. Include helper to accelerate timers using `tokio::time::pause()` and `tokio::time::advance()`.
*   **`.github/workflows/ci.yml`** (Create)
    *   **Scope**: GitHub Actions CI Pipeline
    *   **Specification**:
        *   Step 1: Check formatting (`cargo fmt --all -- --check`).
        *   Step 2: Clippy linter (`cargo clippy --all-targets --all-features -- -D warnings`).
        *   Step 3: Run full test suite (`cargo test --workspace`).
        *   Step 4: Execute SVG sanitizer (`python scripts/sanitize_svg.py --check`).
        *   Step 5: Verify WASM compilation (`trunk build --release --manifest-path crates/monopoly-client/Cargo.toml`).

**Phase 5 Checkpoint**: `cargo test --workspace` executes and passes all unit, integration, and property test suites.

---

### Verification Plan

#### 1. Automated Test Suites & Assertion Matrices

##### A. Domain State Machine Test Matrix (`crates/monopoly-core/tests/rules_test.rs`)

| Test Function | Input Conditions | Expected Assertion |
| :--- | :--- | :--- |
| `test_money_math_overflow_and_formatting` | `Money(2_000_000)` and `Money(-500_000)` | String output matches `"2.000.000 đ"` and `"-500.000 đ"`. Arithmetic prevents negative balance unless explicitly flagged bankrupt. |
| `test_board_indexing_and_pass_go` | Player on Tile 38 rolls 4 (reaches Tile 2) | Tile index resolves to 2; cash increases by +2.000.000 đ (`PASS_GO_REWARD`). |
| `test_property_acquisition_and_ledger` | Player lands on unowned Nguyễn Huệ (Tile 39) with 5M VNĐ and executes `BuyProperty` | Cash balance decrements by 4M VNĐ; `property_owners[39]` equals `player_id`. |
| `test_monopoly_base_rent_doubling` | Player owns all 3 District 3 tiles with 0 houses. Opponent lands on Tile 26. | Base rent of 220.000 đ doubles to 440.000 đ. |
| `test_station_escalating_rent` | Landlord owns 1, 2, 3, or 4 stations when opponent lands on station | Rent correctly asserts at 250k, 500k, 1.000k, and 2.000k VNĐ respectively. |
| `test_utility_dice_multiplier_rent` | Landlord owns Sawaco (1 utility) vs. Sawaco + EVN (2 utilities); dice sum = 7 | Rent asserts at 28.000 đ (4x dice) vs. 70.000 đ (10x dice). |
| `test_doubles_and_consecutive_jail` | Player rolls (4,4), then (2,2), then (6,6) | Turn 1 & 2 award bonus roll; 3rd consecutive doubles immediately transports pawn to Tile 10 (`Jail`) and ends turn. |
| `test_jail_bail_and_card_mechanics` | Player in jail pays 500k bail or uses Get-Out-of-Jail card | `in_jail` flag clears; movement resumes normally on next roll. |
| `test_bankruptcy_asset_surrender` | Player with 200k cash and 1 property owes 1.5M rent to opponent | Cash transfers to opponent, property deed transfers to opponent, debtor is flagged `is_bankrupt = true`. |

##### B. Server & Actor Integration Test Matrix (`crates/monopoly-server/tests/ws_integration_test.rs`)

| Test Function | Scenario Description | Expected Assertion |
| :--- | :--- | :--- |
| `test_room_creation_and_4player_join` | `POST /api/rooms` creates room; 4 mock WS clients connect and send `JoinRoom`. | All 4 clients receive `RoomJoined` and final client trigger broadcasts `GameStarted`. |
| `test_turn_timer_expiration_and_bot_takeover` | Active player client remains idle; test uses `tokio::time::advance(Duration::from_secs(30))` | Server emits `BotAssumedControl { reason: "Timeout" }` and automated dice roll event is broadcast. |
| `test_transient_disconnect_and_reconnection` | Player 2 abruptly drops socket during active turn. Reconnects at second 10. | Reconnect with `session_token` cancels disconnect grace timer; Player 2 retains active turn control. |
| `test_prolonged_disconnect_grace_expiry` | Player 2 socket drops; test uses `tokio::time::advance(Duration::from_secs(15))` | Server emits `BotAssumedControl { reason: "Disconnected" }` and bot completes Player 2's turn. |
| `test_empty_room_cleanup_after_5_minutes` | All 4 clients close sockets; test advances time by 5 minutes | Room actor exits cleanly and `DashMap` registry removes room entry. |

##### C. Client WebAssembly Verification

| Verification Target | Command | Acceptance Criteria |
| :--- | :--- | :--- |
| Compilation & Bundle Size | `cd crates/monopoly-client && trunk build --release` | Zero build errors. Compressed output bundle (`wasm` + `js` + assets) <= 500 KB. |
| Inline SVG Coordinate Alignment | Automated headless browser test (wasm-bindgen-test) | Pawn center coordinate for tile 0 asserts at `(935.0, 935.0)` and tile 20 asserts at `(65.0, 65.0)`. |

#### 2. Manual & Ergonomics Verification
*   **Desktop Multi-Tab Verification**: Run `cargo run -p monopoly-server`. Open 4 browser tabs to `http://localhost:8080`. Verify room creation link sharing, synchronized pawn translations, ledger updates, and turn handoffs.
*   **Mobile Device Emulation**: Open Chrome DevTools in Pixel 7 and iPhone 14 viewports. Verify automatic quadrant camera centering following pawn motion, minimap tap navigation, and minimum 44px touch targets on all control buttons.
*   **Network Interrupt Drill**: Disconnect network in browser DevTools. Verify that reconnection within 15 seconds seamlessly resumes match control using the saved `session_token`.

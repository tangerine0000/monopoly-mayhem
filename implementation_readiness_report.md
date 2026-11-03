# Implementation Readiness Report (Stage 9)

**Target Document**: [`docs/designs/vietnamese_monopoly_saigon.md`](file:///c:/Users/tanpe/monopoly-mayhem/docs/designs/vietnamese_monopoly_saigon.md)  
**Evaluated Scope**: Lines 209–368 (`## Detailed Implementation Plan` & `### Verification Plan`)  
**Status**: Resolved  
**Standard**: [`implementation-readiness-standard.md`](file:///C:/Users/tanpe/.gemini/config/plugins/egm/resources/document_standards/implementation-readiness-standard.md)  
**Reviewer Lenses**:
- `egm-implementation-readiness` (Technical Lead & Implementing Engineer)
- `egm-codebase-details` (Technical Quality & Compliance Auditor)
- `egm-style` (Technical Copy Editor)

---

## Reviewer Status

| Reviewer | Lens | Status |
| :--- | :--- | :--- |
| `egm-implementation-readiness` | Pragmatic Tech Lead & Zero-Decision Enforcement | Completed |
| `egm-codebase-details` | Technical Quality, Scope & Verification Auditor | Completed |
| `egm-style` | Tone, Conciseness, Objectivity & Editorial Standards | Completed |

---

## Executive Summary

The implementation plan provides a structured architectural partition across four crates (`monopoly-proto`, `monopoly-core`, `monopoly-server`, and `monopoly-client`), build tooling, and CI workflows. However, it fails the **Implementation Readiness Standard** due to multiple **Zero-Decision Rule** violations, a flat catalog layout lacking incremental phases, undefined domain data models/catalogs, missing protocol message variants for stated game mechanics, and an underspecified test harness.

---

## Prioritized Findings

### 🔴 Blockers

#### 1. Missing Phasing and Incremental Verifiable Checkpoints
* **Reviewer Lens**: `egm-implementation-readiness`
* **Affected Scope**: Lines 209–341 (`## Detailed Implementation Plan`)
* **Discrepancy Description**: The plan presents a flat, file-by-file inventory grouped strictly by crate rather than sequential, incremental delivery phases. The Implementation Readiness Standard mandates:
  > *"The plan must partition the work into logical, incremental phases. Each phase must leave the codebase in a stable, buildable, and testable state."*
  An implementer cannot determine execution order, critical path dependencies, or intermediate verification criteria.
* **Rationale & Recommendation**: Reorganize the implementation plan into ordered, milestone-driven phases:
  - **Phase 1**: Root Cargo workspace manifest and `monopoly-proto` wire message contracts. (Checkpoint: `cargo check -p monopoly-proto`).
  - **Phase 2**: `monopoly-core` deterministic domain models, Saigon board catalog, card decks, and pure reducer rules. (Checkpoint: `cargo test -p monopoly-core`).
  - **Phase 3**: `monopoly-server` Axum WebSocket routing, room manager actor, turn timers, and "Xe Ôm" heuristic bot. (Checkpoint: `cargo test -p monopoly-server`).
  - **Phase 4**: `monopoly-client` Leptos CSR application, SVG board rendering, and WebSocket state synchronization. (Checkpoint: `trunk build`).
  - **Phase 5**: End-to-end integration tests, SVG asset pipeline, and CI automation. (Checkpoint: `cargo test --workspace` and CI run).

#### 2. Undefined Domain Data Schemas, Rent Tables, and Static Catalogs
* **Reviewer Lens**: `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope**: Lines 247–252 (`crates/monopoly-core/src/board.rs`, `cards.rs`)
* **Discrepancy Description**: The plan directs creation of `board.rs` and `cards.rs` but omits the underlying data structures, schemas, and static data catalogs:
  - `board.rs`: The exact struct/enum schema for `Tile` or `TileKind` (e.g. `Property`, `Station`, `Utility`, `Tax`, `Corner`) is missing. Multi-house multiplier formulas, transit station escalation rules (250k, 500k, 1M, 2M), utility multiplier rules (4x/10x dice), and all 40 Saigon street names, costs, mortgage values, and rents are omitted.
  - `cards.rs`: Data structures for `Card`, `DeckType`, and `CardEffect` are missing, as is the localized text and mechanical penalty/reward catalog for "Cơ Hội" and "Khí Vận".
* **Rationale & Recommendation**: Define concrete Rust struct and enum definitions (`Tile`, `TileKind`, `District`, `Card`, `CardEffect`, `RentTable`) and embed the static data table for all 40 tiles and card definitions in the design so the implementer does not invent rules or domain values.

#### 3. Incomplete Wire Protocol for Stated Game Mechanics
* **Reviewer Lens**: `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope**: Lines 232–235 (`crates/monopoly-proto/src/lib.rs`), Lines 108–110 (`TurnPhase`), Line 255 (`AuctionSession`)
* **Discrepancy Description**: The architectural state machine defines phases for `Auction` ("Tile declined; open bidding initiated among players") and property mortgaging/house management. However, `ClientMessage` and `ServerMessage` lack messages to support these mechanics:
  - No auction messaging (`PlaceBid { amount: Money }`, `PassAuction`, `AuctionStarted`, `BidPlaced`, `AuctionConcluded`).
  - No property downgrade messaging (`SellHouse`, `UnmortgageProperty`).
  - No explicit `DeclareBankruptcy` message.
  - The "Xe Ôm" bot heuristic specifies buying property or passing, but omits how the bot handles auctions or when an auction terminates.
* **Rationale & Recommendation**: Either:
  1. Explicitly specify the wire protocol variants and payloads for auctions, house selling, unmortgaging, and bankruptcy, along with bot auction heuristics; OR
  2. Formally scope out auctions and player-to-player trading for v1 MVP (relegating them to Non-Goals) and adjust `TurnPhase` and `ClientMessage` accordingly.

#### 4. Missing Client WebSocket Handshake & Session Reconnection Lifecycle
* **Reviewer Lens**: `egm-codebase-details`
* **Affected Scope**: Lines 271–276 (`crates/monopoly-server/src/router.rs`, `room.rs`), Line 306 (`crates/monopoly-client/src/ws.rs`)
* **Discrepancy Description**: The plan specifies `/ws` upgrades with query parameters (`?room=...`), while `JoinRoom` is listed as an in-band WebSocket payload containing `session_token: Option<String>`. The exact connection lifecycle is undocumented:
  - Sequence: Does the client connect to `/ws?room={room_id}` and immediately send `JoinRoom` as frame 1?
  - Token Issuance: Does `POST /api/rooms` issue a host `session_token`, or is it issued by the server in `ServerMessage::RoomJoined`?
  - Rejection: How are rejected connections or invalid/expired session tokens communicated (WebSocket close code vs. `ServerMessage::ActionRejected`)?
* **Rationale & Recommendation**: Document the explicit WebSocket connection and reconnection lifecycle state machine:
  1. HTTP `POST /api/rooms` creates room and returns `{ "room_id": "...", "host_token": "..." }`.
  2. Client connects to `/ws?room={room_id}` and transmits `ClientMessage::JoinRoom { player_name, session_token }`.
  3. Server validates seat availability or matches token, responds with `ServerMessage::RoomJoined { session_token, seat_index, state }`.
  4. Specify heartbeat ping/pong interval (e.g. 10s ping, 30s timeout) and error closure codes.

---

### 🟡 Majors

#### 5. Undefined REST API Contracts and Room Manager Actor Concurrency
* **Reviewer Lens**: `egm-implementation-readiness`
* **Affected Scope**: Lines 271–276 (`crates/monopoly-server/src/router.rs`, `room.rs`), Lines 38–42 (Open Question 1)
* **Discrepancy Description**:
  - `/api/rooms`: Missing HTTP method (`POST`), request JSON schema, response payload schema, and error codes.
  - `room.rs`: Concurrency architecture is vague ("Manage per-room Tokio actors"). Does not specify the `RoomCommand` enum or the registry type (e.g. `Arc<DashMap<String, mpsc::Sender<RoomCommand>>>`).
  - Open Question 1 ("How should the server determine room cleanup when all connected human players disconnect simultaneously?") remains unresolved in both architecture and implementation steps.
* **Rationale & Recommendation**: Provide explicit Axum handler function signatures, request/response structs, actor channel message enums, and a deterministic room teardown policy (e.g., terminate room actor after a 5-minute grace period if all human sockets are closed).

#### 6. Ambiguous Disconnect vs. Turn Timeout Arbitration Logic
* **Reviewer Lens**: `egm-implementation-readiness`
* **Affected Scope**: Lines 277–282 (`crates/monopoly-server/src/timer.rs`, `bot.rs`), Lines 55–59 (CUJ 3)
* **Discrepancy Description**: The plan specifies both a 30-second turn timer and a 15-second disconnect grace window without defining their precedence:
  - If a player disconnects with 25 seconds remaining, does bot takeover occur at 15s or 30s?
  - If a player is already disconnected when their turn starts, does the turn immediately execute via bot, wait 15s, or wait 30s?
  - How are cancellation tokens managed when a player reconnects mid-countdown?
* **Rationale & Recommendation**: Specify explicit timer arbitration:
  - Active player socket disconnect triggers a 15-second `GraceTimer`.
  - The active turn timer continues running in parallel.
  - Bot takeover occurs upon whichever timer fires first (`tokio::select!`).
  - If a player is already marked disconnected when their turn begins, the bot acts immediately or after a fixed 3-second simulation delay.

#### 7. Unspecified SVG Rendering Mode, Asset Dependencies, and Coordinate Mapping
* **Reviewer Lens**: `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope**: Lines 307–309 (`crates/monopoly-client/src/components/board.rs`), Lines 325–334 (`assets/svg/`)
* **Discrepancy Description**:
  - `board.rs` notes translating pawns onto a 1000x1000 SVG canvas but provides no coordinate calculation formula or 40-slot `(x, y)` lookup table.
  - Embedding method is undefined: external `<img>`/`<object>` tags prevent CSS manipulation and dynamic pawn positioning.
  - The plan makes client implementation depend on creating custom vector assets for 40 Saigon landmarks, creating an external design dependency.
* **Rationale & Recommendation**: Specify inline SVG rendering via Leptos macros, document the mathematical coordinate mapping function for rectangular perimeter tiles, and implement a procedural geometric fallback rendering mode so frontend development is not blocked by custom SVG illustrations.

#### 8. Test Plan Lacks Specific Test Cases, Assertion Matrices, and Test Harness Helpers
* **Reviewer Lens**: `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope**: Lines 345–355 (`### Verification Plan`)
* **Discrepancy Description**: The test plan lists high-level categories rather than concrete test scenarios. It also omits the testing harness architecture:
  - How are WebSocket tests run without sleeping for 30 seconds? (`tokio::time::pause()` / `advance()` must be specified).
  - How is deterministic RNG testing achieved in `monopoly-core`? (Need standard test fixture helpers like `ChaCha8Rng::seed_from_u64`).
  - Missing critical edge case assertions: bankrupt payout when cash is owed to player vs. bank, 3rd consecutive doubles jail dispatch, and house sales balance checks.
* **Rationale & Recommendation**: Expand the test plan into a table of test functions, inputs, and expected assertion outcomes. Document the test fixture helpers (`create_test_game()`, `create_deterministic_rng()`, `tokio::time::pause()`).

---

### 🟢 Nits

#### 9. Money Value Object Scale Multipliers and String Formatting
* **Reviewer Lens**: `egm-implementation-readiness`
* **Affected Scope**: Lines 244–246 (`crates/monopoly-core/src/money.rs`)
* **Discrepancy Description**: `Money(pub i64)` defines 1 unit = 1,000 VNĐ. The plan should provide explicit formatting unit tests (e.g., `Money(2_000)` formats as `"2.000.000 đ"` using period thousand separators) and clarify whether negative values format as `"-500.000 đ"` or `"(500.000 đ)"`.
* **Recommendation**: Document explicit arithmetic scaling methods (`from_vnd`, `as_vnd`) and string display formatting specifications.

#### 10. Missing Build/CI Pipeline Integration for SVG Sanitizer Script
* **Reviewer Lens**: `egm-codebase-details`
* **Affected Scope**: Lines 331–333 (`scripts/sanitize_svg.py`), Lines 292–294 (`Trunk.toml`), Lines 337–340 (`ci.yml`)
* **Discrepancy Description**: `scripts/sanitize_svg.py` is defined, but its execution hook is omitted from `Trunk.toml` hooks and GitHub Actions steps.
* **Recommendation**: Specify whether `sanitize_svg.py` runs as a Trunk pre-build hook (`[[hooks]]` in `Trunk.toml`) or as a CI check step.

#### 11. Editorial Phrasing: Hyperbole, Unverifiable Claims, and Non-Technical Meta-tags
* **Reviewer Lens**: `egm-style`
* **Affected Scope**: Line 211, Line 261, Line 297, Line 309, Lines 366–368
* **Discrepancy Description**:
  - Line 211: "exhaustive implementation plan" uses unprovable hyperbole and acts as introductory filler.
  - Line 261: "Exhaustive automated tests" is unprovable marketing phrasing.
  - Lines 297 & 309: "authentic typography" and "authentic Saigon naming" contain promotional adjectives.
  - Lines 366–368: Tool provenance comments (`<!-- Provenance: ... -->`, `*Drafted with EGM*`) add non-functional noise.
* **Recommendation**: Replace subjective qualifiers with factual descriptions and remove the drafting provenance footer.

# Goldfish Annotated Review Report

**Target:** [`docs/designs/vietnamese_monopoly_saigon.md`](file:///c:/Users/tanpe/monopoly-mayhem/docs/designs/vietnamese_monopoly_saigon.md) (`Lines 209–368`)  
**Status**: resolved

## Prioritized Findings & AI Suggestions

### 1. Missing Phasing and Incremental Verifiable Checkpoints [`egm-implementation-readiness`] - Severity: Blocker
* **Affected Scope**: [Lines 209–341](docs/designs/vietnamese_monopoly_saigon.md#L209-L341) (`## Detailed Implementation Plan`)
* **Discrepancy Description**: The plan presents a flat, file-by-file catalog grouped strictly by crate rather than partitioned, incremental phases with buildable checkpoints.
* **Rationale**: The Implementation Readiness Standard mandates partitioning work into incremental phases that leave the codebase in a stable, testable state.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Structure the implementation section into 5 sequential phases (Phases 1 to 5) with concrete dependencies and verification commands (`cargo check`, `cargo test`, `trunk build`).
* **Proposed Edit**: Reorganize Section 209–341 into Phase 1 (Workspace & Proto), Phase 2 (Core Domain & Catalogs), Phase 3 (Server Runtime & Bot), Phase 4 (Client UI & SVG Board), and Phase 5 (E2E Tests, Assets & CI).

### 2. Undefined Domain Data Schemas, Rent Tables, and Static Catalogs [`egm-implementation-readiness`, `egm-codebase-details`] - Severity: Blocker
* **Affected Scope**: [Lines 247–252](docs/designs/vietnamese_monopoly_saigon.md#L247-L252) (`board.rs`, `cards.rs`)
* **Discrepancy Description**: Omission of concrete data structures (`Tile`, `TileKind`, `Card`, `CardEffect`), rent escalation formulas, and the static 40-tile and card definitions.
* **Rationale**: Forces the implementer to invent domain rules and values, violating the Zero-Decision Rule.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Define the exact Rust structs/enums and provide the 40-tile Saigon catalog (streets, transit, utilities, taxes) and card decks in the document.
* **Proposed Edit**: Add `Tile`, `TileKind`, `District`, `Card`, `CardEffect`, and complete tile definition tables in `board.rs` and `cards.rs` specifications.

### 3. Incomplete Wire Protocol for Stated Game Mechanics [`egm-implementation-readiness`, `egm-codebase-details`] - Severity: Blocker
* **Affected Scope**: [Lines 232–235](docs/designs/vietnamese_monopoly_saigon.md#L232-L235) (`crates/monopoly-proto/src/lib.rs`)
* **Discrepancy Description**: Stated mechanics (auctions, unmortgaging, selling houses, bankruptcy declaration) lack wire protocol messages.
* **Rationale**: Implementer cannot connect state transitions to client/server messages.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Provide the complete message definitions in `ClientMessage` (`PlaceBid`, `PassAuction`, `SellHouse`, `UnmortgageProperty`, `DeclareBankruptcy`) and `ServerMessage` (`AuctionStarted`, `BidPlaced`, `AuctionConcluded`).
* **Proposed Edit**: Expand `crates/monopoly-proto/src/lib.rs` specifications with all required message variants and payloads.

### 4. Missing Client WebSocket Handshake & Session Reconnection Lifecycle [`egm-codebase-details`] - Severity: Blocker
* **Affected Scope**: [Lines 271–276, 306](docs/designs/vietnamese_monopoly_saigon.md#L271-L276) (`router.rs`, `room.rs`, `ws.rs`)
* **Discrepancy Description**: The exact handshake sequence, token generation, rejection codes, and ping/pong intervals are unspecified.
* **Rationale**: Essential for ensuring server and client network stacks interoperate without assumption gaps.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Specify the connection flow: `POST /api/rooms` creates room, returns `room_id` and host `session_token`; WS connects with query parameter; initial `JoinRoom` message; `RoomJoined` response; 10s ping / 30s timeout heartbeat.
* **Proposed Edit**: Add explicit WebSocket handshake lifecycle and sequence in `router.rs` and `ws.rs`.

### 5. Undefined REST API Contracts and Room Manager Actor Concurrency [`egm-implementation-readiness`] - Severity: Major
* **Affected Scope**: [Lines 271–276](docs/designs/vietnamese_monopoly_saigon.md#L271-L276) (`router.rs`, `room.rs`), [Lines 38–42](docs/designs/vietnamese_monopoly_saigon.md#L38-L42) (Open Questions)
* **Discrepancy Description**: `/api/rooms` lacks request/response JSON types, room actor lacks channel message definition, and Open Question 1 remains unanswered.
* **Rationale**: Concurrency architecture and room lifecycle must be pre-determined.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Provide the Axum handler signatures, JSON schemas, `RoomCommand` enum, and resolve Open Question 1 (rooms terminate after 5 minutes with zero active human players).
* **Proposed Edit**: Add request/response types in `router.rs`, define `RoomCommand` in `room.rs`, and update Open Question 1 to Resolved.

### 6. Ambiguous Disconnect vs. Turn Timeout Arbitration Logic [`egm-implementation-readiness`] - Severity: Major
* **Affected Scope**: [Lines 277–282](docs/designs/vietnamese_monopoly_saigon.md#L277-L282) (`timer.rs`, `bot.rs`)
* **Discrepancy Description**: Undefined precedence between 30s turn timer and 15s disconnect grace timer.
* **Rationale**: Prevents race conditions and desynchronization in bot takeover.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Specify `tokio::select!` arbitration: socket disconnect starts 15s timer; bot triggers on whichever expires first; reconnect cancels grace timer; disconnected players take turns with 3s simulation delay.
* **Proposed Edit**: Add timer arbitration specification in `timer.rs` and `bot.rs`.

### 7. Unspecified SVG Rendering Mode, Asset Dependencies, and Coordinate Mapping [`egm-implementation-readiness`, `egm-codebase-details`] - Severity: Major
* **Affected Scope**: [Lines 307–309](docs/designs/vietnamese_monopoly_saigon.md#L307-L309) (`board.rs`), [Lines 325–334](docs/designs/vietnamese_monopoly_saigon.md#L325-L334)
* **Discrepancy Description**: Missing 1000x1000 coordinate mapping formula and dependency on bespoke vector artwork for 40 landmarks.
* **Rationale**: Implementer must not be blocked by graphic design dependencies.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Define inline SVG rendering in Leptos, the coordinate calculation formula for perimeter tiles, and procedural geometric fallback styling.
* **Proposed Edit**: Update `board.rs` with coordinate mapping formulas and procedural fallback rendering.

### 8. Test Plan Lacks Specific Test Cases, Assertion Matrices, and Test Harness Helpers [`egm-implementation-readiness`, `egm-codebase-details`] - Severity: Major
* **Affected Scope**: [Lines 345–355](docs/designs/vietnamese_monopoly_saigon.md#L345-L355) (`Verification Plan`)
* **Discrepancy Description**: Categories listed instead of specific test cases; missing time manipulation (`tokio::time::pause()`) and deterministic RNG seed helpers.
* **Rationale**: Necessary for automated regression testing and reliable CI.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Detail concrete test case names, test fixture helpers (`deterministic_rng(seed)`, `create_test_game()`), and time-warping test helpers.
* **Proposed Edit**: Expand `Verification Plan` with comprehensive test tables and test infrastructure details.

### 9. Money Value Object Scale Multipliers and String Formatting [`egm-implementation-readiness`] - Severity: Nit
* **Affected Scope**: [Lines 244–246](docs/designs/vietnamese_monopoly_saigon.md#L244-L246) (`money.rs`)
* **Discrepancy Description**: Ambiguity in unit scaling and display format for negative values.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Specify `Money::from_vnd`, `Money::as_vnd`, and `Display` implementation formatting.
* **Proposed Edit**: Add API details and formatting example to `money.rs`.

### 10. Missing Build/CI Pipeline Integration for SVG Sanitizer Script [`egm-codebase-details`] - Severity: Nit
* **Affected Scope**: [Lines 331–333](docs/designs/vietnamese_monopoly_saigon.md#L331-L333), [Lines 292–294](docs/designs/vietnamese_monopoly_saigon.md#L292-L294)
* **Discrepancy Description**: Execution hook for `sanitize_svg.py` is omitted from `Trunk.toml` and CI.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Attach `sanitize_svg.py` to Trunk pre-build hook and CI check.
* **Proposed Edit**: Update `Trunk.toml` and `ci.yml` with hook definitions.

### 11. Editorial Phrasing: Hyperbole, Unverifiable Claims, and Non-Technical Meta-tags [`egm-style`] - Severity: Nit
* **Affected Scope**: Line 211, Line 261, Line 297, Line 309, Lines 366–368
* **Discrepancy Description**: Subjective marketing qualifiers ("exhaustive", "authentic") and non-technical tool provenance footers.
* **AI Suggestion**: PROPOSE FIX
* **Justification**: Ensure objective, professional engineering tone.
* **Proposed Edit**: Strip subjective adjectives, tighten introductory text, and remove the provenance comment block.

# Vietnamese Monopoly ("Cờ Tỷ Phú Sài Gòn") Full-Stack Rust Web Application

* **Author:** Engineering Team
* **Date:** 2026-10-30

## Overview

A lightweight, cross-platform multiplayer web implementation of the classic Vietnamese Saigon-edition Monopoly ("Cờ Tỷ Phú Sài Gòn"). The system uses an end-to-end Rust architecture: an authoritative asynchronous backend (`axum` + WebSockets on `tokio`) paired with a lightweight WebAssembly frontend (`leptos` / DOM + SVG). It features authentic Saigon street names and landmarks, custom vector assets created via an Inkscape/SVG pipeline, responsive views for both desktop and mobile screens, a 30-second turn enforcement timer, and an automated "Xe Ôm" heuristic bot that steps in whenever players disconnect or time out.

---

## Analysis

The multi-lens evaluation identified key technical, structural, and cultural design points:

### 1. Codebase Architecture & Crate Topology
- **`monopoly-core`**: A deterministic, headless state machine crate compiled to both native Rust (for server authority) and WebAssembly (for client-side optimistic UI validation). Implements all game rules, 40-tile Saigon board data, card deck logic, cash calculations using 64-bit integers (`i64` in thousands of VNĐ to prevent overflow and floating-point errors), and seeded PRNG (`rand_chacha`).
- **`monopoly-proto`**: Shared Serde message contracts defining client-to-server actions (`RollDice`, `BuyProperty`, `BuildHouse`, `PlaceBid`, `EndTurn`) and server-to-client events (`DiceRolled`, `TurnChanged`, `PropertyAcquired`, `RentPaid`, `TimerStarted`, `CardDrawn`, `SyncSnapshot`).
- **`monopoly-server`**: Axum HTTP and WebSocket server managing game rooms, heartbeat ping-pong, room timers, and session tokens.
- **`monopoly-client`**: Leptos single-page application compiling to WebAssembly. Renders an interactive, responsive vector board using SVG and communicates with the backend via WebSocket.

### 2. Saigon Street Theming & Cultural Adaptation
- **District Color Bands**:
  - *Brown (Bình Dân - Q4):* Bến Vân Đồn, Tôn Đản.
  - *Cyan (Chợ Lớn / Di Sản - Q5):* Hải Thượng Lãn Ông, Châu Văn Liêm, Triệu Quang Phục.
  - *Magenta (Ẩm Thực / Sôi Động - Phú Nhuận):* Vạn Kiếp, Phan Xích Long.
  - *Orange (Học Xá / Thanh Niên - Q5/Q10):* An Dương Vương, Nguyễn Tri Phương, Hùng Vương.
  - *Red (Trung Tâm Đời Sống - Q1/Q3):* Hai Bà Trưng, Đinh Tiên Hoàng, Cách Mạng Tháng 8.
  - *Yellow (Phố Tài Chính - Q1):* Hàm Nghi, Nam Kỳ Khởi Nghĩa, Nguyễn Công Trứ.
  - *Green (Khu Di Sản / Biệt Thự - Q1):* Lê Duẩn, Pasteur, Đồng Khởi.
  - *Dark Blue (Đất Vàng Đắt Đỏ Nhất - Q1):* Nguyễn Huệ, Công Trường Lam Sơn / Bitexco.
- **Public Transport & Utilities**:
  - *Stations:* Ga Sài Gòn, Bến Xe Miền Đông, Bến Xe Miền Tây, Tuyến Metro Bến Thành - Suối Tiên.
  - *Utilities:* Tổng Công Ty Cấp Nước Sài Gòn (Sawaco), Tổng Công Ty Điện Lực TP.HCM (EVN HCMC).
- **Corner Squares**:
  - *Xuất Phát (GO):* Lĩnh 2.000.000 đ trợ cấp.
  - *Vào Tù / Thăm Tù:* Đồn Công An (Giam xe / Vi phạm giao thông).
  - *Nghỉ Chân (Free Parking):* Cà Phê Bệt Nhà Thờ Đức Bà / Phố Đi Bộ.
  - *Vào Tù (Go to Jail):* Bị phạt vi phạm giao thông / Đua xe.
- **Cơ Hội & Khí Vận Cards**:
  - Culturally resonant events: "Ngập nước đường Nguyễn Hữu Cảnh: nộp 200.000 đ sửa xe bu-gi", "Trúng độc đắc vé số kiến thiết: nhận 5.000.000 đ", "Kẹt xe ngã tư Hàng Xanh: mất 1 lượt".

### 3. Responsive Board Ergonomics
- **Desktop / Tablet Landscape:** Full 40-tile square board layout centered on the screen, flanked by player status widgets, event logs, and card hand drawers.
- **Mobile Portrait:** An adaptive dual-mode interface. A top minimap provides whole-board orientation, while the main view zooms into the active quadrant rail to ensure clear touch targets for dice rolling, buying, and mortgaging.

### 4. Vector Asset Pipeline (Inkscape + SVGO)
- Source vector illustrations authored in Inkscape with strict group and layer IDs (`#tile_00` to `#tile_39`, `#pawn_player_1` to `#4`, `#house_1` to `#4`, `#hotel`).
- A build sanitation script removes Inkscape/Sodipodi metadata and unneeded XML tags.
- Animated elements (pawns, building placements) use hardware-accelerated CSS `translate3d` transforms rather than continuous SVG DOM re-renders.

### 5. Session Resilience & Heuristic Bot Takeover
- **Turn Enforcement:** Strict 30-second server countdown timer per player turn.
- **Automated "Xe Ôm" Bot:**
  - Soft timeout: If 30 seconds elapse without action, the bot executes a mandatory roll or turn pass.
  - Disconnect timeout: If the WebSocket disconnects for >15 seconds, the server assigns the seat to a heuristic bot (evaluating cash reserves, color set completions, and mortgage decisions) to prevent stalling other players.
  - Reconnection: The player's browser holds a `session_token` in `localStorage`, allowing instant reclamation of the seat upon reconnecting.

---

## Goals vs. Non-Goals

* **In-Scope Goals:**
  - Deterministic shared `monopoly-core` crate shared across client and server.
  - 40-tile Saigon street edition board layout, custom cards, and localized sound effects.
  - Responsive SVG board interface supporting both desktop and mobile portrait screens.
  - WebSocket multiplayer with room invite links (e.g., `?room=SGN-888`).
  - Authoritative 30s turn timer with automated heuristic bot fallback and session reconnection.
  - Sanitized SVG vector asset pipeline.
* **Out-of-Scope Non-Goals:**
  - Heavy 3D WebGL physics engine (Bevy/Three.js) in v1.
  - Native mobile app store releases (iOS App Store / Google Play binary wrapping) for the initial MVP; web browser delivery first.
  - Real-money gambling or microtransactions.

---

## Alternatives Weighed

* **Leptos (DOM + SVG) vs. Bevy (Canvas / WebGL):** Bevy delivers richer 3D graphics, but produces a 15-30 MB WASM download, drains mobile battery, and renders blurry text on high-DPI screens. Leptos with SVG produces a sub-500KB bundle that boots quickly on mobile 4G networks and preserves clean vector graphics.
* **Full-State Broadcast vs. Event Sourcing:** Sending complete game state trees on every turn wastes bandwidth on mobile connections. An event-stream model (`DiceRolled`, `PropertyAcquired`, etc.) replayed against the shared deterministic core reduces WebSocket payload size.
* **Immediate Player Forfeit vs. Bot Takeover:** Traditional turn-based web games break when a player leaves. Substituting an automated heuristic bot keeps the game moving for the remaining participants.

---

## Key Decisions & Staging for Next Steps

1. **Monorepo Layout:** Standard Cargo workspace with crates: `monopoly-core`, `monopoly-proto`, `monopoly-server`, and `monopoly-client`.
2. **Board & Lore:** Authentic Saigon street set partitioned into 8 color districts, 4 transit hubs, and 2 municipal utilities.
3. **Session Protocol:** 30s turn countdown with reconnection token and heuristic bot takeover.
4. **Asset Workflow:** Inkscape vector source files converted to clean SVGs and bound to Leptos signals.

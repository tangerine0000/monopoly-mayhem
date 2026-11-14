# Domain Glossary

This glossary defines the canonical module and domain terms for `monopoly-mayhem`.

- **BoardCatalog**: Static, zero-allocation catalog of all 40 board tiles, districts, and property/station/utility/tax specifications.
- **PlayerView**: Typed, secret-free projection of `GameState` (`PublicGameView`, `PublicPlayerView`) sent across the protocol seam without `session_token` or hidden card deck order.
- **RulesEngine**: Deep simulation module in `monopoly-core` that owns action validation, rent/building/mortgage queries, turn transitions, and card resolution.
- **BotPolicy**: Deterministic AI decision module in `monopoly-core` shared by both `monopoly-server` and solo `monopoly-client`.
- **LocaleCatalog**: Multi-language resolution module in `monopoly-client` with `Vi` and `En` adapters mapping `TileId`, `CardId`, `DeckId`, and `MessageKey` to localized copy.
- **RoomActor**: Single-loop authoritative room coordinator in `monopoly-server` that owns turn/disconnect deadlines and single-pass `Arc<str>` broadcasts.
- **Clock**: Time seam (`TokioClock` in production, `ManualClock` in tests) backing `RoomActor` deadlines.
- **LobbyRegistry**: Room lifecycle module in `monopoly-server` owning collision-safe room code allocation, lookup, and idle room reaping.
- **ClientStore**: Deep reactive state module in `monopoly-client` exposing fine-grained tile/player selectors and a bounded `LogRing`.
- **AnimationQueue**: Generation-scoped timer coordinator inside `ClientStore` that cancels in-flight dice and pawn animations on room exit or reset.

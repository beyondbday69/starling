# Task 5: Complete Adapter & Sync Layer (Leaf Variants, 2×2 Cut Tracking, Dimension Scoping, Client Sync, Config & FX)

## Goal
Close the remaining gaps identified in Task 4 so the entire mod — both pure-Rust state/sync/config/FX planning and the `horizon-sdk`-gated `hooks.rs` adapter — is feature-complete for singleplayer and LAN play (with all pure-Rust additions unit-tested and all live-game behaviour clearly marked **UNTESTED** until Horizon can be built/run).

## Files to Create / Modify
- `native-mods/timber_fall/src/config.rs`
  - Add a pure-Rust `key = value` (TOML-like / `.cfg`) parser & serializer (`TimberConfig::parse(text: &str) -> Self` and `to_config_string(&self) -> String`) using `std` only so server/client config files load cleanly with fallback to defaults on missing/invalid keys.
- `native-mods/timber_fall/src/scanner.rs` & `src/falling.rs`
  - Preserve each leaf's own `variant: u16` (instead of `0` / log variant) on `ScannedTree` / `FallingBlock` so `hooks.rs` can resolve the exact leaf block model and leaf loot (sapling/stick/apple) per block.
- `native-mods/timber_fall/src/runtime.rs`
  - Add `dimension_id: u64` (opaque level key) to `ActiveTreeEntry` and filter `tick_dimension(dimension_id, dt, grid)` so multi-dimension servers tick each tree only in its own level.
  - Add `TwoByTwoCutTracker`: tracks recent partial 2×2 base log cuts `(dimension_id, BlockPos, tick_time)` with expiry (e.g. 200 ticks) so cutting all 4 logs of a 2×2 trunk in sequence triggers `FellTree` cleanly.
  - Add pure-Rust client/server sync snapshot types (`SyncFallingTree`, `SyncPhase`, `SyncMessage`: `StartFall`, `UpdatePhase`, `RemoveTree`) + `TimberClientMirror` (receives sync messages and interpolates client-side `angle_rad` between server ticks for smooth rendering in `collect_render_instances`).
  - Add pure-Rust FX event descriptors in `TickOutputs` (`CreakSound`, `LeafBurstParticles`, `ImpactDustAndThud`) so `hooks.rs` has clean triggers for warning creaks, falling leaf trails, and impact dust/thud.
- `native-mods/timber_fall/src/hooks.rs` (`#[cfg(feature = "horizon-sdk")]`)
  - Verify any newly used `mod_api` symbols against `crates/horizon-mod-vocab` / `target/debug/build/horizon-mod-vocab-*/out/` (never invent API names).
  - Wire:
    - `dimension_id` scoping in `level_tick_post` and `TwoByTwoCutTracker` in `break_block`.
    - Exact leaf `variant` -> model name (`minecraft:block/<leaf_name>`) and leaf loot evaluation (sapling, stick, apple drops) on `BreakUp`.
    - Client-side `TimberClientMirror` updates in `level_tick_post` / `submit_custom_geometry` (or singleplayer/LAN sync via `mod_api` payload/network if available, or documented cleanly if `custom_payload` requires registration).
    - Config loading from file (or defaults) and sound/particle emission on `entered_falling` / `impacts` / `completed` using verified `mod_api` calls.
    - Keep `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` compiling cleanly with 0 warnings without needing `horizon-sdk`.

## Constraints
- Pure Rust (`std` only) for all non-`horizon-sdk` modules; 0 `unwrap()` in library logic.
- Do not OOM this 1.9 GiB VPS running `cargo check --features horizon-sdk` if it pulls in the full `horizon-mod-vocab` build; verify `mod_api` symbols by reading `crates/horizon-mod-vocab` / generated files instead, and mark `horizon-sdk` + in-game behaviour as **UNTESTED**.
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).
- Write a concise summary, verified API list, and exact test output to `tasks/REPORT.md` when done.

## Acceptance Criteria
1. `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` compiles with 0 warnings and passes all tests (Tasks 1–5).
2. Unit tests verify:
   - `TimberConfig::parse` & `to_config_string` round-trip and partial/invalid-line fallback to defaults.
   - Leaves preserve their own `variant` through `scanner` -> `falling` -> `runtime` -> `drops`.
   - `tick_dimension` ticks only trees belonging to the matching `dimension_id`.
   - `TwoByTwoCutTracker` rejects after 1–3 cuts, triggers `FellTree` on the 4th cut, and expires stale cuts.
   - `TimberClientMirror` applies `StartFall` / `UpdatePhase` / `RemoveTree` and produces rotated `RenderBlockInstance`s for both logs and leaves.
   - `TickOutputs` emits creak/whoosh/dust/thud FX events at the right phase transitions.
3. `hooks.rs` cleanly wires the above using only verified `mod_api` symbols.

## Test Command
```sh
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

# Fix Task: Add Wildcard Arm to `#[non_exhaustive] HudMode` Match in `zoom/key.rs` (`E0004`)

## Root Cause (CI Run `46007642618`)
```
error[E0004]: non-exhaustive patterns: `_` not covered
   --> crates/horizon-client/src/game/starling/native_compat.rs:343:31
note: `mod_vocab::HudMode` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively
```
- `horizon-client` compiled cleanly all the way through `zoom` except for that single `match game.hud_mode()` in `native-mods/zoom/mixins/zoom/key.rs`!

## Fix
- In `native-mods/zoom/mixins/zoom/key.rs`, add `_ => zoom::Hud::AllVisible,` to `match game.hud_mode() { ... }`.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/zoom/Cargo.toml`, clean `native-mods/zoom/target/`, and append to `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/zoom/Cargo.toml --check
cargo clippy --manifest-path native-mods/zoom/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/zoom/Cargo.toml
```

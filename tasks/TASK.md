# Fix Task: Fix `state` Name Shadowing in `mixins/timber_fall/runtime.rs` (`E0618`)

## Goal
Fix the single `horizon-client` compile error in Android APK CI run `45868507333`:
```
error[E0618]: expected function, found `horizon_client_lib::starling_mod_api::BlockStateId`
  --> crates/horizon-client/src/game.rs:19099:25
   |
18920 | fn state() -> std::sync::MutexGuard<'static, State>
...
18998 | state: BlockStateId,
...
19099 | let mut s = state();
```

## Fix
- In `native-mods/timber_fall/mixins/timber_fall/runtime.rs`, rename `fn state()` to `fn lock_state()` (or rename the parameter `state: BlockStateId` to `block_state: BlockStateId`) so the `BlockStateId` parameter never shadows the `MutexGuard` helper function!
- Check the rest of `native-mods/timber_fall/mixins/timber_fall/runtime.rs` for any other `state()` calls and update them consistently.
- Run `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, and update `tasks/REPORT.md`.

## Constraints
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

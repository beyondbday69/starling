# Fix Task: Clear Clippy `-D warnings` Errors in `timber_fall`

## Goal
Fix the two Clippy errors reported by `.github/workflows/timber-fall.yml` (run `45850400023`) so `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cargo test` all pass.

## Errors to Fix
1. `native-mods/timber_fall/src/runtime.rs:149` — `clippy::too_many_arguments` (8/7) on `TimberWorldRuntime::try_start_fall`:
   - Group inputs cleanly (e.g. a `StartFallRequest<'a>` struct or include `dimension_id: u64` / `player: [f32; 2]` inside `ChopContext` or a request struct, or `#[allow(clippy::too_many_arguments)]` if appropriate — grouping into a small request struct or putting `dimension_id` + `player` in a struct is cleaner). Update callers in `src/hooks.rs` and tests in `src/runtime.rs`.
2. `native-mods/timber_fall/src/scanner.rs:308` — `clippy::filter_map_bool_then`:
   - Replace `(-1..=1).filter_map(move |dx| (dx, dy, dz).ne(&(0, 0, 0)).then(|| BlockPos::new(...)))` with `.filter(move |&dx| (dx, dy, dz) != (0, 0, 0)).map(move |dx| BlockPos::new(...))`.

## Constraints
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).
- Clean up `native-mods/timber_fall/target/` if created.
- Update `tasks/REPORT.md` with the commands and real output.

## Acceptance Criteria
1. `cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check` passes.
2. `cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings` passes with 0 warnings/errors.
3. `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` passes all 80 tests.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

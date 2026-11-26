# Fix Task: Fix Missing Trait Imports (and Any Remaining SDK Type Errors) in `hooks.rs`

## Goal
Fix the `Type-check Horizon SDK adapter` failure in `.github/workflows/timber-fall.yml` (run `45851931101`: `cargo check --manifest-path native-mods/timber_fall/Cargo.toml --features horizon-sdk`).

## Errors from CI Run `45851931101`
1. `native-mods/timber_fall/src/hooks.rs:69`:
   `error[E0405]: cannot find trait Hooks in this scope`
   -> Add `Hooks` to `use mod_api::{...};`.
2. `native-mods/timber_fall/src/hooks.rs:532`:
   `error[E0405]: cannot find trait BlockReader in this scope`
   -> Add `BlockReader` to `use crate::scanner::{...};`.

## Important: Check for Any Subsequent `horizon-sdk` Type Errors
Because `E0405` stopped `rustc` early before full type-checking of `hooks.rs`, please also carefully audit all `mod_api` calls in `native-mods/timber_fall/src/hooks.rs` against `crates/horizon-mod-vocab` / `target/debug/build/horizon-mod-vocab-*/out/` (method names, argument types, `&dyn Host` vs `&mut dyn ServerHost` vs `&dyn ClientHost`, return types, `#[mod_api::horizon_mod]` macro requirements) so the next CI `cargo check --features horizon-sdk` passes cleanly!

## Constraints
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).
- Clean up `native-mods/timber_fall/target/` if created.
- Update `tasks/REPORT.md` with the changes and real output of `fmt --check`, `clippy`, and `test`.

## Acceptance Criteria
1. `cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check` passes.
2. `cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings` passes.
3. `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` passes all 80 tests.
4. `hooks.rs` has `Hooks` and `BlockReader` imported and all `mod_api` calls audited against `crates/horizon-mod-vocab`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

# Fix Task: Fix Dead `native_tick` Hook (`Game<'_>::tick`), `timber_fall_hits` Center Offset, and Unique APK Artifact Zip Name

## Root Cause of "tree stops right there in mid air"
- `crates/horizon-client/src/game/starling/native_compat.rs:78` defines `native_tick(&mut self)`, **but nothing in `crates/horizon-client/src/` ever calls `native_tick()`!**
- Because `mixins/timber_fall/horizon-mixin.toml` injected `timber_fall_tick` into `native_tick`, `timber_fall_tick()` **never ran**, so `tree.elapsed_secs` stayed `0.0` forever and the tree stayed frozen at `0°` in mid-air!

## Fixes
1. **Inject `timber_fall_tick` into `horizon_client::game::Game<'_>::tick` (`crates/horizon-client/src/game.rs:4612`):**
   - Change `[[injection]] id = "tick"` target in `native-mods/timber_fall/mixins/timber_fall/horizon-mixin.toml` from `horizon_client::game::native_hooks::Game<'_>::native_tick` to `horizon_client::game::Game<'_>::tick` (`kind = "HEAD"`, `bindings = [{ name = "game", ty = "&mut crate::game::Game<'_>", value = "&mut *self" }]`).
   - Verify with `packaging/android/native_mod_adapt.py` that `horizon_client::game::Game<'_>::tick` relocates cleanly without error.
   - Note: `Game::tick` also runs when `self.remote.is_some()` (returns early at line 4637), and `timber_fall_tick` already starts with `if self.remote.is_some() { return; }`, which is safe! Also check `if !running` (paused/frozen) if needed, or let `timber_fall_tick` run on server ticks.
2. **Fix `timber_fall_hits` center offset in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`:**
   - In `timber_fall_hits` (lines 666–668), use the block **center** `(log.pos.x as f64 + 0.5 - hinge[0], log.pos.y as f64 + 0.5 - hinge[1], log.pos.z as f64 + 0.5 - hinge[2])` instead of the corner `log.pos.x - hinge[0]` / `log.pos.z - hinge[2]`, so ground collision checks match `rotate_point(tree.cut, tree.toward, block_center(log.pos), angle)`.
3. **Update `.github/workflows/android.yml` artifact zip name:**
   - Change `name: starling-apk-${{ matrix.abi }}` (around line 150) to `name: starling-apk-${{ matrix.abi }}-r${{ github.run_number }}` so every build produces a uniquely named artifact zip.

## Constraints
- Do not modify any files outside `native-mods/timber_fall/`, `.github/workflows/android.yml`, and `tasks/REPORT.md`.
- Clean up `native-mods/timber_fall/target/` if created.
- Run `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

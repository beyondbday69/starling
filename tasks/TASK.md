# Task 6: Direct Android APK Integration (`horizon-mixins` Bundle + `android.yml`)

## Goal
Package `timber_fall` as a `horizon-mixins` bundle under `native-mods/timber_fall/mixins/timber_fall/` so `packaging/android/native_build.sh` and `.github/workflows/android.yml` compile Timber Fall **directly into `libhorizon_android.so` inside the APK** (without needing `.hmod`).

## Reference Files to Inspect First
- `native-mods/fallingtrees/mixins/fallingtrees/horizon-mixin.toml` and its helper `.rs` files (`destroy_block.rs`, `tick.rs`, `list.rs`, `settings.rs`, `click.rs`, `runtime.rs`) — shows the exact `horizon-mixins` bundle layout, `[compatibility]`, `[[module]]`, and `[[injection]]` syntax that `packaging/android/native_build.sh` expects.
- `crates/horizon-client/src/game/starling/native_compat.rs` — exact signatures for `native_destroy_block`, `native_tick`, `native_mods`, `native_settings`, `native_settings_click`, `native_block`, `native_block_collision`, `native_play_local_sound`, etc.
- `crates/horizon-client/src/game/modding/native_custom_geometry.rs` (around line 89) — exact signature of `native_submit_custom_geometry` so the rotating multi-block `ModelInstance` render hook can be injected directly.
- `.github/workflows/android.yml` (lines 33 & 127) — default `MODS` list (`cinematic_interactions skins_3d gyro fallingtrees distant_horizons`).

## Files to Create / Modify
1. `native-mods/timber_fall/mixins/timber_fall/horizon-mixin.toml`
   - `[mod] id = "timber_fall"`, `version = "0.1.0"`, `priority = 100`.
   - `[compatibility]` matching `fallingtrees`.
   - `[[module]]` injecting the Timber Fall module into `crates/horizon-client/src/game.rs` (`source = "mod_bundle.rs"` or `"runtime.rs"` — note: `horizon-mixins` copies the files from `mixins/timber_fall/` into the game tree, so either keep `mixins/timber_fall/` self-contained or use `include!` / clean shared pure-Rust logic without symlinks, since `native_build.sh` deletes symlinks!).
   - `[[injection]]` hooks:
     - `destroy_block` (`HEAD` at `horizon_client::game::native_hooks::Game<'_>::native_destroy_block` or `horizon_client::game::starling::native_compat::Game<'_>::native_destroy_block` — **verify** how `fallingtrees` and `native_mod_adapt.py` rewrite `native_hooks::Game<'_>::...`!).
     - `tick` (`HEAD` at `native_tick`)
     - `submit_custom_geometry` (`HEAD` at `native_submit_custom_geometry` if supported by `native_mod_adapt.py` / `horizon-mixins`, to render rotating `ModelInstance`s for active falling trees!)
     - `list`, `settings`, `click` (`native_mods`, `native_settings`, `native_settings_click`) so Timber Fall appears in Options > Mods with its config toggles.
2. `native-mods/timber_fall/mixins/timber_fall/*.rs`
   - Helper functions called by each injection + the injected module implementing Timber Fall's tree scan, physics, collision, drops, and rotating multi-block render using `Game<'_>` methods verified in `crates/horizon-client/src/game/` (check `fallingtrees/mixins/fallingtrees/runtime.rs` to see how `Game<'_>` block reads/writes/item spawns/tool damage work!).
3. `.github/workflows/android.yml`
   - Replace `fallingtrees` with `timber_fall` in the default `MODS` list (lines 33 and 127) so the APK builds with Timber Fall directly compiled in.

## Constraints
- Verify `packaging/android/native_mod_adapt.py` and `crates/horizon-client/src/game/starling/native_compat.rs` to make sure every target path and `Game<'_>` method exists and compiles on Android (`aarch64-linux-android`).
- No symlinks (deleted by `native_build.sh`).
- Keep `cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check`, `cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings`, and `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` passing.
- Update `tasks/REPORT.md` with the changes, verified targets, and test output.

## Acceptance Criteria
1. `native-mods/timber_fall/mixins/timber_fall/horizon-mixin.toml` and its `.rs` files exist and are self-contained (no symlinks).
2. `.github/workflows/android.yml` includes `timber_fall` in default `MODS`.
3. `fmt --check`, `clippy --all-targets -- -D warnings`, and all `timber_fall` unit tests pass.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

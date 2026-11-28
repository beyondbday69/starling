# Task: Port Stump-Drop-Then-Tip Animation (`timber_fall-mod2.zip`) into Android Mixin (`mixins/timber_fall/runtime.rs`)

## User Request
> "yeah i want" (port the new stump-drop-then-tip animation from `/root/starling/timber_fall-mod2.zip` into the Android APK's `native-mods/timber_fall/mixins/timber_fall/runtime.rs`).

## Reference
Inspect `/root/starling/timber_fall-mod2.zip` (`src/falling.rs` and `src/runtime.rs`) to see the new `Dropping` phase:
1. When the cut log is removed, the rest of the tree hovers 1 block above the stump.
2. First it holds straight (`0°`) briefly, then drops **straight down by 1.0 block onto the stump under gravity** (`-(0.5 * g * t^2).min(1.0)`).
3. On landing on the stump, it gives a subtle landing camera shake (`0.4x`), and **only then** begins tipping over!

## Changes
1. **Update `native-mods/timber_fall/mixins/timber_fall/runtime.rs`:**
   - Add the initial stump-drop phase before the tip-over timeline (`DURATION = 3.0s`):
     - `HOLD_SECS = 0.25` (stands straight at `0°` right after the cut, `stump_shift = 0.0`).
     - `STUMP_DROP_HEIGHT = 1.0`, `STUMP_DROP_SECS = (2.0 * STUMP_DROP_HEIGHT / GRAVITY_FALL).sqrt()` (~`0.378 s` at `GRAVITY_FALL = 14.0`): drops straight down (`angle = 0.0`, `stump_shift = -(0.5 * GRAVITY_FALL * t * t).min(1.0)`).
     - At `HOLD_SECS + STUMP_DROP_SECS`: lands on the stump (triggering a subtle `0.4x` landing camera shake in `timber_fall_camera_shake` for `0.25 s` after landing!).
     - After landing (`smooth_secs >= HOLD_SECS + STUMP_DROP_SECS`): `p = ((smooth_secs - (HOLD_SECS + STUMP_DROP_SECS)) / DURATION).clamp(0.0, 1.0)` runs the existing smooth `voxel-tree-fall.html` tip-over (`0..IMPACT`), 3-hop bounce (`IMPACT..BREAK`), sequential base-to-tip log pop (`BREAK..LAST`), independent leaf fade (no leaf drops), and `drop_y` terrain gravity!
   - Apply `stump_shift` cleanly to:
     - `hinge.y` / `to_camera` in `timber_fall_render` (and detached leaves' `ground` / `center`),
     - `timber_fall_hits` / `timber_fall_hits_at` (so collision uses `stump_shift` + `drop_y`),
     - `landing[1]` in `timber_fall_tick` (so spawned log items land on the ground/stump accurately).
   - Keep `p >= 1.0` completion check based on `elapsed_secs >= HOLD_SECS + STUMP_DROP_SECS + DURATION`.
   - Keep all earlier user preferences intact: **no leaf item drops**, **no white pop flashes**, **pass through other trees**, **smooth 60/120 FPS `Instant` interpolation**, and **sound OFF**.
2. **Sync `src/` from `timber_fall-mod2.zip` if helpful** (keep `Cargo.toml` / `Cargo.lock` clean and all `cargo test` tests passing; do NOT reintroduce any removed dependencies).

## Constraints
- Keep `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` passing on `native-mods/timber_fall/Cargo.toml` with 0 warnings.
- No inner `#![...]` or `//!` file attributes and no symlinks in `mixins/timber_fall/`.
- Clean up `native-mods/timber_fall/target/` and append a concise report to `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

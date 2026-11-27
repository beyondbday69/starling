# Task 7: Fix Tree Removal/Stuck Animation & Implement `voxel-tree-fall.html` Animation

## User Report
> "man tree is still there after mining it and animation stuck, add animation like `/root/starling/voxel-tree-fall.html` (sound off, use default settings) and add in our game"

## Step 1: Fix Why the Tree Stayed There & Animation Got Stuck
Inspect `crates/horizon-client/src/game.rs` (around line 4155), `crates/horizon-client/src/game/blocks/block_breaking.rs`, `crates/horizon-client/src/game/fallingtrees.rs`, and `with_level` (`game.rs:2881`):
1. **Time unit bug in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`:**
   - `self.world.game_time` is in **ticks** (20 ticks/sec), NOT seconds! Comparing `now - since >= WARNING_SECS (0.5)` / `BREAKUP_SECS (1.0)` against `game_time` caused phases to expire in 0.5–1 tick (0.025–0.05s) or desync with `partial`!
   - **Fix:** Track `elapsed_secs: f64` directly on each `ActiveTree`, advancing `elapsed_secs += 0.05` in `timber_fall_tick()` and rendering at `elapsed_secs + partial * 0.05` in `timber_fall_render(...)`!
2. **Why blocks stayed there after mining:**
   - Check why `self.with_level(|b, level, ticks| b.set_block(level, ticks, at, left, 3))` + `*canceled = true` left the tree visible/present in-game!
   - Compare with `fallingtrees.rs` (which does NOT set `*canceled = true` and uses `setblock` / `set_block` cleanly) and check what `with_level` -> `set_block(..., 3)` does on the client chunk mesh (`mark_chunk_dirty` / `remesh` / `set_block_clients`). If `*canceled = true` causes `block_breaking` to restore the mined block or skip mesh updates, either let the cut block break normally (`*canceled = false`, excluding `pos` from `blocks` or spawning only the remaining logs' drops) OR ensure `set_block` + chunk remesh clears every log and leaf immediately!
   - Also check `fallingtrees_destroy_block` in `native_compat.rs:290`: make sure `fallingtrees` does NOT double-handle the tree (e.g. if `timber_fall_destroy_block` already cleared all logs/leaves, `fallingtrees` sees no tree and returns early).

## Step 2: Implement the Exact Animation from `/root/starling/voxel-tree-fall.html`
Use the default settings from `<script id="cfg">` in `/root/starling/voxel-tree-fall.html` with **Sound OFF**:
- **Defaults:** `duration = 3.0` s, `gravity = 14.0`, `bounce = 5.0°`, `overshoot = 0.0°`, linear speed curve (`1.0x`), **sound OFF** (no `play_unshared` audio spam).
- **Timeline (`progress = (elapsed_secs / 3.0).clamp(0.0, 1.0)`):**
  - Constants: `IMPACT = 0.50`, `BREAK = 0.66`, `LAST = 0.80`.
  - **Fall (`0.0 <= p < IMPACT`):**
    - Precompute/evaluate the `SHAPE` curve from `voxel-tree-fall.html` (`th = 0.08`, `w = 0.0`, `while th < PI/2 { w += th.sin() * 0.002; th += w * 0.002; }`, normalized `0.0..=1.0`), mapped over `p / IMPACT` -> `angle = shape_val * PI/2` (and stop at solid ground if terrain collision occurs earlier!).
  - **Bounce (`IMPACT <= p < BREAK`):**
    - `HOPS = [(0.0, 0.54, 1.0), (0.54, 0.30, 0.3), (0.84, 0.16, 0.09)]`, `bounce_deg = 5.0`:
    - 3 parabolic hops: `angle = impact_angle - 4.0 * u * (1.0 - u) * amp * 5.0_f64.to_radians()`.
  - **Sequential Log Break-up (`BREAK <= p <= LAST`):**
    - Order logs from base to tip. Each log `i` pops at `pop_at(i) = BREAK + (LAST - BREAK) * i / max(1, logs - 1)`:
    - Before `pop_at(i)`: swell slightly (`scale = 1.0 + 0.35 * w * w` in the last `0.16s` before popping) and remain attached to the fallen trunk.
    - At `pop_at(i)`: hide that log from the falling mesh and spawn its log item drop at its rotated world landing position!
  - **Independent Leaf Break-off & Fall:**
    - For each leaf block, precompute `leaf.at` (the earliest `p < IMPACT` where its rotated world `y` drops to `hinge_y + 0.55`, or `IMPACT`).
    - For `p <= leaf.at`: leaf rotates rigidly with the trunk.
    - For `p > leaf.at`: leaf detaches, falls vertically/outward under `gravity = 14.0` to ground, and shrinks away after landing (`scale` smoothly goes to `0`), spawning its leaf drops when it breaks/finishes!
  - **Completion (`p >= 1.0` or all logs popped & leaves shrunk):**
    - Remove the `ActiveTree`, apply axe wear, and ensure no leftover ghost blocks remain.

## Constraints
- Keep `native-mods/timber_fall/Cargo.toml` (`fmt --check`, `clippy --all-targets -- -D warnings`, `cargo test`) passing with 0 warnings.
- No symlinks in `mixins/timber_fall/`; no inner `#![...]` or `//!` file attributes in `mixins/timber_fall/*.rs`.
- Verify `native_mod_adapt.py` relocation and check all `Game<'_>` calls against `crates/horizon-client/src/`.
- Update `tasks/REPORT.md` with the root cause of the stuck/unremoved tree and the exact test output.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

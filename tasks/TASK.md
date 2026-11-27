# Task: Add Gravity Drop (`drop_y`) So Trees Fall to the Ground Instead of Floating in Mid-Air

## User Request
> "add like when i br3ak the tree itll fell at ground cuz of gravity not just falling from mid air"

## Problem
Currently `ActiveTree` rotates around a fixed `cut[1]` hinge height, so:
- If you chop a log higher up (leaving a stump or floating tree) or on a slope/ledge, the tree tips to 90° and pops **in mid-air** at `cut[1]` instead of falling down to the ground under gravity!

## Fix in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`
1. **Add vertical gravity (`drop_y`, `drop_vel`) to `ActiveTree`:**
   - Fields on `ActiveTree`: `drop_y: f64` (starts `0.0`), `prev_drop_y: f64` (starts `0.0`), `drop_vel: f64` (starts `0.0`).
   - Wait for the tree to clear its cut/stump slightly or apply gravity immediately once `p > 0.05` (or when no solid non-tree block supports the rotated logs below them):
     - Specifically, in `timber_fall_tick`:
       - Save `tree.prev_drop_y = tree.drop_y;`
       - Test downward gravity step: `let next_vel = tree.drop_vel + GRAVITY * TICK_SECS;` (e.g. `GRAVITY = 14.0`) and `let next_drop_y = tree.drop_y - next_vel * TICK_SECS;`.
       - Check `self.timber_fall_hits_at(tree, angle, next_drop_y)` (ignoring all tree blocks via `is_tree_block`, AND ignoring the cut stump block `[tree.cut[0], tree.cut[1] - 1, tree.cut[2]]` once `angle > 12°` or if we want the tree to slide off its stump!).
       - Actually, even cleaner:
         - Check if any log's rotated center at `(angle, next_drop_y)` (testing `y - 0.5` bottom of the log!) hits solid non-tree terrain (`!is_tree_block && !native_block_collision.is_empty()`):
           - If NOT supported by solid terrain (i.e. it's in mid-air above the ground!), apply `tree.drop_vel = next_vel; tree.drop_y = next_drop_y;` (clamped to a sane max drop like `-32.0` blocks).
           - If it hits solid ground: `tree.drop_vel = 0.0;` (and if `p < IMPACT` and `angle > 0.3`, also record `impact_angle` if stopped by terrain!).
         - Note on the stump directly below `cut` (`[cut[0], cut[1] - 1, cut[2]]`): for `p < 0.15` (the first ~0.45s while tipping), the tree is still resting on its stump (`drop_y = 0.0`), and as it tips past `~10°` (`p >= 0.15`), if the player cut 1+ blocks above ground, the trunk tips off the stump and gravity pulls it smoothly all the way down to the ground! (And if you broke a floating log with air below `[cut[0], cut[1] - 1, cut[2]]`, gravity pulls it down immediately from `p > 0.0`!)
2. **Smooth rendering & accurate drop positions:**
   - In `timber_fall_render`:
     - Smoothly interpolate `drop_y` between `prev_drop_y` and `drop_y` using the tick fraction (or `last_tick_at.elapsed().as_secs_f64() / TICK_SECS` clamped `0.0..1.0`) so the vertical gravity fall is just as silky-smooth (60/120 FPS) as the rotation!
     - Offset `hinge.y += smooth_drop_y;` (so `to_camera` moves down with gravity!).
     - Update `ground` for detached leaves to `tree.cut[1] as f64 + smooth_drop_y` (or actual ground below the leaf) and add `smooth_drop_y` to leaf `center[1]`.
   - In `timber_fall_tick`:
     - Add `[0.0, tree.drop_y, 0.0]` to `landing` when spawning log item drops so items spawn on the ground where the logs actually landed!
3. **Allow downhill tip past 90° if needed (or keep 90° + `drop_y`):**
   - With `drop_y` pulling the whole tree onto the ground below, even a tree chopped 3 blocks high or on a cliff drops cleanly onto the ground and pops its logs right on the terrain!

## Constraints
- Keep `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` passing on `native-mods/timber_fall/Cargo.toml` with 0 warnings.
- Clean up `native-mods/timber_fall/target/` and append a concise report to `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

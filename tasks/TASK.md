# Task 8: Smooth Wall-Clock Animation, No Leaf Drops, Pop Flash, and Little Camera Shake

## User Feedback
> "falling animation is choppy not smooth and do not drop leaves as items just disapper them and add flash on poping animantion and little camera shake"

## Fixes in `native-mods/timber_fall/mixins/timber_fall/`
1. **Make the falling animation silky-smooth (60/120 FPS, zero choppiness):**
   - Currently `timber_fall_tick()` adds `0.05` at 20 TPS while `timber_fall_render()` uses `elapsed_secs + partial * 0.05`. Because `Game::tick()` and `partial` can step out of phase at tick boundaries, `p` stutters every 50 ms!
   - **Fix:** Track `started_at: std::time::Instant` (or `last_tick_at: std::time::Instant` + `smooth_secs: f64` + `last_render_at: Option<std::time::Instant>`) on `ActiveTree`:
     - In `timber_fall_render`: advance `smooth_secs` by `now.duration_since(last_render_at).as_secs_f64().clamp(0.0, 0.05)` as long as `last_tick_at.elapsed().as_secs_f64() < 0.25` (so pausing the game freezes the tree cleanly!). Clamp `smooth_secs` within `[0.0, DURATION]`, and compute `p = (smooth_secs / DURATION).clamp(0.0, 1.0)`.
     - That gives monotonic 60/120 FPS animation with zero backwards jumps or 20 TPS stair-stepping!
2. **Do NOT drop leaves as items — just disappear them:**
   - Remove `drops` from `LeafBlock` (no `block_drops` evaluation for leaves/attached blocks, and never push leaf drops into `spawns`).
   - Leaves still break off at `leaf.at`, fall under gravity, and shrink away (`scale -> 0`) visually, then disappear without spawning items.
   - Only logs spawn item drops when they pop!
3. **Add flash on popping animation:**
   - Look at `voxel-tree-fall.html`: in the last `0.16s` before a log pops (`w = (1.0 - (pop - p) * DURATION / 0.16).max(0.0)`), `scale = 1.0 + 0.35 * w * w` AND `emissive = 0.7 * w * w * w`.
   - In `timber_fall_render`, record `let v_start = out.len();` before `self.block_model_vertices(log.state, 42, matrix, packed, true, out);`.
   - When `w > 0.0`, iterate over `&mut out[v_start..]`:
     - Boost `v.light` toward fullbright (`0xF0F0` / `240 | (240 << 8)`) and lerp `v.color` (RGBA8 bytes) toward white (`255, 255, 255`) by `(0.75 * w * w * w).min(1.0)` (preserving alpha)! That creates a crisp white-hot pop flash right before each log bursts into drops.
4. **Add a little camera shake on impact:**
   - Add a `camera_shake` injection (`HEAD` of `horizon_client::game::native_hooks::Game<'_>::native_camera_shake`, bindings: `game: &crate::game::Game<'_>` = `self`, `partial: f32` = `partial`, `shake: &mut crate::mod_api::Shake` = `&mut *shake` — verify `Shake` path in `crates/horizon-client/src/game/native_hooks/base.rs:894` and `native_mod_adapt.py`!).
   - Create `native-mods/timber_fall/mixins/timber_fall/camera_shake.rs` calling `game.timber_fall_camera_shake(shake)`.
   - In `timber_fall_camera_shake(&self, shake: &mut ...)`: for each active tree in the current dimension where `t = (p - IMPACT) * DURATION` is in `0.0..0.35`:
     - `let env = (1.0 - t / 0.35).powi(2);`
     - Subtle shake: `shake.offset[1] += ((t * 75.0).sin() * 0.05 * env) as f32; shake.offset[0] += ((t * 53.0).sin() * 0.025 * env) as f32; shake.roll += ((t * 60.0).sin() * 0.45 * env) as f32;`

## Constraints
- Verify `native_camera_shake` relocation with `packaging/android/native_mod_adapt.py`.
- Keep `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` passing on `native-mods/timber_fall/Cargo.toml` with 0 warnings.
- Clean up `native-mods/timber_fall/target/` and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

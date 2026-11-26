# Fix Task: Render Falling Trees with `block_model_vertices` & Cancel Vanilla Break in `mixins/timber_fall/runtime.rs`

## Root Cause of "no realistic falling tree animation" in APK
1. **Zero blocks rendered:** `timber_fall_model` called `self.native_model("minecraft:block/<name>")`, which ONLY resolves mod-registered `standalone_geometry` models — it returns `Err(ModelError::UnknownModel)` for all vanilla blocks, so `timber_fall_geometry` drew **0 triangles**!
2. **Cut block wasn't canceled:** `timber_fall_destroy_block` left `_canceled` untouched (`false`), so vanilla still broke/dropped the cut log immediately.

## Fix
1. **Replace `native_model` / `native_world_draw_model_instances` with `block_model_vertices`:**
   - Inspect `crates/horizon-client/src/game/entities/text_displays.rs` (`block_item_display_geometry` around line 264) and `block_model_vertices` definition in `crates/horizon-client/src/game/` to get exact types (`glam::DVec3`, `glam::Mat4`, `Vertex`, `light`).
   - Change the render injection in `native-mods/timber_fall/mixins/timber_fall/horizon-mixin.toml` from `native_submit_custom_geometry` to `TAIL` (or `HEAD`) of `horizon_client::game::Game<'_>::block_item_display_geometry` (verify with `packaging/android/native_mod_adapt.py` that `horizon_client::game::Game<'_>::block_item_display_geometry` relocates cleanly to `text_displays::Game<'_>::block_item_display_geometry`).
   - In `timber_fall_render(&mut self, camera: glam::DVec3, partial: f64, out: &mut Vec<...>)`:
     - Compute smooth sub-tick angle using `partial` (`now = self.world.game_time as f64 + partial`, and interpolate `Falling` angle using `angular_vel * partial / 20.0` so it's silky-smooth at 60/120 FPS!).
     - Compute `matrix = glam::Mat4::from_translation((origin - camera).as_vec3()) * glam::Mat4::from_axis_angle(glam::Vec3::new(dz as f32, 0.0, -dx as f32), angle as f32) * glam::Mat4::from_translation(glam::Vec3::new(ox as f32, oy as f32, oz as f32))` (verify rotation sign so `+angle` tips `(+Y)` toward `toward = (dx, 0, dz)`!).
     - Call `self.mark_entity(out.len(), "minecraft:falling_block");` (if needed, check `text_displays.rs:279`) and `self.block_model_vertices(state, 42, matrix, light, true, out);` with world sky/block light at the hinge/block!
2. **Cancel vanilla break in `timber_fall_destroy_block`:**
   - Rename `_canceled` to `canceled` and set `*canceled = true;` when Timber Fall takes over and clears the tree.
   - Check `crates/horizon-client/src/game/fallingtrees.rs` to make sure `fallingtrees_destroy_block` doesn't interfere when `*canceled` is already `true` (or note if `fallingtrees_destroy_block` checks `*canceled`).
3. **Verify locally & check `native_mod_adapt.py`:**
   - Run `python3 -c` or inspect `native_mod_adapt.py` to confirm `block_item_display_geometry` relocates without error.
   - Run `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, and update `tasks/REPORT.md`.

## Constraints
- Do not modify any files outside `native-mods/timber_fall/` (plus `tasks/REPORT.md`).
- Clean up `native-mods/timber_fall/target/` if created.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

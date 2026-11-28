# Task: Add 3D Item Physics Native Mod (`native-mods/item_physics`)

## User Request
> "add item physics"

## Goal
Create a new native mod `native-mods/item_physics/` (and add `item_physics` to `.github/workflows/android.yml` `MODS`) that replaces vanilla floating/spinning dropped items with 3D item physics (items tumble in mid-air and lie flat on the ground at a natural seeded yaw).

## Reference Files to Inspect First
- `native-mods/zoom/` (`Cargo.toml`, `mixins/zoom/horizon-mixin.toml`, `list.rs`, `settings.rs`, `click.rs`, `runtime.rs`) — follow this exact native-mod layout.
- `native-mods/cinematic_interactions/mixins/cinematic_interactions/horizon-mixin.toml` (lines 60-71) — `cancellable = true` `HEAD` injection pattern.
- `crates/horizon-client/src/game/interface/item_draw.rs` (lines 748-935: `dropped_item_geometry` & `item_entity_geometry`, plus helper visibility like `owner_at`, `dropped_item_cull_box`, `resolve_item`, `submit_item_skinned`, `submit_item_world`, `mark_outline`, `entity_cull_decision_in_bounds`).
- `crates/horizon-client/src/entities.rs` (`ItemEntity` fields: `id`, `stack`, `previous`, `position`, `velocity`, `on_ground`, `age`, `bob_offset`, `fire_ticks`, etc.).

## Requirements
1. **Mixin (`native-mods/item_physics/mixins/item_physics/`):**
   - `horizon-mixin.toml`:
     - `[[module]]` in `crates/horizon-client/src/game.rs` (name `item_physics`, source `runtime.rs`).
     - `[[injection]]` `HEAD` (`cancellable = true`) on `horizon_client::game::interface::item_draw::Game<'_>::dropped_item_geometry` with `game: &crate::game::Game<'_>`, `camera: glam::DVec3`, `alpha: f64`, `out: &mut Vec<horizon_client::mesher::Vertex>`.
     - `native_mods` (`list.rs`), `native_settings` (`settings.rs`), `native_settings_click` (`click.rs`) so `"Item Physics"` appears in the Mods screen with an `Enabled: ON / OFF` toggle (ON by default).
   - **Physics & Pose (`runtime.rs`):**
     - When disabled, return `false` (don't cancel) so vanilla bob/spin runs normally.
     - When enabled, cancel vanilla `dropped_item_geometry` and render each `item` in `self.dropped` using the same culling, lighting, `resolve_item(..., DisplayContext::Ground, ...)`, `cluster_poses_with_depth`, `submit_item_skinned` / `submit_item_world`, and `mark_outline` pipeline as `item_entity_geometry`.
     - **No floating bob**: items rest flush on the block surface (`y` offset based on `state.bounding_box()` so they never float or sink into the ground).
     - **Airborne tumble**: when `!item.on_ground` (or falling/moving vertically), smoothly tumble in 3D (`rotation_x` + `rotation_z` + `rotation_y`) using `age = item.age as f32 + alpha as f32` and a per-`item.id` seed.
     - **Ground rest**:
       - Flat items (non-block items / thin `hi.z - lo.z < 0.2`) lie flat on the ground (`rotation_x = FRAC_PI_2`) with a deterministic seeded yaw (`rotation_z` / `rotation_y`) and flush `y` offset.
       - 3D block items sit upright/flush on the ground with a deterministic seeded yaw (no continuous spinning).
2. **Crate (`native-mods/item_physics/Cargo.toml` & `src/lib.rs` if needed for unit tests):**
   - Include unit tests covering the pose math (grounded flat item lies at `FRAC_PI_2` with zero bob, grounded block sits flush, airborne item tumbles smoothly as `age` advances, deterministic yaw from `item.id`).
3. **Android CI (`.github/workflows/android.yml`):**
   - Add `item_physics` to the `native_mods` default and fallback `MODS` string in `.github/workflows/android.yml`.
4. **Verification:**
   - Verify every method/field/module path used in `runtime.rs` against `crates/horizon-client/src/` (especially visibility of `item_draw.rs` helpers from `crate::game` — remember `horizon-mixin` injects `runtime.rs` as `crate::game::item_physics`, so `pub(super)` in `game::interface::item_draw` is `pub(in crate::game::interface)`, NOT visible to `crate::game`! Only `pub(in crate::game)` / `pub(crate)` or calling public/self methods on `Game` is visible — check this carefully and use `crate::game` visible methods or replicate small pure helpers like `dropped_item_cull_box` / `owner_at`!).
   - Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/item_physics/Cargo.toml`.
   - Clean `native-mods/item_physics/target/`, append a concise report to `/root/starling/tasks/REPORT.md`, and run:
     `tmux send-keys -t 1 "review" Enter`

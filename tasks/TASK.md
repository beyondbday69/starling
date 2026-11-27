# Task 9: Fix Pop White Flash (`v.layer = 5`) and Pass Through Other Trees (`timber_fall_hits`)

## User Feedback
> "its good but not perfect no white flash and it stucking i other trees just pass trough it"

## Fixes in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`
1. **Real White Flash on Popping (`v.layer = 5`):**
   - `v.color` is multiplicative, so lerping `v.color` to `255` didn't produce a white flash.
   - In `horizon-client` (`crates/horizon-client/src/game/entities/minecarts.rs:2818-2825` & `tnt.rs:824`), the official `OverlayTexture` white flash is:
     ```rust
     for v in &mut out[v_start..] {
         v.layer = 5;
     }
     ```
   - When `w > 0.35` (the final ~0.10 s right before each log pops, while `scale = 1.0 + 0.35 * w * w`), set `v.layer = 5` (and `v.light = 0xF0F0` if desired) on `&mut out[v_start..]` so the log flashes crisp white right before bursting!
2. **Pass Through Other Trees (`timber_fall_hits`):**
   - Currently `timber_fall_hits` stops as soon as `!self.native_block_collision(at).is_empty()`, which causes a falling tree to get stuck on neighboring trees' trunks/branches/leaves in a forest.
   - In `timber_fall_hits`, before treating `at` as a solid obstacle, read `let block_name = self.res.facts().get(self.world.block(at[0], at[1], at[2])).name;` and **skip** (`continue`) if it is any tree block (`is_log(&block_name)`, `is_leaves(&block_name)`, `block_name == "minecraft:vine"`, `"minecraft:cocoa"`, `"minecraft:bee_nest"`, `"minecraft:mangrove_roots"`, `"minecraft:muddy_mangrove_roots"`, `"minecraft:shroomlight"`, `block_name.ends_with("_wart_block")`).
   - Also: only check `native_block_collision(at)` for non-tree solid terrain (ground, stone, dirt, etc.), so a falling tree passes cleanly through any neighboring trees and lands on the terrain!

## Constraints
- Keep `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` passing on `native-mods/timber_fall/Cargo.toml` with 0 warnings.
- Clean up `native-mods/timber_fall/target/` and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

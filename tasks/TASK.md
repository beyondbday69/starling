# Task: Add Original Game Sound Effects to Timber Fall

## User Request
> "can we add sound eff from games orignal sound"
> "tell agent to add and gimme mp3 of sounds"

## Change
- Inspect `crates/horizon-client/src/game/modding/native_level_effects.rs` and `crates/horizon-client/src/game/modding/mod_api_host.rs` to see the exact method on `Game` / `Level` for playing vanilla sounds (e.g. `level_play_sound` / `native_play_sound`, `SoundSource::Blocks`).
- In `native-mods/timber_fall/mixins/timber_fall/runtime.rs`:
  - Play original vanilla Minecraft sound effects at the tree's world position during the fall animation (matching the vanilla sounds used in `native-mods/timber_fall/src/hooks.rs`):
    1. **Creak / fall start** (when a tree begins falling): `"minecraft:block.wood.break"` (volume `0.5`, pitch `0.6`).
    2. **Ground impact thud** (once when the trunk hits the ground / `impact_at` is set): `"minecraft:block.wood.fall"` (volume `0.9`, pitch `0.8`) and `"minecraft:block.wood.break"` (volume `0.7`, pitch `0.5`).
    3. **Leaf burst / log pop** (once on leaf burst / per-log pop, without spamming): `"minecraft:block.grass.break"` (volume `0.4`, pitch `1.4`) for leaves, and a light `"minecraft:block.wood.break"` (e.g. volume `0.35`, pitch `1.1`) when logs pop.
  - Keep the existing `0b2b2b8e71` animation intact (no pop flashes, no gravity drop, pass-through other trees).
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`.
- Clean `native-mods/timber_fall/target/` if created, append a concise report to `/root/starling/tasks/REPORT.md`, and when completely done run:
  `tmux send-keys -t 1 "review" Enter`

## Test Commands
```sh
cargo fmt --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml
```

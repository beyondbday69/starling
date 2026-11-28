# Task: Fix Sound Playback Method in Timber Fall Mixin

## Build Failure in CI (`android.yml` Run #46121778511)
```
error[E0599]: no method named `native_play_sound` found for mutable reference `&mut game::Game<'_>` in the current scope
     --> crates/horizon-client/src/game.rs:19303:18
```
`native_play_sound` is on `NativeLevelHandle`, not `Game<'_>`.

## Fix
- In `native-mods/timber_fall/mixins/timber_fall/runtime.rs`:
  - Update `timber_fall_sound` to take `dimension: &str` (instead of `native_play_sound`), look up `let Some(level) = self.level(dimension) else { return; };` (from `crates/horizon-client/src/game/modding/native_entity_refs.rs:190`), and call:
    ```rust
    let _ = self.level_play_sound(
        level,
        sound.0,
        at,
        crate::mod_api::SoundSource::Blocks,
        sound.1,
        sound.2,
    );
    ```
    (from `crates/horizon-client/src/game/modding/native_level_effects.rs:24`).
  - Pass `&tree.dimension` from `timber_fall_tick` (keeping the borrow clean since `tree.dimension` can be cloned or passed after `tree` borrow ends, or `let dim = tree.dimension.clone();` if needed, or queueing `(sound, at, dim)` / passing `&tree.dimension` if `tree` is not borrowed across `&mut self`). Note: in the existing `timber_fall_tick`, `tree` is `&mut ActiveTree` inside `{ let tree = &mut s.active[i]; ... }` while `s` is `MutexGuard` — wait: `self.level(&tree.dimension)` borrows `&self` (not `s`), and `self.level_play_sound` borrows `&mut self` (not `s`), so `self.timber_fall_sound(&tree.dimension, sounds::CREAK, at)` works cleanly without borrowing `s`!
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, clean `native-mods/timber_fall/target/`, append to `/root/starling/tasks/REPORT.md`, and run:
  `tmux send-keys -t 1 "review" Enter`

## Test Commands
```sh
cargo fmt --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path /root/starling/native-mods/timber_fall/Cargo.toml
```

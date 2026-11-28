# Task: Remove Pop Flashes from Timber Fall

## User Request
> "remove flashes from timber fall"

## Change
- In `native-mods/timber_fall/mixins/timber_fall/runtime.rs` (`timber_fall_render`):
  - Remove the white pop flash (`v.normal = [0, 0, 0, 0]`, `v.light = 0xF0F0`, `v.layer = 5`, and `v_start` vertex loop) completely so logs never flash white before popping.
  - Keep the rest of the falling/bounce/swell/pop/leaf/camera-shake animation intact.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`, clean `native-mods/timber_fall/target/`, and append a short report to `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

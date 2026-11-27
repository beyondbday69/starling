# Fix Task: Match `batch` Binding Type in `zoom/horizon-mixin.toml` and `bars.rs` (`E_INVALID_BINDING`)

## Root Cause (CI Run `46002993425`)
```json
{
  "code": "E_INVALID_BINDING",
  "message": "binding 'batch' must match helper parameter name and type in declaration order",
  "mod_id": "zoom",
  "injection_id": "bars"
}
```
- `native-mods/zoom/mixins/zoom/horizon-mixin.toml:58` has `ty = "&mut HudBatch"`, while `native-mods/zoom/mixins/zoom/bars.rs:4` has `batch: &mut crate::hud::HudBatch`.

## Fix
- Make the `batch` parameter type identical in both `native-mods/zoom/mixins/zoom/horizon-mixin.toml` and `native-mods/zoom/mixins/zoom/bars.rs` (use `&mut crate::hud::HudBatch` in both, matching `native_mods`'s `crate::...` style — and verify `crate::hud::HudBatch` resolves inside `horizon_client::game::native_hooks::gui_render_post`, or use `&mut horizon_client::hud::HudBatch` if `gui_render_post` is in `horizon-client`).
- Verify with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/zoom/Cargo.toml`, clean `native-mods/zoom/target/`, and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/zoom/Cargo.toml --check
cargo clippy --manifest-path native-mods/zoom/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/zoom/Cargo.toml
```

# Task: Add Cinematic Bars & Auto-Hide HUD to Zoom Mod (`native-mods/zoom`)

## User Request
> "for zoom mod add feature cinimatic bars will came from above and bottem snd huid will get dissper when we relese the z key itll come back"

## Requirements
1. **Settings (`native-mods/zoom/mixins/zoom/runtime.rs`):**
   - Add `pub cinematic: bool` to `Settings` (default `true`, read from `"cinematic"` via `flag("cinematic")`).
   - Expose `"Cinematic bars: ON/OFF"` in `Settings::labels()` (button index 3), toggle it in `Settings::click(3)`, and persist `"cinematic"` in `Settings::stored()` (keep existing `"level"`, `"smooth"`, `"sensitivity"` intact).
2. **Auto-Hide HUD on Zoom Key (`key.rs` + `runtime.rs`):**
   - In `zoom_key` (`native-mods/zoom/mixins/zoom/key.rs`):
     - When `pressed == true` and `game.native_playing()` and `settings.cinematic` is `true`:
       - Save the player's previous `mod_api::HudMode` (`game.ui.hud_mode()`) in `State` (only if not already saved during this hold) and call `game.ui.set_hud_mode(mod_api::HudMode::AllHidden)` (which sets `hide_gui = true` so the HUD disappears cleanly).
     - When `pressed == false` (Z key released) or zoom is disabled:
       - Restore the saved `HudMode` via `game.ui.set_hud_mode(prev)` and clear the saved mode so the HUD comes right back!
3. **Smooth Animated Cinematic Bars from Top & Bottom (`bars.rs` / `horizon-mixin.toml` / `runtime.rs`):**
   - Inspect `crates/horizon-client/src/game.rs` around line 5457 (`draw_every_toast(&mut self, batch: &mut HudBatch, ...)` or `hud_layers`) and `crates/horizon-client-render/src/hud.rs` (`HudBatch` rectangle drawing helper, e.g. `solid` / `fill_rect` / `quad`) to get exact verified method signatures and `horizon-mixins` relocation with `packaging/android/native_mod_adapt.py`.
   - Inject a `HEAD` hook into `horizon_client::game::Game<'_>::draw_every_toast` (or another verified unconditional HUD-pass method that runs even when `hide_gui == true`) so the cinematic bars STILL draw while the HUD is hidden!
   - Smoothly animate `bar_progress: f32` (`0.0..=1.0`) using `std::time::Instant` frame delta (~`0.18 s` ease-out):
     - While `held.is_some()` and `settings.cinematic`: animate `bar_progress` toward `1.0`.
     - When released (`held.is_none()`): animate `bar_progress` back down toward `0.0`.
     - When `bar_progress > 0.0` (and `game.native_playing()`): draw two solid black (`0xFF00_0000` / `rgba(0, 0, 0, 255)`) bars across the full `batch.width`, sliding in from the top (`y = 0.0`, `height = bar_h`) and bottom (`y = batch.height - bar_h`, `height = bar_h`), where `bar_h = batch.height * 0.12 * eased(bar_progress)`.
4. **Include `zoom` in `.github/workflows/android.yml`:**
   - Ensure `zoom` is in the default `MODS` list in `.github/workflows/android.yml` (`cinematic_interactions skins_3d gyro timber_fall zoom distant_horizons`) so the APK includes it.

## Constraints
- Verify all `HudBatch` and `Game<'_>` targets/methods in `crates/horizon-client/` and with `packaging/android/native_mod_adapt.py`.
- Keep `native-mods/zoom` pure-Rust tests passing (`cargo fmt --manifest-path native-mods/zoom/Cargo.toml --check`, `cargo clippy --manifest-path native-mods/zoom/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path native-mods/zoom/Cargo.toml`), and update/add unit tests in `runtime.rs` for `cinematic` setting and `HudMode` / bar progress animation!
- No symlinks or inner `#![...]` / `//!` file attributes in `native-mods/zoom/mixins/zoom/*.rs`.
- Clean up `native-mods/zoom/target/` and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/zoom/Cargo.toml --check
cargo clippy --manifest-path native-mods/zoom/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/zoom/Cargo.toml
```

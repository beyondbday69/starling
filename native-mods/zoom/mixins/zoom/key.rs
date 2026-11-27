// The zoom key went down or up: the player's choice of key, kept as the
// WebAssembly Zoom kept it, and the cinematic bars' hold on the HUD.
fn zoom_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::official_zoom as zoom;
    use crate::mod_api::HudMode;
    // Turned off in Options > Mods: nothing until the next build leaves it out.
    if !game.native_on(zoom::ID) {
        return;
    }
    let bound = game.native_mod_key(zoom::ID, zoom::KEY, zoom::DEFAULT_KEY);
    if key != bound {
        return;
    }
    let playing = game.native_playing();
    let cinematic = zoom::settings(|name| game.native_setting(zoom::ID, name)).cinematic;
    zoom::key(playing, pressed, |name| game.native_setting(zoom::ID, name));
    // The bars slide in over the view, so the HUD gives way for the hold and
    // comes straight back when the key is let go (`zoom_bars` draws them).
    if pressed && playing && cinematic {
        let saved = zoom::hide_hud(match game.hud_mode() {
            HudMode::AllVisible => zoom::Hud::AllVisible,
            HudMode::NoHud => zoom::Hud::NoHud,
            HudMode::AllHidden => zoom::Hud::AllHidden,
        });
        if saved {
            game.set_hud_mode(HudMode::AllHidden);
        }
    } else if let Some(hidden) = zoom::show_hud() {
        game.set_hud_mode(match hidden {
            zoom::Hud::AllVisible => HudMode::AllVisible,
            zoom::Hud::NoHud => HudMode::NoHud,
            zoom::Hud::AllHidden => HudMode::AllHidden,
        });
    }
}

// native_hooks::Game::native_settings_click (a hook point): a click on the
// Falling Trees page changes and keeps that setting.
fn fallingtrees_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::fallingtrees::ID {
        game.fallingtrees_settings_click(button);
    }
}

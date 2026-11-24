// native_hooks::Game::native_settings_click (a hook point): a click on the
// Sodium page changes and keeps that setting.
fn sodium_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::sodium::ID {
        game.sodium_settings_click(button);
    }
}

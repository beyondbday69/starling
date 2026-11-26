// native_hooks::Game::native_settings_click (a hook point): toggles a Timber Fall setting.
fn timber_fall_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::official_timber_fall::ID {
        game.timber_fall_settings_click(button);
    }
}
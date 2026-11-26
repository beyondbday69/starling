// native_hooks::Game::native_settings (a hook point): Timber Fall's buttons.
fn timber_fall_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == crate::game::official_timber_fall::ID {
        buttons.extend(game.timber_fall_settings());
    }
}
// native_hooks::Game::native_settings (a hook point): Sodium's settings buttons.
fn sodium_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == crate::game::sodium::ID {
        buttons.extend(game.sodium_settings());
    }
}

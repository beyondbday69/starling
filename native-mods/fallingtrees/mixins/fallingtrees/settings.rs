// native_hooks::Game::native_settings (a hook point): Falling Trees buttons.
fn fallingtrees_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == crate::game::fallingtrees::ID {
        buttons.extend(game.fallingtrees_settings());
    }
}

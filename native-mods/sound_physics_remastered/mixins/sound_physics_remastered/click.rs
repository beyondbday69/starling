// native_hooks::Game::native_settings_click (a hook point): a click on the
// Sound Physics Remastered page changes and keeps that setting.
fn sound_physics_remastered_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::sound_physics_remastered::ID {
        game.sound_physics_remastered_settings_click(button);
    }
}

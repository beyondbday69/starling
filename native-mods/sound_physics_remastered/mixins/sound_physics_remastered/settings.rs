// native_hooks::Game::native_settings (a hook point): Sound Physics Remastered's settings buttons.
fn sound_physics_remastered_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == crate::game::sound_physics_remastered::ID {
        buttons.extend(game.sound_physics_remastered_settings());
    }
}

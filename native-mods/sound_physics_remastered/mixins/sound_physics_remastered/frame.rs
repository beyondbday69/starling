// native_hooks::Game::native_frame (a hook point): process sound physics every frame.
fn sound_physics_remastered_frame_hook(game: &mut crate::game::Game<'_>) {
    game.sound_physics_remastered_frame();
}

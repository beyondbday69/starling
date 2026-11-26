// native_hooks::Game::native_tick (a hook point): advances active falling trees.
fn timber_fall_tick(game: &mut crate::game::Game<'_>) {
    game.timber_fall_tick();
}
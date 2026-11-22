// native_hooks::Game::native_tick (a hook point): executes queued fellings.
fn fallingtrees_tick(game: &mut crate::game::Game<'_>) {
    game.fallingtrees_tick();
}

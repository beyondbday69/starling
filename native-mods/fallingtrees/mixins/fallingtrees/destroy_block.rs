// native_hooks::Game::native_destroy_block (a hook point): chops the whole tree.
fn fallingtrees_destroy_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    state: horizon_world::registry::BlockStateId,
    canceled: &mut bool,
) {
    game.fallingtrees_destroy_block(pos, state, canceled);
}

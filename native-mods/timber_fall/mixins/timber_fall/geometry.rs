// native_hooks::Game::native_submit_custom_geometry (a hook point): draws each active
// falling tree as rotated model instances around its stump hinge.
fn timber_fall_geometry(
    game: &crate::game::Game<'_>,
    draw: &mut horizon_client::world_geometry::WorldGeometry,
) {
    game.timber_fall_geometry(draw);
}
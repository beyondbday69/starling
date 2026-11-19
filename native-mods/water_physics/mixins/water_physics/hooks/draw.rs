fn water_physics_draw(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    crate::game::native_water_physics_glue::draw(game, camera, out);
}

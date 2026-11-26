// block_item_display_geometry (a hook point): draws every active falling tree's blocks as
// rotated vanilla block models, without the standalone-model lookup.
fn timber_fall_display_geometry(
    game: &mut crate::game::Game<'_>,
    camera: glam::DVec3,
    partial: f64,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    game.timber_fall_render(camera, partial, out);
}
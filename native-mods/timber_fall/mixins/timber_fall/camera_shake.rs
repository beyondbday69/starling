// native_camera_shake (a hook point): a little shake as the trunk hits the ground.
fn timber_fall_camera_shake(
    game: &crate::game::Game<'_>,
    partial: f32,
    shake: &mut mod_api::Shake,
) {
    game.timber_fall_camera_shake(partial, shake);
}
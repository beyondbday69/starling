// Each frame, after vanilla placed the camera: Punchy Moves' shoulder camera,
// eased in and out and drawn from the pose the controller computes.
fn punchy_moves_camera(
    game: &crate::game::Game<'_>,
    position: &mut [f64; 3],
    yaw: &mut f32,
    pitch: &mut f32,
) {
    use crate::game::punchy_moves as punchy;
    if !game.native_on(punchy::ID) {
        punchy::camera::reset();
        return;
    }
    let settings = punchy::camera::settings(|key| game.native_setting(punchy::ID, key));
    if !settings.enabled {
        punchy::camera::reset();
        return;
    }
    // The game's own collision is not reachable here, so the arm starts at
    // full reach; a frame that finds a wall pulls it in without a pop.
    let dt = punchy::camera::frame_dt();
    if let Some(pose) = punchy::camera::frame_pose(*position, *yaw, *pitch, dt, 1.0, &settings) {
        *position = pose.position;
        *yaw = pose.yaw;
        *pitch = pose.pitch;
    }
}

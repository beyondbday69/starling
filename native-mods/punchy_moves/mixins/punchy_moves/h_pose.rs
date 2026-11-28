// After vanilla posed a player: Punchy Moves' angles on top of the vanilla
// pose (radians, model units).
fn punchy_moves_pose(value: (), p: &mut crate::mobs::anim::Poses) {
    use crate::game::punchy_moves::engine;
    if let Some(pose) = engine::finish_pose() {
        for (name, part) in pose.parts() {
            let q = p.get(name);
            q[0] += part.x;
            q[1] += part.y;
            q[2] += part.z;
            q[3] += part.x_rot;
            q[4] += part.y_rot;
            q[5] += part.z_rot;
        }
    }
    value
}

// Which player the player model is being posed for (setupAnim), and the walk
// state vanilla extracted: the engine's step for that player this frame.
fn punchy_moves_pose_start(m: &crate::mobs::Mob, s: &crate::mobs::anim::State) {
    use crate::game::punchy_moves::engine::{PoseInput, begin_pose};
    begin_pose(
        m.id as u64,
        PoseInput {
            age: s.age,
            speed: s.speed,
            position: s.pos,
            partial: s.partial,
        },
    );
}

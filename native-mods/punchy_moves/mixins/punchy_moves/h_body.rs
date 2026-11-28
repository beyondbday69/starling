// After setupRotations: Punchy Moves' lean, bank, bob, squash and roll on the
// whole player model.
fn punchy_moves_body(value: glam::Mat4, m: &crate::mobs::Mob, alpha: f32) -> glam::Mat4 {
    use crate::game::punchy_moves::engine;
    let Some(root) = engine::sampled_root_motion(m.id as u64, alpha) else {
        return value;
    };
    value
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, root.bob, 0.0))
        * glam::Mat4::from_rotation_x(root.lean)
        * glam::Mat4::from_rotation_z(root.bank)
        * glam::Mat4::from_rotation_y(root.roll)
        * glam::Mat4::from_scale(glam::Vec3::new(
            1.0 + root.squash * 0.5,
            1.0 - root.squash,
            1.0 + root.squash * 0.5,
        ))
}

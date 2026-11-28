// After a player's body is submitted: Punchy Moves' face, one quad per
// feature, on the front of the head. The sheets are the mod's own textures,
// looked up in the atlas the mobs are drawn from, so a pack without them
// simply draws no face.
fn punchy_moves_face(
    value: (),
    m: &crate::mobs::Mob,
    alpha: f32,
    camera: glam::DVec3,
    atlas: &dyn Fn(&str) -> Option<[f32; 4]>,
    light: u32,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    use crate::game::punchy_moves::{engine, faces};
    if &*m.kind.id != "minecraft:player" {
        return value;
    }
    let Some(quads) = engine::sampled_face_quads(m.id as u64) else {
        return value;
    };
    if quads.is_empty() {
        return value;
    }
    let (Some(fx), Some(marks)) = (atlas(faces::FX_TEXTURE), atlas(faces::MARKS_TEXTURE)) else {
        return value;
    };
    let Some(head) = engine::sampled_head_pose(m.id as u64) else {
        return value;
    };
    let base = crate::mobs::render::base_transform(m, alpha, camera);
    // The player model's head pivot sits one and a half blocks above the
    // feet; the engine's head pose turns it.
    let head_matrix = base
        * glam::Mat4::from_translation(glam::Vec3::new(0.0, 1.5, 0.0))
        * glam::Mat4::from_quat(glam::Quat::from_euler(
            glam::EulerRot::ZYX,
            head.z_rot,
            head.y_rot,
            head.x_rot,
        ));
    let n = head_matrix
        .transform_vector3(glam::Vec3::Z)
        .normalize_or_zero();
    let n = n.to_array().map(|v| (v.clamp(-1.0, 1.0) * 127.0) as i8);
    // Two-sided: a face is never culled when the head turns away.
    let normal = [n[0], n[1], n[2], -127];
    for quad in quads.iter() {
        let rect = if quad.texture == faces::MARKS_TEXTURE {
            marks
        } else {
            fx
        };
        let corners = quad.corners();
        let uvs = quad.uv_corners();
        let alpha8 = (quad.color[3].clamp(0.0, 1.0) * 255.0) as u32;
        let color = alpha8 << 24 | 0x00FF_FFFF;
        // Two triangles, the corners of the quad in order.
        for i in [0usize, 1, 2, 0, 2, 3] {
            let pos = head_matrix.transform_point3(glam::Vec3::from_array(corners[i]));
            out.push(crate::mesher::Vertex {
                pos: pos.to_array(),
                uv: [
                    rect[0] + (rect[2] - rect[0]) * uvs[i][0],
                    rect[1] + (rect[3] - rect[1]) * uvs[i][1],
                ],
                layer: 0,
                color,
                light,
                normal,
                glint_uv: [0.0, 0.0],
            });
        }
    }
    value
}

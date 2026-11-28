// item_draw::Game::dropped_item_geometry (a hook point): when Item Physics is
// on it takes the dropped items over — the same culling, lighting, resolver and
// submit pipeline as the vanilla draw, with physics poses instead of the bob
// and the spin. Off, vanilla runs as it always did.
fn item_physics_dropped(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> ::core::ops::ControlFlow<()> {
    if dropped(game, camera, alpha, out) {
        ::core::ops::ControlFlow::Break(())
    } else {
        ::core::ops::ControlFlow::Continue(())
    }
}

/// The dropped items, drawn with physics poses: `true` when it drew them, so
/// the caller cancels vanilla's bob and spin.
fn dropped(
    game: &crate::game::Game<'_>,
    camera: glam::DVec3,
    alpha: f64,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> bool {
    use crate::game::item_physics as physics;
    use crate::game::entity_culling::EntityCullDecision;
    use crate::game::item_draw::owner_at;
    use horizon_assets::item_model::DisplayContext;
    use horizon_client::{block_entity_render::diffuse_lights, graphics::EntityCullingMode, item_render};
    use horizon_core::geometry::{Aabb, Vec3};

    if !game.native_on(physics::ID)
        || !physics::enabled(|key| game.native_setting(physics::ID, key))
    {
        return false;
    }
    // Entity.shouldRenderAtSqrDistance for the item's 0.25 box, from its current
    // position, as the vanilla draw scales it by the view.
    let range = crate::game::entity_culling::default_range(
        glam::DVec3::splat(0.25),
        game.view_scale(),
    );
    let skinning = game.skinning();
    let lights = diffuse_lights(game.dim == crate::game::Dim::Nether);
    for item in game.dropped.iter() {
        let entity_start = out.len();
        game.mark_motion(entity_start, (2 << 32) | u64::from(item.id));
        let (p, q) = (item.previous, item.position);
        let at = [
            p.x + (q.x - p.x) * alpha,
            p.y + (q.y - p.y) * alpha,
            p.z + (q.z - p.z) * alpha,
        ];
        let far =
            (glam::DVec3::new(q.x, q.y, q.z) - camera).length_squared() >= range * range;
        if far {
            continue;
        }
        let pos = glam::DVec3::from_array(at) - camera;
        // `EntityRenderer.shouldRender`: the item's box, grown by 0.5, in the
        // view frustum, before its model is resolved.
        if let Some(view) = &game.view_frustum {
            let (lo, hi) = cull_box(glam::DVec3::from_array(at), camera);
            if !view.intersects(lo, hi) {
                continue;
            }
        }
        // `extractItemGroupRenderState`: GROUND, seeded by the entity id.
        let owner = owner_at([q.x, q.y, q.z], 0.0, "minecraft:item");
        let state = game.resolve_item(
            &item.stack,
            DisplayContext::Ground,
            Some(&owner),
            item.id as i32,
        );
        if state.is_empty() {
            continue;
        }
        let age = item.age as f32 + alpha as f32;
        let (lo, hi) = state.bounding_box();
        let pose = physics::pose(
            item.id,
            item.on_ground,
            physics::is_thin(hi.z - lo.z),
            age,
            [lo.x, lo.y, lo.z],
            [hi.x, hi.y, hi.z],
        );
        let model = glam::Mat4::from_translation(pos.as_vec3())
            * glam::Mat4::from_translation(glam::Vec3::new(0.0, pose.lift, 0.0))
            * glam::Mat4::from_rotation_y(pose.rotation_y)
            * glam::Mat4::from_rotation_z(pose.rotation_z)
            * glam::Mat4::from_rotation_x(pose.rotation_x);
        // `EntityRenderer.getPackedLightCoords`: lit from the block its light
        // probe (its eye, 0.2125 up) is in.
        let probe = [
            at[0],
            at[1] + horizon_client::entities::ItemEntity::EYE_HEIGHT,
            at[2],
        ];
        let packed = item.render_light(game.world.light(
            probe[0].floor() as i32,
            probe[1].floor() as i32,
            probe[2].floor() as i32,
        ));
        let seed = i64::from(game.pack.item_renderer.item_id(&item.stack.item))
            + i64::from(item.stack.damage);
        let copies = item_render::cluster_poses_with_depth(
            model,
            item_render::rendered_amount(item.stack.count),
            seed,
            hi.z - lo.z,
        );
        // ItemEntity's collision box is only 0.25 blocks and cannot prove that
        // this larger rendered model stays behind an occluder. The posed
        // envelope walks every model point in every copy; with culling off the
        // decision is `Bypass` whatever it holds.
        let culling = Some(
            if game.settings.horizon.entity_culling == EntityCullingMode::Off {
                EntityCullDecision::Bypass
            } else {
                state
                    .posed_bounds(&copies)
                    .map(|(lo, hi)| {
                        game.entity_cull_decision_in_bounds(
                            item,
                            Aabb::new(
                                Vec3::new(
                                    camera.x + f64::from(lo.x),
                                    camera.y + f64::from(lo.y),
                                    camera.z + f64::from(lo.z),
                                ),
                                Vec3::new(
                                    camera.x + f64::from(hi.x),
                                    camera.y + f64::from(hi.y),
                                    camera.z + f64::from(hi.z),
                                ),
                            ),
                        )
                    })
                    .unwrap_or_else(|| {
                        game.entity_cull_decision(item, f64::from(alpha as f32))
                    })
            },
        );
        if culling.is_some_and(|decision| decision.hides_entity()) {
            continue;
        }
        let outline = game.thing_outline(item.id, "minecraft:item");
        // Posed on the GPU when every layer can be, its silhouette too when it glows.
        let skinned = skinning
            .then(|| game.submit_item_skinned(&state, &copies, packed, lights))
            .flatten();
        if skinned.is_none() {
            for &copy in &copies {
                game.submit_item_world(&state, copy, packed, out);
            }
        }
        if let Some(rgb) = outline {
            game.mark_outline(&out[entity_start..], rgb);
            if let Some(((first, _, _), _)) = skinned {
                let color = horizon_client::mobs::render::abgr(rgb);
                game.skin_frame.borrow_mut().outline_since(first, color);
            }
        } else if let Some(culling) = culling {
            game.finish_entity_occlusion(culling, item, out, entity_start);
        } else {
            game.cull_skinned(out, entity_start, skinned);
        }
    }
    true
}

/// `EntityRenderer.shouldRender`'s box for a dropped item, in camera space
/// (`game/interface/item_draw.rs`'s own helper, replicated here: it is private
/// to that module).
fn cull_box(position: glam::DVec3, camera: glam::DVec3) -> (glam::Vec3, glam::Vec3) {
    let feet = (position - camera).as_vec3();
    let half = glam::Vec3::new(0.125 + 0.5, 0.5, 0.125 + 0.5);
    (
        feet - half,
        feet + glam::Vec3::new(half.x, 0.25 + 0.5, half.z),
    )
}

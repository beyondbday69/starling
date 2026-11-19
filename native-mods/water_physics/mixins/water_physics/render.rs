// Geometric water wave and wake renderer (v3.5).
//
// Key features:
// - Shader-Preserving Water Optics: Eliminates artificial body tint overlays. Uses 100% transparent
//   baseline with pure neutral Fresnel specular sun/skylight glint highlights so the underlying
//   shader pack's water color (e.g. tropical turquoise, emerald, sapphire), caustics, and depth absorption
//   show through completely.
// - Full 3D Boat Wake Field: Replaces two isolated ribbon lines with a complete hydrodynamic wake field:
//   * Central hull wash / stern keel trail trailing directly behind the boat keel.
//   * Transverse Kelvin wavelets bridging the hull wash and the outer V-arms.
//   * Hydrodynamic lateral Kelvin V-arms radiating outwards.
// - Concentric ripple rings: Smooth 24-gon expanding torus quads with raised 3D crests (+0.045m to +0.060m).
//   Harmonic envelope ensures zero spawn/death pops and zero-alpha seamless edge blending.

use crate::game::Game;
use super::native_water_physics_sim as sim;
use super::native_water_physics_api as api;
use horizon_client::mesher::Vertex;

pub struct RenderCache {
    pub dirty: bool,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderCache {
    pub fn new() -> Self {
        Self { dirty: true }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.dirty = false;
    }
}

pub fn render_wave_overlays(
    sim: &sim::WaterSimulation,
    v4: &api::WaterV4,
    cache: &mut RenderCache,
    game: &Game<'_>,
    camera: glam::DVec3,
    out: &mut Vec<Vertex>,
) {
    game.render_wave_overlays(sim, v4, cache, camera, out);
}

impl Game<'_> {
    pub(super) fn render_wave_overlays(
        &self,
        sim: &sim::WaterSimulation,
        v4: &api::WaterV4,
        _cache: &mut RenderCache,
        camera: glam::DVec3,
        out: &mut Vec<Vertex>,
    ) {
        // v4: render ambient 3D surface whenever either engine has activity.
        let (v4_ripples, v4_wakes) = v4.active_counts();
        if !sim.any_active() && v4_ripples == 0 && v4_wakes == 0 {
            return;
        }

        let (u_white, v_white) = if let Some(r) = self.gui.atlas.sprites.get("horizon/white") {
            ((r.u0 + r.u1) * 0.5, (r.v0 + r.v1) * 0.5)
        } else {
            (0.0, 0.0)
        };

        let layer = horizon_client::gfx::TRANSLUCENT_PARTICLE_OVERLAY | horizon_client::gfx::OIT_ENTITY;
        let cam_y = -camera.y as f32;

        let sun_x = 0.28f32;
        let sun_z = -0.38f32;

        // Neutral specular glint color (pure celestial sun/skylight glint)
        // Baseline has alpha = 0 so underlying shader water color shows through 100% untouched.
        let (glint_r, glint_g, glint_b) = (248u32, 252u32, 255u32);
        let color_edge = (glint_b << 16) | (glint_g << 8) | glint_r; // alpha = 0 for 100% transparent baseline

        // -------------------------------------------------------------
        // 1. Render Concentric Ripple Rings (Smooth circular 3D waves)
        // -------------------------------------------------------------
        let segments = sim.segments;
        let step_rad = std::f32::consts::PI * 2.0 / (segments as f32);

        for ring in &sim.rings {
            if !ring.is_alive() {
                continue;
            }

            let env = ring.envelope();
            let amp = ring.amplitude * env;

            if amp < 0.005 {
                continue;
            }

            let bx = ring.center_x.floor() as i32;
            let by = ring.water_y.floor() as i32;
            let bz = ring.center_z.floor() as i32;
            let light_packed = self.world.light(bx, by + 1, bz);
            let light = horizon_client::block_entity_render::vertex_light(light_packed);

            // Ring geometry parameters: expands smoothly with radius
            let r_crest = ring.radius;
            let width = (0.14 + 0.16 * ring.radius).clamp(0.18, 0.48);
            let r_inner = (r_crest - width * 0.5).max(0.02);
            let r_outer = r_crest + width * 0.5;

            // Physical 3D crest displacement: elevates smoothly up to +0.060m.
            // v4 adds a live ambient height sample so crests undulate in full 3D
            // instead of sitting on a flat plane.
            let crest_h = (0.045 * amp).min(0.060);
            let base_cam_x = (ring.center_x - camera.x) as f32;
            let base_cam_z = (ring.center_z - camera.z) as f32;
            let ambient = v4.surface_height(ring.center_x as f32, ring.center_z as f32) * 0.5;
            let y_base = ring.water_y as f32 + 0.002 + cam_y;
            let y_crest = ring.water_y as f32 + 0.003 + crest_h + ambient + cam_y;

            let alpha_crest = (0.18 * amp).clamp(0.0, 0.22);
            let a_crest_u32 = (alpha_crest * 255.0) as u32;

            for k in 0..segments {
                let th0 = (k as f32) * step_rad;
                let th1 = ((k + 1) as f32) * step_rad;

                let c0 = th0.cos();
                let s0 = th0.sin();
                let c1 = th1.cos();
                let s1 = th1.sin();

                // Surface-normal lighting on wave crest. v4 supplies a real
                // per-vertex 3D normal; blend it with the radial sun term.
                let n = v4.surface_normal(
                    (ring.center_x + c0 * r_crest) as f32,
                    (ring.center_z + s0 * r_crest) as f32,
                );
                let norm_dot = (c0 * sun_x + n[1] * 0.92 + s0 * sun_z).clamp(0.0, 1.0);
                let lit = 0.85 + 0.20 * norm_dot + 0.10 * n[0];

                // Neutral specular sun glint highlight - shader water shows through 100%
                let r_col = ((glint_r as f32 * lit).min(255.0)) as u32;
                let g_col = ((glint_g as f32 * lit).min(255.0)) as u32;
                let b_col = ((glint_b as f32 * lit).min(255.0)) as u32;

                let color_crest = (a_crest_u32 << 24) | (b_col << 16) | (g_col << 8) | r_col;

                // Positions:
                // Inner rim (y_base, alpha=0)
                let in0 = [base_cam_x + c0 * r_inner, y_base, base_cam_z + s0 * r_inner];
                let in1 = [base_cam_x + c1 * r_inner, y_base, base_cam_z + s1 * r_inner];

                // Crest peak (y_crest, alpha=crest)
                let cr0 = [base_cam_x + c0 * r_crest, y_crest, base_cam_z + s0 * r_crest];
                let cr1 = [base_cam_x + c1 * r_crest, y_crest, base_cam_z + s1 * r_crest];

                // Outer rim (y_base, alpha=0)
                let out0 = [base_cam_x + c0 * r_outer, y_base, base_cam_z + s0 * r_outer];
                let out1 = [base_cam_x + c1 * r_outer, y_base, base_cam_z + s1 * r_outer];

                // Slope 1: Inner rim -> Crest
                out.push(Vertex { pos: in0, uv: [u_white, v_white], layer, color: color_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: in1, uv: [u_white, v_white], layer, color: color_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: cr1, uv: [u_white, v_white], layer, color: color_crest, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: cr0, uv: [u_white, v_white], layer, color: color_crest, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                // Slope 2: Crest -> Outer rim
                out.push(Vertex { pos: cr0, uv: [u_white, v_white], layer, color: color_crest, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: cr1, uv: [u_white, v_white], layer, color: color_crest, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: out1, uv: [u_white, v_white], layer, color: color_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: out0, uv: [u_white, v_white], layer, color: color_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            }
        }

        // -------------------------------------------------------------
        // 2. Render Continuous Kelvin V-Wakes & Hydrodynamic Wake Fields
        // -------------------------------------------------------------
        self.render_wake_ribbon(&sim.player_wakes, camera, cam_y, u_white, v_white, layer, out);
        self.render_wake_ribbon(&sim.boat_wakes, camera, cam_y, u_white, v_white, layer, out);
    }

    fn render_wake_ribbon(
        &self,
        wake_nodes: &[sim::WakeNode],
        camera: glam::DVec3,
        cam_y: f32,
        u_white: f32,
        v_white: f32,
        layer: u32,
        out: &mut Vec<Vertex>,
    ) {
        if wake_nodes.len() < 2 {
            return;
        }

        let sun_x = 0.28f32;
        let sun_y = 0.88f32;
        let sun_z = -0.38f32;

        let (glint_r, glint_g, glint_b) = (248u32, 252u32, 255u32);
        let col_edge = (glint_b << 16) | (glint_g << 8) | glint_r; // alpha = 0 for 100% transparent baseline

        for i in 0..wake_nodes.len() - 1 {
            let n0 = &wake_nodes[i];
            let n1 = &wake_nodes[i + 1];

            // Continuity guard 1: Same entity type
            if n0.is_boat != n1.is_boat {
                continue;
            }

            // Continuity guard 2: Distance check (prevent stretching across pauses, jumps, teleports)
            let dx = (n1.pos_x - n0.pos_x) as f32;
            let dz = (n1.pos_z - n0.pos_z) as f32;
            let dist_sq = dx * dx + dz * dz;
            if dist_sq > 1.40 * 1.40 || dist_sq < 0.0001 {
                continue;
            }

            // Continuity guard 3: Direction check (prevent twisted bowtie ribbons on sharp turns)
            let dot_dir = n0.dir_x * n1.dir_x + n0.dir_z * n1.dir_z;
            if dot_dir < 0.15 {
                continue;
            }

            let env0 = n0.envelope();
            let env1 = n1.envelope();
            let amp0 = n0.amplitude * env0;
            let amp1 = n1.amplitude * env1;

            if amp0 < 0.003 && amp1 < 0.003 {
                continue;
            }

            let bx0 = n0.pos_x.floor() as i32;
            let by0 = n0.water_y.floor() as i32;
            let bz0 = n0.pos_z.floor() as i32;
            let light = horizon_client::block_entity_render::vertex_light(self.world.light(bx0, by0 + 1, bz0));

            let base_x0 = (n0.pos_x - camera.x) as f32;
            let base_z0 = (n0.pos_z - camera.z) as f32;
            let base_x1 = (n1.pos_x - camera.x) as f32;
            let base_z1 = (n1.pos_z - camera.z) as f32;

            let y_base0 = n0.water_y as f32 + 0.002 + cam_y;
            let y_base1 = n1.water_y as f32 + 0.002 + cam_y;

            // Physical 3D wave displacement: sleek raised crest line (+0.04m to +0.06m)
            let crest_h0 = (0.040 * amp0).min(0.060);
            let crest_h1 = (0.040 * amp1).min(0.060);
            let y_crest0 = y_base0 + 0.001 + crest_h0;
            let y_crest1 = y_base1 + 0.001 + crest_h1;

            let hw0 = n0.half_width;
            let hw1 = n1.half_width;

            // Slender, hydrodynamic arm thickness
            let arm_w0 = if n0.is_boat {
                (0.22 + 0.06 * hw0).clamp(0.24, 0.42)
            } else {
                (0.16 + 0.05 * hw0).clamp(0.18, 0.32)
            };
            let arm_w1 = if n1.is_boat {
                (0.22 + 0.06 * hw1).clamp(0.24, 0.42)
            } else {
                (0.16 + 0.05 * hw1).clamp(0.18, 0.32)
            };

            let half_w0 = arm_w0 * 0.5;
            let half_w1 = arm_w1 * 0.5;

            // Pure neutral specular glint highlight over shader water
            let alpha_cr0 = (0.18 * amp0).clamp(0.0, 0.22);
            let alpha_cr1 = (0.18 * amp1).clamp(0.0, 0.22);
            let a_cr0_u32 = (alpha_cr0 * 255.0) as u32;
            let a_cr1_u32 = (alpha_cr1 * 255.0) as u32;

            let lit0 = 1.0 + 0.20 * (n0.dir_x * sun_x + 0.92 * sun_y + n0.dir_z * sun_z).clamp(0.0, 1.0);
            let lit1 = 1.0 + 0.20 * (n1.dir_x * sun_x + 0.92 * sun_y + n1.dir_z * sun_z).clamp(0.0, 1.0);

            let r_cr0 = ((glint_r as f32 * lit0).min(255.0)) as u32;
            let g_cr0 = ((glint_g as f32 * lit0).min(255.0)) as u32;
            let b_cr0 = ((glint_b as f32 * lit0).min(255.0)) as u32;

            let r_cr1 = ((glint_r as f32 * lit1).min(255.0)) as u32;
            let g_cr1 = ((glint_g as f32 * lit1).min(255.0)) as u32;
            let b_cr1 = ((glint_b as f32 * lit1).min(255.0)) as u32;

            let col_crest0 = (a_cr0_u32 << 24) | (b_cr0 << 16) | (g_cr0 << 8) | r_cr0;
            let col_crest1 = (a_cr1_u32 << 24) | (b_cr1 << 16) | (g_cr1 << 8) | r_cr1;

            // Outer Kelvin V-arm width coordinates:
            let w_in0 = (hw0 - half_w0).max(0.03);
            let w_in1 = (hw1 - half_w1).max(0.03);
            let w_cr0 = hw0;
            let w_cr1 = hw1;
            let w_out0 = hw0 + half_w0;
            let w_out1 = hw1 + half_w1;

            // --- Left V-Arm (Inner -> Crest -> Outer) ---
            let l_in0 = [base_x0 + n0.left_x * w_in0, y_base0, base_z0 + n0.left_z * w_in0];
            let l_in1 = [base_x1 + n1.left_x * w_in1, y_base1, base_z1 + n1.left_z * w_in1];
            let l_cr0 = [base_x0 + n0.left_x * w_cr0, y_crest0, base_z0 + n0.left_z * w_cr0];
            let l_cr1 = [base_x1 + n1.left_x * w_cr1, y_crest1, base_z1 + n1.left_z * w_cr1];
            let l_out0 = [base_x0 + n0.left_x * w_out0, y_base0, base_z0 + n0.left_z * w_out0];
            let l_out1 = [base_x1 + n1.left_x * w_out1, y_base1, base_z1 + n1.left_z * w_out1];

            // 1. Left inner slope (in -> cr)
            out.push(Vertex { pos: l_in0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_in1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // 2. Left outer slope (cr -> out)
            out.push(Vertex { pos: l_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_out1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_out0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // --- Right V-Arm (Inner -> Crest -> Outer) ---
            let r_in0 = [base_x0 - n0.left_x * w_in0, y_base0, base_z0 - n0.left_z * w_in0];
            let r_in1 = [base_x1 - n1.left_x * w_in1, y_base1, base_z1 - n1.left_z * w_in1];
            let r_cr0 = [base_x0 - n0.left_x * w_cr0, y_crest0, base_z0 - n0.left_z * w_cr0];
            let r_cr1 = [base_x1 - n1.left_x * w_cr1, y_crest1, base_z1 - n1.left_z * w_cr1];
            let r_out0 = [base_x0 - n0.left_x * w_out0, y_base0, base_z0 - n0.left_z * w_out0];
            let r_out1 = [base_x1 - n1.left_x * w_out1, y_base1, base_z1 - n1.left_z * w_out1];

            // 1. Right inner slope (in -> cr)
            out.push(Vertex { pos: r_in0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_in1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // 2. Right outer slope (cr -> out)
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // -------------------------------------------------------------
            // Boat Hydrodynamic Wake Field: Stern Wash & Transverse Wavelets
            // -------------------------------------------------------------
            if n0.is_boat {
                // Central Stern Wash (hull keel trail):
                // Trailing directly behind the hull along the keel centerline (w=0)
                let hw_wash0 = ((0.32 + 0.08 * n0.age).clamp(0.35, 0.55)).min(w_in0);
                let hw_wash1 = ((0.32 + 0.08 * n1.age).clamp(0.35, 0.55)).min(w_in1);

                let crest_keel0 = (0.025 * amp0).min(0.035);
                let crest_keel1 = (0.025 * amp1).min(0.035);
                let y_keel0 = y_base0 + 0.001 + crest_keel0;
                let y_keel1 = y_base1 + 0.001 + crest_keel1;

                let alpha_keel0 = (0.13 * amp0).clamp(0.0, 0.16);
                let alpha_keel1 = (0.13 * amp1).clamp(0.0, 0.16);
                let a_keel0_u32 = (alpha_keel0 * 255.0) as u32;
                let a_keel1_u32 = (alpha_keel1 * 255.0) as u32;

                let col_keel0 = (a_keel0_u32 << 24) | (b_cr0 << 16) | (g_cr0 << 8) | r_cr0;
                let col_keel1 = (a_keel1_u32 << 24) | (b_cr1 << 16) | (g_cr1 << 8) | r_cr1;

                // Center keel ridge:
                let keel0 = [base_x0, y_keel0, base_z0];
                let keel1 = [base_x1, y_keel1, base_z1];

                // Left & right wash boundaries:
                let w_l0 = [base_x0 + n0.left_x * hw_wash0, y_base0, base_z0 + n0.left_z * hw_wash0];
                let w_l1 = [base_x1 + n1.left_x * hw_wash1, y_base1, base_z1 + n1.left_z * hw_wash1];
                let w_r0 = [base_x0 - n0.left_x * hw_wash0, y_base0, base_z0 - n0.left_z * hw_wash0];
                let w_r1 = [base_x1 - n1.left_x * hw_wash1, y_base1, base_z1 - n1.left_z * hw_wash1];

                // Hull wash left slope (left wash edge -> keel center):
                out.push(Vertex { pos: w_l0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: w_l1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: keel1, uv: [u_white, v_white], layer, color: col_keel1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: keel0, uv: [u_white, v_white], layer, color: col_keel0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                // Hull wash right slope (keel center -> right wash edge):
                out.push(Vertex { pos: keel0, uv: [u_white, v_white], layer, color: col_keel0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: keel1, uv: [u_white, v_white], layer, color: col_keel1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: w_r1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                out.push(Vertex { pos: w_r0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                // Transverse wave bridges:
                // When outer arms have spread past the hull wash, transverse wavelets bridge the gap
                if w_in0 > hw_wash0 + 0.06 && w_in1 > hw_wash1 + 0.06 {
                    let w_mid0 = (hw_wash0 + w_in0) * 0.5;
                    let w_mid1 = (hw_wash1 + w_in1) * 0.5;

                    let crest_tr0 = (0.020 * amp0).min(0.030);
                    let crest_tr1 = (0.020 * amp1).min(0.030);
                    let y_tr0 = y_base0 + 0.001 + crest_tr0;
                    let y_tr1 = y_base1 + 0.001 + crest_tr1;

                    let alpha_tr0 = (0.10 * amp0).clamp(0.0, 0.13);
                    let alpha_tr1 = (0.10 * amp1).clamp(0.0, 0.13);
                    let a_tr0_u32 = (alpha_tr0 * 255.0) as u32;
                    let a_tr1_u32 = (alpha_tr1 * 255.0) as u32;

                    let col_tr0 = (a_tr0_u32 << 24) | (b_cr0 << 16) | (g_cr0 << 8) | r_cr0;
                    let col_tr1 = (a_tr1_u32 << 24) | (b_cr1 << 16) | (g_cr1 << 8) | r_cr1;

                    // Left transverse crest:
                    let tr_l0 = [base_x0 + n0.left_x * w_mid0, y_tr0, base_z0 + n0.left_z * w_mid0];
                    let tr_l1 = [base_x1 + n1.left_x * w_mid1, y_tr1, base_z1 + n1.left_z * w_mid1];

                    // Left transverse inner slope (wash edge -> transverse crest):
                    out.push(Vertex { pos: w_l0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: w_l1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_l1, uv: [u_white, v_white], layer, color: col_tr1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_l0, uv: [u_white, v_white], layer, color: col_tr0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                    // Left transverse outer slope (transverse crest -> outer arm inner edge):
                    out.push(Vertex { pos: tr_l0, uv: [u_white, v_white], layer, color: col_tr0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_l1, uv: [u_white, v_white], layer, color: col_tr1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: l_in1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: l_in0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                    // Right transverse crest:
                    let tr_r0 = [base_x0 - n0.left_x * w_mid0, y_tr0, base_z0 - n0.left_z * w_mid0];
                    let tr_r1 = [base_x1 - n1.left_x * w_mid1, y_tr1, base_z1 - n1.left_z * w_mid1];

                    // Right transverse outer slope (outer arm inner edge -> transverse crest):
                    out.push(Vertex { pos: r_in0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: r_in1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_r1, uv: [u_white, v_white], layer, color: col_tr1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_r0, uv: [u_white, v_white], layer, color: col_tr0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

                    // Right transverse inner slope (transverse crest -> wash edge):
                    out.push(Vertex { pos: tr_r0, uv: [u_white, v_white], layer, color: col_tr0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: tr_r1, uv: [u_white, v_white], layer, color: col_tr1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: w_r1, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: w_r0, uv: [u_white, v_white], layer, color: col_edge, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                }
            }
        }
    }
}

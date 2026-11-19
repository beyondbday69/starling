// Geometric water wave and wake renderer (v3.1).
//
// Key features:
// - Circular ripple rings: Smooth 24-gon expanding torus quads with raised 3D crests (+0.02m to +0.04m).
//   Zero-pop envelope ensures perfectly smooth attack and cubic decay to zero alpha and zero height.
// - V-Wake ribbons: True continuous Kelvin wake arms flaring at 19.47 degrees behind swimmers and boats.
//   Guarded by continuity distance and direction checks; zero artificial center white strip.
// - Soft glass water reflection with zero-alpha edges for 100% seamless blending.

use crate::game::Game;
use super::native_water_physics_sim as sim;
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
    cache: &mut RenderCache,
    game: &Game<'_>,
    camera: glam::DVec3,
    out: &mut Vec<Vertex>,
) {
    game.render_wave_overlays(sim, cache, camera, out);
}

impl Game<'_> {
    pub(super) fn render_wave_overlays(
        &self,
        sim: &sim::WaterSimulation,
        _cache: &mut RenderCache,
        camera: glam::DVec3,
        out: &mut Vec<Vertex>,
    ) {
        if !sim.any_active() {
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
        let sun_y = 0.88f32;
        let sun_z = -0.38f32;

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
            let width = (0.14 + 0.16 * ring.radius).clamp(0.18, 0.50);
            let r_inner = (r_crest - width * 0.5).max(0.02);
            let r_outer = r_crest + width * 0.5;

            // Physical 3D crest displacement: elevates smoothly up to +0.038m
            let crest_h = (0.038 * amp).min(0.055);
            let y_base = ring.water_y as f32 + 0.002 + cam_y;
            let y_crest = ring.water_y as f32 + 0.003 + crest_h + cam_y;

            let base_cam_x = (ring.center_x - camera.x) as f32;
            let base_cam_z = (ring.center_z - camera.z) as f32;

            let alpha_crest = (0.38 * amp).clamp(0.0, 0.45);
            let a_crest_u32 = (alpha_crest * 255.0) as u32;

            for k in 0..segments {
                let th0 = (k as f32) * step_rad;
                let th1 = ((k + 1) as f32) * step_rad;

                let c0 = th0.cos();
                let s0 = th0.sin();
                let c1 = th1.cos();
                let s1 = th1.sin();

                // Surface normal lighting on wave crest
                let norm_dot = (c0 * sun_x + 0.92 * sun_y + s0 * sun_z).clamp(0.0, 1.0);
                let lit = 0.70 + 0.30 * norm_dot;

                // Crisp transparent water glint reflection
                let r_col = ((220.0 * lit).min(255.0)) as u32;
                let g_col = ((240.0 * lit).min(255.0)) as u32;
                let b_col = 255u32;

                let color_crest = (a_crest_u32 << 24) | (b_col << 16) | (g_col << 8) | r_col;
                let color_edge = (b_col << 16) | (g_col << 8) | r_col; // alpha = 0 for seamless blend

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
        // 2. Render Continuous Kelvin V-Wakes (Player and Boat ribbons)
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
            if dist_sq > 1.25 * 1.25 || dist_sq < 0.0001 {
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

            if amp0 < 0.008 && amp1 < 0.008 {
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

            let crest_h0 = (0.035 * amp0).min(0.055);
            let crest_h1 = (0.035 * amp1).min(0.055);
            let y_crest0 = n0.water_y as f32 + 0.003 + crest_h0 + cam_y;
            let y_crest1 = n1.water_y as f32 + 0.003 + crest_h1 + cam_y;

            let arm_w0 = (0.24 + 0.10 * n0.half_width).min(0.55);
            let arm_w1 = (0.24 + 0.10 * n1.half_width).min(0.55);

            // Shading & Alpha: soft glass reflection, zero opaque white
            let alpha0 = (0.34 * amp0).clamp(0.0, 0.42);
            let alpha1 = (0.34 * amp1).clamp(0.0, 0.42);
            let a0_u32 = (alpha0 * 255.0) as u32;
            let a1_u32 = (alpha1 * 255.0) as u32;

            let (r0, g0, b0) = (215u32, 235u32, 255u32);
            let (r1, g1, b1) = (215u32, 235u32, 255u32);

            let col_crest0 = (a0_u32 << 24) | (b0 << 16) | (g0 << 8) | r0;
            let col_crest1 = (a1_u32 << 24) | (b1 << 16) | (g1 << 8) | r1;
            let col_edge0 = (b0 << 16) | (g0 << 8) | r0; // alpha = 0 for seamless blend
            let col_edge1 = (b1 << 16) | (g1 << 8) | r1; // alpha = 0 for seamless blend

            // --- Left V-Arm Ribbon ---
            let w_left_cr0 = n0.half_width;
            let w_left_cr1 = n1.half_width;
            let w_left_in0 = (w_left_cr0 - arm_w0 * 0.5).max(0.04);
            let w_left_in1 = (w_left_cr1 - arm_w1 * 0.5).max(0.04);
            let w_left_out0 = w_left_cr0 + arm_w0 * 0.5;
            let w_left_out1 = w_left_cr1 + arm_w1 * 0.5;

            let l_in0 = [base_x0 + n0.left_x * w_left_in0, y_base0, base_z0 + n0.left_z * w_left_in0];
            let l_in1 = [base_x1 + n1.left_x * w_left_in1, y_base1, base_z1 + n1.left_z * w_left_in1];
            let l_cr0 = [base_x0 + n0.left_x * w_left_cr0, y_crest0, base_z0 + n0.left_z * w_left_cr0];
            let l_cr1 = [base_x1 + n1.left_x * w_left_cr1, y_crest1, base_z1 + n1.left_z * w_left_cr1];
            let l_out0 = [base_x0 + n0.left_x * w_left_out0, y_base0, base_z0 + n0.left_z * w_left_out0];
            let l_out1 = [base_x1 + n1.left_x * w_left_out1, y_base1, base_z1 + n1.left_z * w_left_out1];

            // Left inner slope (in -> cr)
            out.push(Vertex { pos: l_in0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_in1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // Left outer slope (cr -> out)
            out.push(Vertex { pos: l_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_out1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_out0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // --- Right V-Arm Ribbon ---
            let r_in0 = [base_x0 - n0.left_x * w_left_in0, y_base0, base_z0 - n0.left_z * w_left_in0];
            let r_in1 = [base_x1 - n1.left_x * w_left_in1, y_base1, base_z1 - n1.left_z * w_left_in1];
            let r_cr0 = [base_x0 - n0.left_x * w_left_cr0, y_crest0, base_z0 - n0.left_z * w_left_cr0];
            let r_cr1 = [base_x1 - n1.left_x * w_left_cr1, y_crest1, base_z1 - n1.left_z * w_left_cr1];
            let r_out0 = [base_x0 - n0.left_x * w_left_out0, y_base0, base_z0 - n0.left_z * w_left_out0];
            let r_out1 = [base_x1 - n1.left_x * w_left_out1, y_base1, base_z1 - n1.left_z * w_left_out1];

            // Right inner slope (in -> cr)
            out.push(Vertex { pos: r_in0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_in1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // Right outer slope (cr -> out)
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
        }
    }
}

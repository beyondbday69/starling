// Geometric water wave and wake renderer (v3.0).
//
// Renders smooth analytical fluid geometry:
// - Circular ripple rings: Smooth 24-gon expanding torus quads with raised 3D crests (+0.02m to +0.05m),
//   bright specular sun highlights, and zero-alpha inner/outer edges for 100% seamless blending.
// - V-Wake ribbons: True continuous Kelvin wake arms flaring at 19.47 degrees behind swimmers and boats,
//   with aerated foam crests and a central churning trail.
// - Zero square grid artifacts, zero low-poly creases, zero opaque blobs.

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

            let bx = ring.center_x.floor() as i32;
            let by = ring.water_y.floor() as i32;
            let bz = ring.center_z.floor() as i32;
            let light_packed = self.world.light(bx, by + 1, bz);
            let light = horizon_client::block_entity_render::vertex_light(light_packed);

            let progress = (ring.age / ring.max_age).clamp(0.0, 1.0);
            let fade = (1.0 - progress) * (1.0 - progress);
            let amp = ring.amplitude * fade;

            if amp < 0.01 {
                continue;
            }

            // Ring geometry parameters
            let r_crest = ring.radius;
            let width = (0.24 + 0.16 * ring.radius).min(0.60);
            let r_inner = (r_crest - width * 0.5).max(0.01);
            let r_outer = r_crest + width * 0.5;

            // Physical 3D crest displacement: elevates smoothly up to +0.05m
            let crest_h = (0.045 * amp).min(0.065);
            let y_base = ring.water_y as f32 + 0.002 + cam_y;
            let y_crest = ring.water_y as f32 + 0.003 + crest_h + cam_y;

            let base_cam_x = (ring.center_x - camera.x) as f32;
            let base_cam_z = (ring.center_z - camera.z) as f32;

            let alpha_crest = (0.55 * amp).clamp(0.0, 0.65);
            let a_crest_u32 = (alpha_crest * 255.0) as u32;

            for k in 0..segments {
                let th0 = (k as f32) * step_rad;
                let th1 = ((k + 1) as f32) * step_rad;

                let c0 = th0.cos();
                let s0 = th0.sin();
                let c1 = th1.cos();
                let s1 = th1.sin();

                // Surface normal on wave crest
                let norm_dot = (c0 * sun_x + 0.92 * sun_y + s0 * sun_z).clamp(0.0, 1.0);
                let lit = 0.70 + 0.30 * norm_dot;

                let (r_col, g_col, b_col) = if ring.has_foam && progress < 0.45 {
                    // Brilliant white aerated foam on new impact rims
                    (255u32, 255u32, 255u32)
                } else {
                    // Crisp water glint reflection
                    (((230.0 * lit).min(255.0)) as u32, ((245.0 * lit).min(255.0)) as u32, 255u32)
                };

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
        // 2. Render Continuous Kelvin V-Wakes (Smooth trailing ribbons)
        // -------------------------------------------------------------
        if sim.wake_nodes.len() < 2 {
            return;
        }

        for i in 0..sim.wake_nodes.len() - 1 {
            let n0 = &sim.wake_nodes[i];
            let n1 = &sim.wake_nodes[i + 1];

            let p0 = (n0.age / n0.max_age).clamp(0.0, 1.0);
            let p1 = (n1.age / n1.max_age).clamp(0.0, 1.0);
            let fade0 = (1.0 - p0) * (1.0 - p0);
            let fade1 = (1.0 - p1) * (1.0 - p1);

            let amp0 = n0.amplitude * fade0;
            let amp1 = n1.amplitude * fade1;

            if amp0 < 0.01 && amp1 < 0.01 {
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

            let crest_h0 = (0.040 * amp0).min(0.065);
            let crest_h1 = (0.040 * amp1).min(0.065);
            let y_crest0 = n0.water_y as f32 + 0.003 + crest_h0 + cam_y;
            let y_crest1 = n1.water_y as f32 + 0.003 + crest_h1 + cam_y;

            let arm_w0 = (0.28 + 0.12 * n0.half_width).min(0.65);
            let arm_w1 = (0.28 + 0.12 * n1.half_width).min(0.65);

            // Shading & Alpha
            let alpha0 = (0.58 * amp0).clamp(0.0, 0.70);
            let alpha1 = (0.58 * amp1).clamp(0.0, 0.70);
            let a0_u32 = (alpha0 * 255.0) as u32;
            let a1_u32 = (alpha1 * 255.0) as u32;

            let (r0, g0, b0) = if n0.is_boat { (255u32, 255u32, 255u32) } else { (225u32, 245u32, 255u32) };
            let (r1, g1, b1) = if n1.is_boat { (255u32, 255u32, 255u32) } else { (225u32, 245u32, 255u32) };

            let col_crest0 = (a0_u32 << 24) | (b0 << 16) | (g0 << 8) | r0;
            let col_crest1 = (a1_u32 << 24) | (b1 << 16) | (g1 << 8) | r1;
            let col_edge0 = (b0 << 16) | (g0 << 8) | r0; // alpha = 0
            let col_edge1 = (b1 << 16) | (g1 << 8) | r1; // alpha = 0

            // --- Left V-Arm Ribbon ---
            let w_left_cr0 = n0.half_width;
            let w_left_cr1 = n1.half_width;
            let w_left_in0 = (w_left_cr0 - arm_w0 * 0.5).max(0.05);
            let w_left_in1 = (w_left_cr1 - arm_w1 * 0.5).max(0.05);
            let w_left_out0 = w_left_cr0 + arm_w0 * 0.5;
            let w_left_out1 = w_left_cr1 + arm_w1 * 0.5;

            let l_in0 = [base_x0 + n0.left_x * w_left_in0, y_base0, base_z0 + n0.left_z * w_left_in0];
            let l_in1 = [base_x1 + n1.left_x * w_left_in1, y_base1, base_z1 + n1.left_z * w_left_in1];
            let l_cr0 = [base_x0 + n0.left_x * w_left_cr0, y_crest0, base_z0 + n0.left_z * w_left_cr0];
            let l_cr1 = [base_x1 + n1.left_x * w_left_cr1, y_crest1, base_z1 + n1.left_z * w_left_cr1];
            let l_out0 = [base_x0 + n0.left_x * w_left_out0, y_base0, base_z0 + n0.left_z * w_left_out0];
            let l_out1 = [base_x1 + n1.left_x * w_left_out1, y_base1, base_z1 + n1.left_z * w_left_out1];

            // Left inner slope
            out.push(Vertex { pos: l_in0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_in1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: l_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // Left outer slope
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

            // Right inner slope
            out.push(Vertex { pos: r_in0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_in1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // Right outer slope
            out.push(Vertex { pos: r_cr0, uv: [u_white, v_white], layer, color: col_crest0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_cr1, uv: [u_white, v_white], layer, color: col_crest1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out1, uv: [u_white, v_white], layer, color: col_edge1, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
            out.push(Vertex { pos: r_out0, uv: [u_white, v_white], layer, color: col_edge0, light, normal: [0; 4], glint_uv: [0.0, 0.0] });

            // --- Center Churning Foam Trail (for boats and swimmers) ---
            if p0 < 0.65 {
                let foam_a = ((0.45 * amp0 * (1.0 - p0 / 0.65)).min(0.60) * 255.0) as u32;
                if foam_a > 3 {
                    let col_foam = (foam_a << 24) | (255u32 << 16) | (255u32 << 8) | 255u32;
                    let fw0 = (0.25 + 0.15 * n0.half_width).min(0.55);
                    let fw1 = (0.25 + 0.15 * n1.half_width).min(0.55);

                    let f_l0 = [base_x0 + n0.left_x * fw0, y_base0 + 0.001, base_z0 + n0.left_z * fw0];
                    let f_l1 = [base_x1 + n1.left_x * fw1, y_base1 + 0.001, base_z1 + n1.left_z * fw1];
                    let f_r0 = [base_x0 - n0.left_x * fw0, y_base0 + 0.001, base_z0 - n0.left_z * fw0];
                    let f_r1 = [base_x1 - n1.left_x * fw1, y_base1 + 0.001, base_z1 - n1.left_z * fw1];

                    out.push(Vertex { pos: f_l0, uv: [u_white, v_white], layer, color: col_foam, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: f_l1, uv: [u_white, v_white], layer, color: col_foam, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: f_r1, uv: [u_white, v_white], layer, color: col_foam, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                    out.push(Vertex { pos: f_r0, uv: [u_white, v_white], layer, color: col_foam, light, normal: [0; 4], glint_uv: [0.0, 0.0] });
                }
            }
        }
    }
}

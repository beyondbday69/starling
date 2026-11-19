// Water wave overlay renderer (v2).
//
// Generates a smooth 3D displaced mesh for active wave tiles:
// - Vertex y = water_surface + baseline_lift + h * amplitude
// - Elevated baseline prevents troughs from clipping into flat vanilla water
// - Smooth envelope fades displacement and alpha to 0 at edges and shoreline
// - Preallocated quad buffer cached at 30Hz simulation steps; zero per-frame allocation
// - Cheap directional diffuse lighting (no heavy CPU specular loops)

use crate::game::Game;
use super::native_water_physics_sim as sim;
use horizon_client::mesher::Vertex;

#[derive(Clone, Copy, Default)]
pub struct CachedQuad {
    pub center_x: f64,
    pub center_z: f64,
    pub p00: [f32; 3],
    pub p01: [f32; 3],
    pub p11: [f32; 3],
    pub p10: [f32; 3],
    pub c00: u32,
    pub c01: u32,
    pub c11: u32,
    pub c10: u32,
    pub light: u32,
}

pub struct RenderCache {
    pub quads: Vec<CachedQuad>,
    pub node_y: Vec<f32>,
    pub node_color: Vec<u32>,
    pub node_weight: Vec<f32>,
    pub dirty: bool,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            quads: Vec::with_capacity(1024),
            node_y: Vec::with_capacity(1024),
            node_color: Vec::with_capacity(1024),
            node_weight: Vec::with_capacity(1024),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.quads.clear();
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
    pub(super) fn rebuild_wave_mesh(
        &self,
        sim: &sim::WaterSimulation,
        cache: &mut RenderCache,
    ) {
        cache.quads.clear();
        cache.dirty = false;

        let sun_x = 0.24f32;
        let sun_y = 0.88f32;
        let sun_z = -0.40f32;

        let base_lift = 0.050f32;
        let ripple_scale = 0.075f32 * sim.intensity;

        for tile in &sim.tiles {
            if !tile.active {
                continue;
            }

            let res = tile.res;
            let total_nodes = res * res;
            let dx = sim::TILE_SIZE / (res as f32);
            let half_size = (sim::TILE_SIZE * 0.5) as f32;
            let water_y = tile.water_y as f32;
            let block_y = (tile.water_y - 0.4).floor() as i32;

            cache.node_y.resize(total_nodes, 0.0);
            cache.node_color.resize(total_nodes, 0);
            cache.node_weight.resize(total_nodes, 0.0);
            cache.node_y.fill(0.0);
            cache.node_color.fill(0);
            cache.node_weight.fill(0.0);

            for j in 0..res {
                let row = j * res;
                let rel_z = -half_size + (j as f32) * dx;
                let wz = tile.center_z + rel_z as f64;
                let bz = wz.floor() as i32;

                for i in 0..res {
                    let idx = row + i;
                    let rel_x = -half_size + (i as f32) * dx;
                    let wx = tile.center_x + rel_x as f64;
                    let bx = wx.floor() as i32;

                    // Water presence check
                    let is_water = self.wp_is_water(bx, block_y, bz);
                    if !is_water {
                        continue;
                    }

                    let h_val = tile.h[idx];
                    let foam_val = tile.foam[idx];

                    // Boundary fade to zero on tile edges
                    let edge_dist = (i.min(j).min(res - 1 - i).min(res - 1 - j)) as f32;
                    let b_fade = (edge_dist / 3.0).min(1.0);

                    // Activity envelope
                    let activity = (h_val.abs() * 6.0 + foam_val * 2.5).min(1.0);
                    let w = b_fade * activity;
                    if w < 0.005 {
                        continue;
                    }

                    cache.node_weight[idx] = w;

                    // 3D displaced surface elevation:
                    // base_lift ensures troughs stay above vanilla flat water top
                    let y = water_y + 0.002 + w * (base_lift + h_val * ripple_scale);
                    cache.node_y[idx] = y;

                    // Central-difference slope for per-vertex normal
                    let i_prev = if i > 0 { i - 1 } else { 0 };
                    let i_next = if i + 1 < res { i + 1 } else { res - 1 };
                    let j_prev = if j > 0 { j - 1 } else { 0 };
                    let j_next = if j + 1 < res { j + 1 } else { res - 1 };

                    let step_x = ((i_next - i_prev) as f32) * dx;
                    let step_z = ((j_next - j_prev) as f32) * dx;
                    let dh_dx = (tile.h[row + i_next] - tile.h[row + i_prev]) / step_x.max(0.001);
                    let dh_dz = (tile.h[j_next * res + i] - tile.h[j_prev * res + i]) / step_z.max(0.001);

                    let nx = -dh_dx * ripple_scale;
                    let nz = -dh_dz * ripple_scale;
                    let inv_len = 1.0 / (nx * nx + 1.0 + nz * nz).sqrt();
                    let norm_x = nx * inv_len;
                    let norm_y = inv_len;
                    let norm_z = nz * inv_len;

                    // Fast directional lighting (single dot product, no CPU specular loop)
                    let dot = (norm_x * sun_x + norm_y * sun_y + norm_z * sun_z).max(0.0);
                    let lit = 0.70 + 0.30 * dot;

                    // Glass-like water (v2.1): the overlay has no body colour of its own, the real water
                    // underneath (vanilla or a shader pack such as Reimagined) already supplies tint,
                    // reflections and depth. Only a soft light crest, a soft dark trough and a little
                    // foam are painted, each at low alpha, so the real water shows through.
                    let crest = (h_val * 5.0).clamp(0.0, 1.0);
                    let trough = (-h_val * 5.0).clamp(0.0, 1.0);
                    let f = foam_val.clamp(0.0, 1.0);

                    let r = (((200.0 + 45.0 * crest - 150.0 * trough) * (1.0 - f) + 255.0 * f) * lit).clamp(0.0, 255.0);
                    let g = (((225.0 + 25.0 * crest - 150.0 * trough) * (1.0 - f) + 255.0 * f) * lit).clamp(0.0, 255.0);
                    let b = (((245.0 + 10.0 * crest - 110.0 * trough) * (1.0 - f) + 255.0 * f) * lit).clamp(0.0, 255.0);
                    let a = (w * (0.03 + 0.22 * crest + 0.12 * trough + 0.40 * f)).clamp(0.0, 0.45);

                    cache.node_color[idx] = ((a * 255.0) as u32) << 24
                        | ((b as u32) << 16)
                        | ((g as u32) << 8)
                        | (r as u32);
                }
            }

            // Emit active displaced quads
            for j in 0..res - 1 {
                let row = j * res;
                let row_next = (j + 1) * res;

                let rel_z0 = -half_size + (j as f32) * dx;
                let rel_z1 = rel_z0 + dx;

                for i in 0..res - 1 {
                    let idx00 = row + i;
                    let idx10 = row + (i + 1);
                    let idx01 = row_next + i;
                    let idx11 = row_next + (i + 1);

                    let w00 = cache.node_weight[idx00];
                    let w10 = cache.node_weight[idx10];
                    let w01 = cache.node_weight[idx01];
                    let w11 = cache.node_weight[idx11];

                    if w00 < 0.005 && w10 < 0.005 && w01 < 0.005 && w11 < 0.005 {
                        continue;
                    }

                    let rel_x0 = -half_size + (i as f32) * dx;
                    let rel_x1 = rel_x0 + dx;

                    let p00 = [rel_x0, cache.node_y[idx00], rel_z0];
                    let p01 = [rel_x0, cache.node_y[idx01], rel_z1];
                    let p11 = [rel_x1, cache.node_y[idx11], rel_z1];
                    let p10 = [rel_x1, cache.node_y[idx10], rel_z0];

                    let block_x = (tile.center_x + (rel_x0 + rel_x1) as f64 * 0.5).floor() as i32;
                    let block_z = (tile.center_z + (rel_z0 + rel_z1) as f64 * 0.5).floor() as i32;
                    let light_packed = self.world.light(block_x, block_y + 1, block_z);
                    let light = horizon_client::block_entity_render::vertex_light(light_packed);

                    cache.quads.push(CachedQuad {
                        center_x: tile.center_x,
                        center_z: tile.center_z,
                        p00,
                        p01,
                        p11,
                        p10,
                        c00: cache.node_color[idx00],
                        c01: cache.node_color[idx01],
                        c11: cache.node_color[idx11],
                        c10: cache.node_color[idx10],
                        light,
                    });
                }
            }
        }
    }

    pub(super) fn render_wave_overlays(
        &self,
        sim: &sim::WaterSimulation,
        cache: &mut RenderCache,
        camera: glam::DVec3,
        out: &mut Vec<Vertex>,
    ) {
        if cache.dirty {
            self.rebuild_wave_mesh(sim, cache);
        }

        if cache.quads.is_empty() {
            return;
        }

        let (u_white, v_white) = if let Some(r) = self.gui.atlas.sprites.get("horizon/white") {
            ((r.u0 + r.u1) * 0.5, (r.v0 + r.v1) * 0.5)
        } else {
            (0.0, 0.0)
        };

        let layer = horizon_client::gfx::TRANSLUCENT_PARTICLE_OVERLAY | horizon_client::gfx::OIT_ENTITY;
        let cam_y = -camera.y as f32;

        for q in &cache.quads {
            let base_cam_x = (q.center_x - camera.x) as f32;
            let base_cam_z = (q.center_z - camera.z) as f32;

            out.push(Vertex {
                pos: [base_cam_x + q.p00[0], q.p00[1] + cam_y, base_cam_z + q.p00[2]],
                uv: [u_white, v_white],
                layer,
                color: q.c00,
                light: q.light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            });
            out.push(Vertex {
                pos: [base_cam_x + q.p01[0], q.p01[1] + cam_y, base_cam_z + q.p01[2]],
                uv: [u_white, v_white],
                layer,
                color: q.c01,
                light: q.light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            });
            out.push(Vertex {
                pos: [base_cam_x + q.p11[0], q.p11[1] + cam_y, base_cam_z + q.p11[2]],
                uv: [u_white, v_white],
                layer,
                color: q.c11,
                light: q.light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            });
            out.push(Vertex {
                pos: [base_cam_x + q.p10[0], q.p10[1] + cam_y, base_cam_z + q.p10[2]],
                uv: [u_white, v_white],
                layer,
                color: q.c10,
                light: q.light,
                normal: [0; 4],
                glint_uv: [0.0, 0.0],
            });
        }
    }
}

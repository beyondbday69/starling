// Water wave overlay renderer.
//
// Generates translucent mesh quads at the water surface for active wave tiles,
// shaded using heightfield surface normals, specular highlights, and foam blending.

use crate::game::Game;
use super::native_water_physics_sim as sim;
use horizon_client::mesher::Vertex;

impl Game<'_> {
    pub(super) fn render_wave_overlays(
        &self,
        sim: &sim::WaterSimulation,
        camera: glam::DVec3,
        out: &mut Vec<Vertex>,
    ) {
        let (u_white, v_white) = if let Some(r) = self.gui.atlas.sprites.get("horizon/white") {
            ((r.u0 + r.u1) * 0.5, (r.v0 + r.v1) * 0.5)
        } else {
            (0.0, 0.0)
        };

        let layer = horizon_client::gfx::TRANSLUCENT_PARTICLE_OVERLAY | horizon_client::gfx::OIT_ENTITY;
        let sun_x = 0.161f32;
        let sun_y = 0.803f32;
        let sun_z = -0.562f32;

        for tile in &sim.tiles {
            if !tile.active {
                continue;
            }

            let dist_sq = (tile.center_x - camera.x).powi(2) + (tile.center_z - camera.z).powi(2);
            if dist_sq > 64.0 * 64.0 {
                continue;
            }

            let res = tile.res;
            let dx = sim::TILE_SIZE / (res as f32);
            let origin_x = tile.center_x - (sim::TILE_SIZE * 0.5) as f64;
            let origin_z = tile.center_z - (sim::TILE_SIZE * 0.5) as f64;
            let base_y = tile.water_y + 0.003;

            for j in 0..res - 1 {
                let row = j * res;
                let row_next = (j + 1) * res;

                let z0 = origin_z + (j as f64) * (dx as f64);
                let z1 = z0 + (dx as f64);

                for i in 0..res - 1 {
                    let idx00 = row + i;
                    let idx10 = row + (i + 1);
                    let idx01 = row_next + i;
                    let idx11 = row_next + (i + 1);

                    let h00 = tile.h[idx00];
                    let h10 = tile.h[idx10];
                    let h01 = tile.h[idx01];
                    let h11 = tile.h[idx11];

                    let f00 = tile.foam[idx00];
                    let f10 = tile.foam[idx10];
                    let f01 = tile.foam[idx01];
                    let f11 = tile.foam[idx11];

                    // Fast culling of undisturbed flat water
                    let max_h = h00.abs().max(h10.abs()).max(h01.abs()).max(h11.abs());
                    let max_f = f00.max(f10).max(f01).max(f11);
                    if max_h < 0.001 && max_f < 0.01 {
                        continue;
                    }

                    let x0 = origin_x + (i as f64) * (dx as f64);
                    let x1 = x0 + (dx as f64);

                    // Check that cell center sits above a water block
                    let block_x = ((x0 + x1) * 0.5).floor() as i32;
                    let block_z = ((z0 + z1) * 0.5).floor() as i32;
                    let block_y = (tile.water_y - 0.4).floor() as i32;

                    let state = self.world.block(block_x, block_y, block_z);
                    if self.res.facts().get(state).fluid != horizon_items::blocks::Fluid::Water {
                        continue;
                    }

                    // Compute normal and slope
                    let dh_dx = (h10 - h00) / dx;
                    let dh_dz = (h01 - h00) / dx;
                    let inv_len = 1.0 / (dh_dx * dh_dx + 1.0 + dh_dz * dh_dz).sqrt();
                    let nx = -dh_dx * inv_len;
                    let ny = inv_len;
                    let nz = -dh_dz * inv_len;

                    // View vector for specular highlight
                    let cell_wx = ((x0 + x1) * 0.5) as f32;
                    let cell_wz = ((z0 + z1) * 0.5) as f32;
                    let mut vx = (camera.x as f32) - cell_wx;
                    let mut vy = (camera.y as f32) - (base_y as f32);
                    let mut vz = (camera.z as f32) - cell_wz;
                    let v_len = (vx * vx + vy * vy + vz * vz).sqrt().max(0.001);
                    vx /= v_len;
                    vy /= v_len;
                    vz /= v_len;

                    // Halfway vector
                    let mut hx = sun_x + vx;
                    let mut hy = sun_y + vy;
                    let mut hz = sun_z + vz;
                    let h_len = (hx * hx + hy * hy + hz * hz).sqrt().max(0.001);
                    hx /= h_len;
                    hy /= h_len;
                    hz /= h_len;

                    let ndoth = (nx * hx + ny * hy + nz * hz).max(0.0);
                    let spec4 = ndoth * ndoth * ndoth * ndoth;
                    let spec16 = spec4 * spec4 * spec4 * spec4;
                    let slope = (dh_dx * dh_dx + dh_dz * dh_dz).sqrt();

                    // Tint and foam shading
                    let f = max_f.clamp(0.0, 1.0);
                    let r = ((0.28 + 0.55 * spec16 + 0.65 * f) * 255.0).clamp(0.0, 255.0) as u32;
                    let g = ((0.52 + 0.40 * spec16 + 0.45 * f) * 255.0).clamp(0.0, 255.0) as u32;
                    let b = ((0.78 + 0.20 * spec16 + 0.20 * f) * 255.0).clamp(0.0, 255.0) as u32;
                    let a = ((0.08 + 1.5 * slope + 0.70 * f).clamp(0.05, 0.88) * 255.0) as u32;
                    let color = (a << 24) | (b << 16) | (g << 8) | r;

                    let light_packed = self.world.light(block_x, block_y + 1, block_z);
                    let light = horizon_client::block_entity_render::vertex_light(light_packed);

                    let y00 = base_y + (h00 as f64) * 0.25;
                    let y10 = base_y + (h10 as f64) * 0.25;
                    let y01 = base_y + (h01 as f64) * 0.25;
                    let y11 = base_y + (h11 as f64) * 0.25;

                    let p00 = [(x0 - camera.x) as f32, (y00 - camera.y) as f32, (z0 - camera.z) as f32];
                    let p01 = [(x0 - camera.x) as f32, (y01 - camera.y) as f32, (z1 - camera.z) as f32];
                    let p11 = [(x1 - camera.x) as f32, (y11 - camera.y) as f32, (z1 - camera.z) as f32];
                    let p10 = [(x1 - camera.x) as f32, (y10 - camera.y) as f32, (z0 - camera.z) as f32];

                    out.push(Vertex {
                        pos: p00,
                        uv: [u_white, v_white],
                        layer,
                        color,
                        light,
                        normal: [0; 4],
                        glint_uv: [0.0, 0.0],
                    });
                    out.push(Vertex {
                        pos: p01,
                        uv: [u_white, v_white],
                        layer,
                        color,
                        light,
                        normal: [0; 4],
                        glint_uv: [0.0, 0.0],
                    });
                    out.push(Vertex {
                        pos: p11,
                        uv: [u_white, v_white],
                        layer,
                        color,
                        light,
                        normal: [0; 4],
                        glint_uv: [0.0, 0.0],
                    });
                    out.push(Vertex {
                        pos: p10,
                        uv: [u_white, v_white],
                        layer,
                        color,
                        light,
                        normal: [0; 4],
                        glint_uv: [0.0, 0.0],
                    });
                }
            }
        }
    }
}

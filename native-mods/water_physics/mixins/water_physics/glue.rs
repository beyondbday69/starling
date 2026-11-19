// Main lifecycle, disturbance scanning, and hooks for the Water Physics native mod (v2).
//
// Key improvements:
// - Fixed 30Hz simulation update independent of render frame rate
// - Mesh rebuild cached and triggered only on simulation ticks
// - Early-out optimization when no water is near the player
// - Mob scanning capped to within 12m and at most 2 mobs
// - 1-second interval logcat profiling (avg sim ms, mesh build ms, active tiles, verts)

use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::game::Game;
use super::native_water_physics_render as render;
use super::native_water_physics_sim as sim;
use horizon_client::mesher::Vertex;

pub const ID: &str = "water_physics";
pub const NAME: &str = "Water Physics";
pub const VERSION: &str = "2.0.0";
pub const DESCRIPTION: &str = "Real 3D ripples, concentric splash shockwaves, and Kelvin boat wakes.";

const SIM_TICK_INTERVAL: Duration = Duration::from_millis(33); // 30 Hz

struct WaterPhysicsState {
    sim: sim::WaterSimulation,
    cache: render::RenderCache,
    last_sim_tick: Instant,
    // Profiling counters
    last_log_time: Instant,
    sim_time_sum_us: u64,
    sim_count: u32,
    mesh_time_sum_us: u64,
    mesh_count: u32,
}

static STATE: Mutex<Option<WaterPhysicsState>> = Mutex::new(None);
static TRACKING: Mutex<PlayerTracking> = Mutex::new(PlayerTracking {
    last_pos_y: 0.0,
    last_vel_y: 0.0,
    step_timer: 0,
});

struct PlayerTracking {
    last_pos_y: f64,
    last_vel_y: f64,
    step_timer: u32,
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut WaterPhysicsState) -> R,
{
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        let now = Instant::now();
        *guard = Some(WaterPhysicsState {
            sim: sim::WaterSimulation::new(),
            cache: render::RenderCache::new(),
            last_sim_tick: now,
            last_log_time: now,
            sim_time_sum_us: 0,
            sim_count: 0,
            mesh_time_sum_us: 0,
            mesh_count: 0,
        });
    }
    f(guard.as_mut().unwrap())
}

impl Game<'_> {
    #[inline]
    pub(super) fn wp_is_water(&self, x: i32, y: i32, z: i32) -> bool {
        let state = self.world.block(x, y, z);
        self.res.facts().get(state).fluid == horizon_items::blocks::Fluid::Water
    }

    pub(super) fn wp_player_near_water(&self, px: f64, py: f64, pz: f64) -> bool {
        let bx = px.floor() as i32;
        let by = py.floor() as i32;
        let bz = pz.floor() as i32;
        for dx in -1..=1 {
            for dz in -1..=1 {
                for dy in -1..=0 {
                    if self.wp_is_water(bx + dx, by + dy, bz + dz) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn wp_water_surface_at(&self, wx: f64, wy: f64, wz: f64) -> Option<f64> {
        let bx = wx.floor() as i32;
        let base_y = wy.floor() as i32;
        let bz = wz.floor() as i32;

        for y in (base_y - 2..=base_y + 1).rev() {
            if self.wp_is_water(bx, y, bz) && !self.wp_is_water(bx, y + 1, bz) {
                return Some(y as f64 + (8.0 / 9.0));
            }
        }
        None
    }

    pub(super) fn water_physics_frame(&mut self) {
        if !self.native_on(ID) {
            return;
        }

        let now = Instant::now();

        with_state(|state| {
            // Early-out: if player is not near water, no boats exist, and no tiles active -> do zero work
            let player_opt = self.player.as_ref();
            let (px, py, pz, p_vy, is_touching_water) = if let Some(player) = player_opt {
                (
                    player.position.x as f64,
                    player.position.y as f64,
                    player.position.z as f64,
                    player.velocity.y as f64,
                    player.fluids.was_touching_water,
                )
            } else {
                return;
            };

            let player_in_or_near_water = is_touching_water || self.wp_player_near_water(px, py, pz);
            let has_boats = !self.boats.is_empty();
            let tiles_active = state.sim.any_active();

            if !player_in_or_near_water && !has_boats && !tiles_active {
                return;
            }

            // Fixed 30Hz sim rate check
            if now.duration_since(state.last_sim_tick) < SIM_TICK_INTERVAL {
                return;
            }
            state.last_sim_tick = now;

            // Sync settings
            if let Some(res) = self.native_setting(ID, "resolution").and_then(|s| s.parse().ok()) {
                state.sim.set_resolution(res);
            }
            if let Some(tiles) = self.native_setting(ID, "tiles").and_then(|s| s.parse().ok()) {
                state.sim.set_max_tiles(tiles);
            }
            if let Some(int_str) = self.native_setting(ID, "intensity") {
                let mult = match int_str.as_str() {
                    "Low" => 0.7f32,
                    "High" => 1.4f32,
                    _ => 1.0f32,
                };
                state.sim.set_intensity(mult);
            }

            let t_sim_start = Instant::now();

            // 1. Local Player disturbances
            if let Some(player) = &self.player {
                if let Some(water_y) = self.wp_water_surface_at(px, py, pz) {
                    let dy = py - water_y;
                    let vx = player.velocity.x as f64;
                    let vz = player.velocity.z as f64;
                    let speed = (vx * vx + vz * vz).sqrt();

                    let mut track = TRACKING.lock().unwrap();

                    // Splash on entering water
                    if track.last_vel_y < -0.18 && track.last_pos_y > water_y - 0.25 && py <= water_y + 0.15 {
                        let intensity = ((-track.last_vel_y as f32) * 1.8).clamp(0.4, 2.2);
                        log::info!(
                            "water_physics: splash impact at ({:.1}, {:.1}, {:.1}), vy={:.2}, intensity={:.2}",
                            px, water_y, pz, track.last_vel_y, intensity
                        );
                        let r = (0.35 * intensity.sqrt()).clamp(0.25, 0.65);
                        let amp = 0.55 * intensity;
                        let foam = 0.85 * intensity;
                        state.sim.inject_splash(px, pz, water_y, r, amp, foam);
                    } else if player.swimming || (dy < -0.2 && speed > 0.08) {
                        // Swimming trailing V-wake
                        let dir_x = vx / speed.max(0.001);
                        let dir_z = vz / speed.max(0.001);
                        let wake_x = px - dir_x * 0.35;
                        let wake_z = pz - dir_z * 0.35;
                        let amp = (0.22 * (speed / 0.15).min(1.2)) as f32;
                        state.sim.inject_impulse(wake_x, wake_z, water_y, 0.30, amp, 0.35);
                    } else if dy >= -0.85 && dy <= 0.30 && speed > 0.04 {
                        // Walking / wading concentric footstep ripples
                        track.step_timer += 1;
                        if track.step_timer >= 3 {
                            track.step_timer = 0;
                            let amp = (0.18 * (speed / 0.15).min(1.0)) as f32;
                            state.sim.inject_impulse(px, pz, water_y, 0.28, amp, 0.25);
                        }
                    }

                    track.last_pos_y = py;
                    track.last_vel_y = p_vy;
                }
            }

            // 2. Boat disturbances: Kelvin wake, bow spray, stern trough, and oar ripples
            for b in &self.boats {
                let bx = b.position.x as f64;
                let by = b.position.y as f64;
                let bz = b.position.z as f64;

                if let Some(water_y) = self.wp_water_surface_at(bx, by, bz) {
                    let bvx = b.velocity.x as f64;
                    let bvz = b.velocity.z as f64;
                    let speed = (bvx * bvx + bvz * bvz).sqrt();
                    if speed > 0.02 {
                        let rad = (b.yaw as f32).to_radians();
                        let forward_x = -rad.sin();
                        let forward_z = rad.cos();
                        let right_x = rad.cos();
                        let right_z = rad.sin();

                        // Bow wave impulse
                        let bow_x = bx + (forward_x * 0.85) as f64;
                        let bow_z = bz + (forward_z * 0.85) as f64;
                        let bow_amp = ((speed * 1.5).min(0.60)) as f32;
                        let bow_foam = ((speed * 1.8).min(0.90)) as f32;
                        state.sim.inject_impulse(bow_x, bow_z, water_y, 0.40, bow_amp, bow_foam);

                        // Stern depression trough
                        let stern_x = bx - (forward_x * 0.85) as f64;
                        let stern_z = bz - (forward_z * 0.85) as f64;
                        state.sim.inject_impulse(stern_x, stern_z, water_y, 0.35, -0.40 * bow_amp, 0.15);

                        // Oar ripples
                        if b.paddling[0] {
                            let oar_lx = bx - (right_x * 0.85) as f64;
                            let oar_lz = bz - (right_z * 0.85) as f64;
                            state.sim.inject_impulse(oar_lx, oar_lz, water_y, 0.30, 0.25, 0.45);
                        }
                        if b.paddling[1] {
                            let oar_rx = bx + (right_x * 0.85) as f64;
                            let oar_rz = bz + (right_z * 0.85) as f64;
                            state.sim.inject_impulse(oar_rx, oar_rz, water_y, 0.30, 0.25, 0.45);
                        }
                    }
                }
            }

            // 3. Nearby Mobs (capped to within 12m and at most 2 mobs)
            let mut mob_count = 0;
            for m in &self.mobs {
                if mob_count >= 2 {
                    break;
                }
                if !m.alive() || !m.fluids.in_water {
                    continue;
                }
                let mx = m.position.x as f64;
                let mz = m.position.z as f64;
                let d2 = (mx - px) * (mx - px) + (mz - pz) * (mz - pz);
                if d2 > 12.0 * 12.0 {
                    continue;
                }
                let mvx = m.velocity.x as f64;
                let mvz = m.velocity.z as f64;
                let mspeed = (mvx * mvx + mvz * mvz).sqrt();
                if mspeed > 0.05 {
                    if let Some(water_y) = self.wp_water_surface_at(mx, m.position.y as f64, mz) {
                        let amp = (0.12 * (mspeed / 0.15).min(1.0)) as f32;
                        state.sim.inject_impulse(mx, mz, water_y, 0.25, amp, 0.15);
                        mob_count += 1;
                    }
                }
            }

            // Step wave simulation
            let stepped = state.sim.step();
            let sim_us = t_sim_start.elapsed().as_micros() as u64;
            state.sim_time_sum_us += sim_us;
            state.sim_count += 1;

            // Rebuild cached mesh when simulation stepped or tiles are active
            if stepped || state.sim.any_active() {
                let t_mesh_start = Instant::now();
                self.rebuild_wave_mesh(&state.sim, &mut state.cache);
                let mesh_us = t_mesh_start.elapsed().as_micros() as u64;
                state.mesh_time_sum_us += mesh_us;
                state.mesh_count += 1;
            } else if !state.sim.any_active() && !state.cache.quads.is_empty() {
                state.cache.clear();
            }

            // 1-second interval logcat profiling output
            if now.duration_since(state.last_log_time) >= Duration::from_secs(1) {
                let avg_sim_ms = (state.sim_time_sum_us as f64 / state.sim_count.max(1) as f64) / 1000.0;
                let avg_mesh_ms = (state.mesh_time_sum_us as f64 / state.mesh_count.max(1) as f64) / 1000.0;
                let active_tiles = state.sim.tiles.iter().filter(|t| t.active).count();
                let vert_count = state.cache.quads.len() * 4;
                log::info!(
                    "water_physics: sim={:.2}ms mesh={:.2}ms tiles={}/{} verts={}",
                    avg_sim_ms,
                    avg_mesh_ms,
                    active_tiles,
                    state.sim.tiles.len(),
                    vert_count
                );
                state.sim_time_sum_us = 0;
                state.sim_count = 0;
                state.mesh_time_sum_us = 0;
                state.mesh_count = 0;
                state.last_log_time = now;
            }
        });
    }
}

pub fn list(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    log::info!("water_physics: native mod registered v{}", VERSION);
    mods.push(horizon_client::mod_api::NativeMod {
        id: ID.into(),
        title: format!("{NAME} {VERSION}"),
        description: DESCRIPTION.into(),
        keys: Vec::new(),
    });
}

pub fn settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    let res = game
        .native_setting(ID, "resolution")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(sim::DEFAULT_RES);
    let tiles = game
        .native_setting(ID, "tiles")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(sim::DEFAULT_MAX_TILES);
    let intensity = game
        .native_setting(ID, "intensity")
        .unwrap_or_else(|| "Normal".into());

    buttons.push(format!("Tile Resolution: {res}x{res}"));
    buttons.push(format!("Max Active Tiles: {tiles}"));
    buttons.push(format!("Wave Intensity: {intensity}"));
}

pub fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    match button {
        0 => {
            let now = game
                .native_setting(ID, "resolution")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(sim::DEFAULT_RES);
            let next = match now {
                24 => 32,
                32 => 40,
                _ => 24,
            };
            game.native_set_setting(ID, "resolution", &next.to_string());
            with_state(|s| {
                s.sim.set_resolution(next);
                s.cache.mark_dirty();
            });
            log::info!("water_physics: resolution changed to {next}x{next}");
        }
        1 => {
            let now = game
                .native_setting(ID, "tiles")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(sim::DEFAULT_MAX_TILES);
            let next = match now {
                1 => 2,
                2 => 3,
                _ => 1,
            };
            game.native_set_setting(ID, "tiles", &next.to_string());
            with_state(|s| {
                s.sim.set_max_tiles(next);
                s.cache.mark_dirty();
            });
            log::info!("water_physics: max active tiles changed to {next}");
        }
        2 => {
            let now = game
                .native_setting(ID, "intensity")
                .unwrap_or_else(|| "Normal".into());
            let (next_str, mult) = match now.as_str() {
                "Low" => ("Normal", 1.0f32),
                "Normal" => ("High", 1.4f32),
                _ => ("Low", 0.7f32),
            };
            game.native_set_setting(ID, "intensity", next_str);
            with_state(|s| {
                s.sim.set_intensity(mult);
                s.cache.mark_dirty();
            });
            log::info!("water_physics: intensity changed to {next_str}");
        }
        _ => {}
    }
}

pub fn frame(game: &mut Game<'_>) {
    game.water_physics_frame();
}

pub fn draw(game: &Game<'_>, camera: glam::DVec3, out: &mut Vec<Vertex>) {
    if !game.native_on(ID) {
        return;
    }

    let mut guard = STATE.lock().unwrap();
    if let Some(state) = guard.as_mut() {
        game.render_wave_overlays(&state.sim, &mut state.cache, camera, out);
    }
}

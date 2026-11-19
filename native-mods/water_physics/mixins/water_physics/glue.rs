// Main lifecycle, disturbance scanning, and hooks for the Water Physics native mod.

use std::sync::Mutex;
use crate::game::Game;
use super::native_water_physics_render as render;
use super::native_water_physics_sim as sim;
use horizon_client::mesher::Vertex;

pub const ID: &str = "water_physics";
pub const NAME: &str = "Water Physics";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "Realistic water wakes, ripples, Kelvin wakes, splashes, and foam trails.";

static SIM: Mutex<Option<sim::WaterSimulation>> = Mutex::new(None);
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

fn with_sim<F, R>(f: F) -> R
where
    F: FnOnce(&mut sim::WaterSimulation) -> R,
{
    let mut guard = SIM.lock().unwrap();
    if guard.is_none() {
        *guard = Some(sim::WaterSimulation::new());
    }
    f(guard.as_mut().unwrap())
}

impl Game<'_> {
    #[inline]
    fn wp_is_water(&self, x: i32, y: i32, z: i32) -> bool {
        let state = self.world.block(x, y, z);
        self.res.facts().get(state).fluid == horizon_items::blocks::Fluid::Water
    }

    fn wp_water_surface_at(&self, wx: f64, wy: f64, wz: f64) -> Option<f64> {
        let bx = wx.floor() as i32;
        let base_y = wy.floor() as i32;
        let bz = wz.floor() as i32;

        for y in (base_y - 2..=base_y + 2).rev() {
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

        with_sim(|sim| {
            // Sync settings from persisted game settings
            if let Some(res) = self.native_setting(ID, "resolution").and_then(|s| s.parse().ok()) {
                sim.set_resolution(res);
            }
            if let Some(tiles) = self.native_setting(ID, "tiles").and_then(|s| s.parse().ok()) {
                sim.set_max_tiles(tiles);
            }
            if let Some(int_str) = self.native_setting(ID, "intensity") {
                let mult = match int_str.as_str() {
                    "Low" => 0.7f32,
                    "High" => 1.4f32,
                    _ => 1.0f32,
                };
                sim.set_intensity(mult);
            }

            // 1. Local Player disturbances: walking ring ripples, swimming V wake, and fall splashes
            if let Some(player) = &self.player {
                let pos = player.position;
                let vel = player.velocity;
                let px = pos.x as f64;
                let py = pos.y as f64;
                let pz = pos.z as f64;

                if let Some(water_y) = self.wp_water_surface_at(px, py, pz) {
                    let dy = py - water_y;
                    let speed = (vel.x * vel.x + vel.z * vel.z).sqrt() as f32;

                    let mut track = TRACKING.lock().unwrap();

                    // Falling into water splash: check downward entry impact
                    if track.last_vel_y < -0.18 && track.last_pos_y > water_y - 0.25 && py <= water_y + 0.15 {
                        let intensity = ((-track.last_vel_y as f32) * 1.5).clamp(0.3, 2.0);
                        log::info!(
                            "water_physics: splash detected at ({:.1}, {:.1}, {:.1}), vy={:.2}, intensity={:.2}",
                            px, water_y, pz, track.last_vel_y, intensity
                        );
                        sim.inject_splash(px, pz, water_y, 1.2 * intensity, 0.45 * intensity, 0.85 * intensity);
                    } else if player.swimming || (dy < -0.2 && speed > 0.08) {
                        // Swimming V wake: body impulse slightly trailing motion
                        let dir_x = vel.x / speed.max(0.001);
                        let dir_z = vel.z / speed.max(0.001);
                        let wake_x = px - (dir_x * 0.4) as f64;
                        let wake_z = pz - (dir_z * 0.4) as f64;
                        let amp = 0.16 * (speed / 0.15).min(1.2);
                        sim.inject_impulse(wake_x, wake_z, water_y, 0.55, amp, 0.35);
                    } else if dy >= -0.85 && dy <= 0.30 && speed > 0.04 {
                        // Walking in shallow water: periodic footstep ring ripples
                        track.step_timer += 1;
                        if track.step_timer >= 4 {
                            track.step_timer = 0;
                            let amp = 0.14 * (speed / 0.15).min(1.0);
                            sim.inject_impulse(px, pz, water_y, 0.45, amp, 0.25);
                        }
                    }

                    track.last_pos_y = py;
                    track.last_vel_y = vel.y as f64;
                }
            }

            // 2. Boat disturbances: Kelvin wake cone, bow spray foam trail, stern depression, and oar ripples
            for b in &self.boats {
                let bx = b.position.x as f64;
                let by = b.position.y as f64;
                let bz = b.position.z as f64;

                if let Some(water_y) = self.wp_water_surface_at(bx, by, bz) {
                    let speed = (b.velocity.x * b.velocity.x + b.velocity.z * b.velocity.z).sqrt() as f32;
                    if speed > 0.02 {
                        let rad = (b.yaw as f32).to_radians();
                        let forward_x = -rad.sin();
                        let forward_z = rad.cos();
                        let right_x = rad.cos();
                        let right_z = rad.sin();

                        // Bow spray and forward wave impulse
                        let bow_x = bx + (forward_x * 0.85) as f64;
                        let bow_z = bz + (forward_z * 0.85) as f64;
                        let bow_amp = (speed * 1.3).min(0.55);
                        let bow_foam = (speed * 1.8).min(0.9);
                        sim.inject_impulse(bow_x, bow_z, water_y, 0.65, bow_amp, bow_foam);

                        // Stern depression trough
                        let stern_x = bx - (forward_x * 0.85) as f64;
                        let stern_z = bz - (forward_z * 0.85) as f64;
                        sim.inject_impulse(stern_x, stern_z, water_y, 0.50, -0.35 * bow_amp, 0.15);

                        // Oar ripples when paddling
                        if b.paddling[0] {
                            let oar_lx = bx - (right_x * 0.90) as f64;
                            let oar_lz = bz - (right_z * 0.90) as f64;
                            sim.inject_impulse(oar_lx, oar_lz, water_y, 0.45, 0.22, 0.45);
                        }
                        if b.paddling[1] {
                            let oar_rx = bx + (right_x * 0.90) as f64;
                            let oar_rz = bz + (right_z * 0.90) as f64;
                            sim.inject_impulse(oar_rx, oar_rz, water_y, 0.45, 0.22, 0.45);
                        }
                    }
                }
            }

            // 3. Nearby Mobs in water
            if let Some(player) = &self.player {
                let px = player.position.x as f64;
                let pz = player.position.z as f64;
                for m in &self.mobs {
                    if !m.alive() {
                        continue;
                    }
                    let mx = m.position.x as f64;
                    let mz = m.position.z as f64;
                    let d2 = (mx - px) * (mx - px) + (mz - pz) * (mz - pz);
                    if d2 > 24.0 * 24.0 {
                        continue;
                    }
                    if m.fluids.in_water {
                        let mspeed = (m.velocity.x * m.velocity.x + m.velocity.z * m.velocity.z).sqrt() as f32;
                        if mspeed > 0.04 {
                            if let Some(water_y) = self.wp_water_surface_at(mx, m.position.y as f64, mz) {
                                let amp = 0.10 * (mspeed / 0.15).min(1.0);
                                sim.inject_impulse(mx, mz, water_y, 0.40, amp, 0.15);
                            }
                        }
                    }
                }
            }

            // Step wave tiles
            sim.step();
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
                16 => 24,
                24 => 32,
                _ => 16,
            };
            game.native_set_setting(ID, "resolution", &next.to_string());
            with_sim(|s| s.set_resolution(next));
            log::info!("water_physics: resolution changed to {next}x{next}");
        }
        1 => {
            let now = game
                .native_setting(ID, "tiles")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(sim::DEFAULT_MAX_TILES);
            let next = match now {
                2 => 4,
                4 => 6,
                _ => 2,
            };
            game.native_set_setting(ID, "tiles", &next.to_string());
            with_sim(|s| s.set_max_tiles(next));
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
            with_sim(|s| s.set_intensity(mult));
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

    let guard = SIM.lock().unwrap();
    if let Some(sim) = guard.as_ref() {
        render::render_wave_overlays(sim, game, camera, out);
    }
}

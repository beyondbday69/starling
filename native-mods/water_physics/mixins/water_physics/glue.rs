// Main lifecycle, disturbance scanning, and hooks for the Water Physics native mod (v3.0).
//
// Key features:
// - Smooth geometric fluid simulation: Concentric expanding circular 3D waves & continuous Kelvin V-wakes
// - Real water splash droplet bursts via ParticleType::Splash on impact
// - Alternating footstep concentric ripple rings when wading in shallow water
// - High-speed hydrodynamics for boats with hull wake, foam churning, and oar ripples
// - Continuous frame-rate-independent delta-time integration for buttery smooth 60/90/120 FPS animation
// - Position-delta motion tracking, resolving zeroed velocity on mobile/multiplayer

use std::sync::Mutex;
use std::time::Instant;
use crate::game::Game;
use super::native_water_physics_render as render;
use super::native_water_physics_sim as sim;
use horizon_client::mesher::Vertex;
use horizon_client::particles::{ParticleOptions, ParticleType};

pub const ID: &str = "water_physics";
pub const NAME: &str = "Water Physics";
pub const VERSION: &str = "3.7.0";
pub const DESCRIPTION: &str = "Full surrounding water hydrodynamics, wide 8.5m lateral bow surge waves, 3D rolling swells, boat spray & bubble churn, and shader-preserving optics.";

struct WaterPhysicsState {
    sim: sim::WaterSimulation,
    cache: render::RenderCache,
    last_tick: Instant,
    last_log_time: Instant,
    boat_spray_timer: f32,
    boat_surge_timer: f32,
    ambient_wavelet_timer: f32,
}

static STATE: Mutex<Option<WaterPhysicsState>> = Mutex::new(None);
static TRACKING: Mutex<PlayerTracking> = Mutex::new(PlayerTracking {
    last_pos_x: 0.0,
    last_pos_y: 0.0,
    last_pos_z: 0.0,
    last_vel_y: 0.0,
    last_time: None,
    step_accum: 0.0,
    wake_accum: 0.0,
    alt_step: false,
    initialized: false,
});

struct PlayerTracking {
    last_pos_x: f64,
    last_pos_y: f64,
    last_pos_z: f64,
    last_vel_y: f64,
    last_time: Option<Instant>,
    step_accum: f64,
    wake_accum: f64,
    alt_step: bool,
    initialized: bool,
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
            last_tick: now,
            last_log_time: now,
            boat_spray_timer: 0.0,
            boat_surge_timer: 0.0,
            ambient_wavelet_timer: 0.0,
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
        for dx in -2..=2 {
            for dz in -2..=2 {
                for dy in -2..=1 {
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

        for y in (base_y - 3..=base_y + 5).rev() {
            if self.wp_is_water(bx, y, bz) && !self.wp_is_water(bx, y + 1, bz) {
                return Some(y as f64 + (8.0 / 9.0));
            }
        }
        // Check immediate horizontal neighbors if right on the shoreline edge or wading
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for y in (base_y - 3..=base_y + 5).rev() {
                if self.wp_is_water(bx + dx, y, bz + dz) && !self.wp_is_water(bx + dx, y + 1, bz + dz) {
                    return Some(y as f64 + (8.0 / 9.0));
                }
            }
        }
        None
    }

    pub(super) fn water_physics_frame(&mut self) {
        if !self.native_on(ID) {
            return;
        }

        let now = Instant::now();

        // 1. Collect disturbances outside the mutex to allow mutable particle spawning
        let mut splash_impact: Option<(f64, f64, f64, f32)> = None;
        let mut wake_sample: Option<(f64, f64, f64, f32, f32, f32, bool)> = None;
        let mut player_rings: Vec<(f64, f64, f64, f32)> = Vec::new();
        let mut ambient_rings: Vec<(f64, f64, f64, f32)> = Vec::new();
        let mut surrounding_surge_rings: Vec<(f64, f64, f64, f32)> = Vec::new();
        let mut boat_samples: Vec<(f64, f64, f64, f32, f32, f32)> = Vec::new();
        let mut boat_spray_emitters: Vec<(f64, f64, f64, f32, f32, f32)> = Vec::new();
        let mut boat_surge_emitters: Vec<(f64, f64, f64, f32, f32, f32)> = Vec::new();
        let mut mob_rings: Vec<(f64, f64, f64)> = Vec::new();

        // Player disturbance evaluation
        if let Some(player) = self.player.as_ref() {
            let px = player.position.x as f64;
            let py = player.position.y as f64;
            let pz = player.position.z as f64;
            let p_vy = player.velocity.y as f64;

            let is_in_boat = self.riding.is_some()
                || player.is_passenger
                || player.fluids.boat.is_some()
                || self.boats.iter().any(|b| {
                    let dx = b.position.x as f64 - px;
                    let dz = b.position.z as f64 - pz;
                    let dy = (b.position.y as f64 - py).abs();
                    (dx * dx + dz * dz) < 2.5 && dy < 1.6
                });

            if is_in_boat {
                // When riding in a boat or vehicle, suppress all footstep rings,
                // splash impacts, transverse ripple rings, and player wake ribbons!
                // Keep tracking state synchronized with the player's position so
                // that dismounting doesn't create a giant sudden delta jump.
                let mut track = TRACKING.lock().unwrap();
                track.last_pos_x = px;
                track.last_pos_y = py;
                track.last_pos_z = pz;
                track.last_vel_y = p_vy;
                track.last_time = Some(now);
                track.step_accum = 0.0;
                track.wake_accum = 0.0;
                track.initialized = true;
            } else if let Some(water_y) = self.wp_water_surface_at(px, py, pz) {
                let mut track = TRACKING.lock().unwrap();
                if !track.initialized {
                    track.last_pos_x = px;
                    track.last_pos_y = py;
                    track.last_pos_z = pz;
                    track.last_vel_y = p_vy;
                    track.last_time = Some(now);
                    track.step_accum = 0.0;
                    track.wake_accum = 0.0;
                    track.alt_step = false;
                    track.initialized = true;
                } else {
                    let delta_x = px - track.last_pos_x;
                    let delta_y = py - track.last_pos_y;
                    let delta_z = pz - track.last_pos_z;
                    let dist_h = (delta_x * delta_x + delta_z * delta_z).sqrt();

                    let dt_sec = track
                        .last_time
                        .map(|t| now.duration_since(t).as_secs_f64())
                        .unwrap_or(0.016)
                        .clamp(0.001, 0.10);
                    track.last_time = Some(now);

                    let speed_mps = (dist_h / dt_sec) as f32;
                    let p_vx = player.velocity.x as f64;
                    let p_vz = player.velocity.z as f64;
                    let vel_h = (p_vx * p_vx + p_vz * p_vz).sqrt();
                    let vel_mps = (vel_h * 20.0) as f32; // tick velocity to m/s
                    let speed = speed_mps.max(vel_mps).clamp(0.0, 8.0);

                    let (dir_x, dir_z) = if dist_h > 0.002 {
                        ((delta_x / dist_h) as f32, (delta_z / dist_h) as f32)
                    } else if vel_h > 0.002 {
                        ((p_vx / vel_h) as f32, (p_vz / vel_h) as f32)
                    } else {
                        (0.0, 0.0)
                    };

                    // Splash on impact
                    let vy_check = delta_y.min(p_vy);
                    if vy_check < -0.15 && track.last_pos_y > water_y - 0.20 && py <= water_y + 0.15 {
                        let impact = ((-vy_check as f32) * 2.2).clamp(0.5, 2.5);
                        splash_impact = Some((px, water_y, pz, impact));
                    }

                    // Surface interaction (wading or swimming at water boundary)
                    let in_water = player.fluids.was_touching_water || player.swimming;
                    let is_at_surface = (py <= water_y + 0.40 && py + 2.50 >= water_y) || (in_water && py <= water_y + 0.25);
                    if is_at_surface && speed > 0.12 && (dir_x * dir_x + dir_z * dir_z) > 0.01 {
                        let is_swimming = player.swimming || py < water_y - 0.35;
                        wake_sample = Some((px, pz, water_y, dir_x, dir_z, speed, is_swimming));

                        if !is_swimming {
                            // Wading footstep ripples
                            track.step_accum += dist_h;
                            if track.step_accum >= 0.45 {
                                track.step_accum = 0.0;
                                track.alt_step = !track.alt_step;
                                let offset_lat = if track.alt_step { 0.22 } else { -0.22 };
                                let foot_x = px + (-dir_z as f64) * offset_lat;
                                let foot_z = pz + (dir_x as f64) * offset_lat;
                                player_rings.push((foot_x, foot_z, water_y, 0.55));
                            }
                        }

                        // Periodic trailing transverse 3D ripple shedding behind the moving body
                        track.wake_accum += dist_h;
                        let shed_threshold = if is_swimming { 0.60 } else { 0.50 };
                        if track.wake_accum >= shed_threshold {
                            track.wake_accum = 0.0;
                            let trail_offset = 0.30;
                            let shed_x = px - (dir_x as f64) * trail_offset;
                            let shed_z = pz - (dir_z as f64) * trail_offset;
                            let ring_amp = (speed * 0.28).clamp(0.40, 0.95);
                            player_rings.push((shed_x, shed_z, water_y, ring_amp));
                        }
                    }

                    track.last_pos_x = px;
                    track.last_pos_y = py;
                    track.last_pos_z = pz;
                    track.last_vel_y = p_vy;
                }
            }
        }

        // Boat disturbance & particle evaluation
        for b in &self.boats {
            let bx = b.position.x as f64;
            let by = b.position.y as f64;
            let bz = b.position.z as f64;
            if let Some(water_y) = self.wp_water_surface_at(bx, by, bz) {
                let bvx = b.velocity.x as f64;
                let bvz = b.velocity.z as f64;
                let b_speed = (((bvx * bvx + bvz * bvz).sqrt() * 20.0) as f32).clamp(0.0, 10.0);
                let rad = (b.yaw as f32).to_radians();
                let fwd_x = -rad.sin();
                let fwd_z = rad.cos();

                if b_speed > 0.25 {
                    // Offset slightly toward the boat stern where the wake originates
                    let stern_x = bx - (fwd_x as f64) * 0.35;
                    let stern_z = bz - (fwd_z as f64) * 0.35;

                    boat_samples.push((stern_x, stern_z, water_y, fwd_x, fwd_z, b_speed));
                }

                // Boat water particles when cruising through water
                if b_speed > 1.4 {
                    boat_spray_emitters.push((bx, bz, water_y, fwd_x, fwd_z, b_speed));
                }

                // Surrounding water reaction ("aju baju ka pani"):
                // When cruising, the bow pushes outward lateral surge wave pulses
                // that propagate 6m - 9m across the surrounding water on both sides
                if b_speed > 0.75 {
                    boat_surge_emitters.push((bx, bz, water_y, fwd_x, fwd_z, b_speed));
                }
            }
        }

        // Nearby mob evaluation
        let px = self.player.as_ref().map(|p| p.position.x as f64).unwrap_or(0.0);
        let pz = self.player.as_ref().map(|p| p.position.z as f64).unwrap_or(0.0);
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
            // Skip mobs riding a boat
            if self.boats.iter().any(|b| {
                let dx = b.position.x as f64 - mx;
                let dz = b.position.z as f64 - mz;
                (dx * dx + dz * dz) < 2.0
            }) {
                continue;
            }
            let mvx = m.velocity.x as f64;
            let mvz = m.velocity.z as f64;
            if (mvx * mvx + mvz * mvz).sqrt() > 0.06 {
                if let Some(water_y) = self.wp_water_surface_at(mx, m.position.y as f64, mz) {
                    mob_rings.push((mx, mz, water_y));
                    mob_count += 1;
                }
            }
        }

        // 2. Check timers for boat spray, lateral surge, & ambient wavelets
        let (emit_boat_spray, emit_boat_surge, emit_ambient) = with_state(|state| {
            let dt = now.duration_since(state.last_tick).as_secs_f32().min(0.10);
            state.boat_spray_timer += dt;
            let spray = if state.boat_spray_timer >= 0.08 {
                state.boat_spray_timer = 0.0;
                true
            } else {
                false
            };

            state.boat_surge_timer += dt;
            let surge = if state.boat_surge_timer >= 0.26 {
                state.boat_surge_timer = 0.0;
                true
            } else {
                false
            };

            state.ambient_wavelet_timer += dt;
            let ambient = if state.ambient_wavelet_timer >= 2.2 {
                state.ambient_wavelet_timer = 0.0;
                true
            } else {
                false
            };

            (spray, surge, ambient)
        });

        // 3. Spawn real water splash droplet particles when entering water
        if let Some((sx, sy, sz, impact)) = splash_impact {
            let droplet_count = ((impact * 10.0) as usize).clamp(6, 20);
            for i in 0..droplet_count {
                let angle = (i as f64 / droplet_count as f64) * std::f64::consts::PI * 2.0;
                let spd = 0.07 + 0.06 * ((i % 3) as f64);
                let vx = angle.cos() * spd;
                let vz = angle.sin() * spd;
                let vy = 0.14 + 0.08 * ((i % 2) as f64) * (impact as f64);
                self.particles.add(
                    &ParticleOptions::simple(ParticleType::Splash),
                    [sx + vx * 0.4, sy + 0.04, sz + vz * 0.4],
                    [vx, vy, vz],
                );
            }
        }

        // 4. Spawn boat water particles (bow spray & stern bubble churn)
        if emit_boat_spray {
            for (bx, bz, water_y, fwd_x, fwd_z, b_speed) in boat_spray_emitters {
                let left_x = -fwd_z;
                let left_z = fwd_x;

                // Bow cutting spray (front of the boat cutting laterally through water)
                let bow_x = bx + (fwd_x as f64) * 0.95;
                let bow_z = bz + (fwd_z as f64) * 0.95;
                let bow_y = water_y + 0.05;

                let spd_factor = (b_speed * 0.035).clamp(0.06, 0.20) as f64;
                let vy = (0.07 + (b_speed * 0.015).clamp(0.02, 0.10)) as f64;

                // Left spray droplet
                let vx_l = (fwd_x as f64) * 0.05 + (left_x as f64) * spd_factor;
                let vz_l = (fwd_z as f64) * 0.05 + (left_z as f64) * spd_factor;
                self.particles.add(
                    &ParticleOptions::simple(ParticleType::Splash),
                    [bow_x + (left_x as f64) * 0.32, bow_y, bow_z + (left_z as f64) * 0.32],
                    [vx_l, vy, vz_l],
                );

                // Right spray droplet
                let vx_r = (fwd_x as f64) * 0.05 - (left_x as f64) * spd_factor;
                let vz_r = (fwd_z as f64) * 0.05 - (left_z as f64) * spd_factor;
                self.particles.add(
                    &ParticleOptions::simple(ParticleType::Splash),
                    [bow_x - (left_x as f64) * 0.32, bow_y, bow_z - (left_z as f64) * 0.32],
                    [vx_r, vy, vz_r],
                );

                // Stern wash churn (bubbles & foaming wake droplets)
                let stern_x = bx - (fwd_x as f64) * 0.90;
                let stern_z = bz - (fwd_z as f64) * 0.90;
                let stern_y = water_y - 0.02;

                let churn_vx = -(fwd_x as f64) * 0.06;
                let churn_vz = -(fwd_z as f64) * 0.06;
                self.particles.add(
                    &ParticleOptions::simple(ParticleType::Bubble),
                    [stern_x, stern_y, stern_z],
                    [churn_vx, 0.06, churn_vz],
                );

                if b_speed > 2.8 {
                    self.particles.add(
                        &ParticleOptions::simple(ParticleType::Splash),
                        [stern_x, water_y + 0.03, stern_z],
                        [churn_vx * 0.8, 0.08, churn_vz * 0.8],
                    );
                }
            }
        }

        // 5. Surrounding water reaction ("aju baju ka pani"):
        // Lateral surge waves shed from the bow shoulders and roll 8.5m across the surrounding water
        if emit_boat_surge {
            for (bx, bz, water_y, fwd_x, fwd_z, b_speed) in boat_surge_emitters {
                let left_x = -fwd_z;
                let left_z = fwd_x;

                let l_x = bx + (fwd_x as f64) * 0.65 + (left_x as f64) * 0.85;
                let l_z = bz + (fwd_z as f64) * 0.65 + (left_z as f64) * 0.85;
                let r_x = bx + (fwd_x as f64) * 0.65 - (left_x as f64) * 0.85;
                let r_z = bz + (fwd_z as f64) * 0.65 - (left_z as f64) * 0.85;

                let surge_amp = (b_speed * 0.25).clamp(0.40, 1.25);
                surrounding_surge_rings.push((l_x, l_z, water_y, surge_amp));
                surrounding_surge_rings.push((r_x, r_z, water_y, surge_amp));
            }
        }

        // 6. Ambient 3D surface wavelets (gentle water breathing when idling)
        if emit_ambient {
            for b in &self.boats {
                let bx = b.position.x as f64;
                let by = b.position.y as f64;
                let bz = b.position.z as f64;
                if let Some(water_y) = self.wp_water_surface_at(bx, by, bz) {
                    let bvx = b.velocity.x as f64;
                    let bvz = b.velocity.z as f64;
                    let b_speed = (((bvx * bvx + bvz * bvz).sqrt() * 20.0) as f32).clamp(0.0, 10.0);
                    if b_speed < 0.40 {
                        ambient_rings.push((bx, bz, water_y, 0.22));
                    }
                }
            }

            if let Some(player) = self.player.as_ref() {
                let px = player.position.x as f64;
                let py = player.position.y as f64;
                let pz = player.position.z as f64;
                if let Some(water_y) = self.wp_water_surface_at(px, py, pz) {
                    let in_water = player.fluids.was_touching_water || player.swimming;
                    let is_at_surface = (py <= water_y + 0.40 && py + 2.50 >= water_y) || (in_water && py <= water_y + 0.25);
                    let p_vx = player.velocity.x as f64;
                    let p_vz = player.velocity.z as f64;
                    let vel_h = (p_vx * p_vx + p_vz * p_vz).sqrt() * 20.0;
                    if is_at_surface && vel_h < 0.25 {
                        ambient_rings.push((px, pz, water_y, 0.18));
                    }
                }
            }
        }

        // 7. Inject geometric waves and step simulation
        with_state(|state| {
            let dt = now.duration_since(state.last_tick).as_secs_f32().min(0.10);
            state.last_tick = now;

            // Sync user settings
            if let Some(seg) = self.native_setting(ID, "quality").and_then(|s| match s.as_str() {
                "Low" => Some(16),
                "Ultra" => Some(32),
                _ => Some(24),
            }) {
                state.sim.set_segments(seg);
            }
            if let Some(rings) = self.native_setting(ID, "rings").and_then(|s| s.parse().ok()) {
                state.sim.set_max_rings(rings);
            }
            if let Some(int_str) = self.native_setting(ID, "intensity") {
                let mult = match int_str.as_str() {
                    "Low" => 0.7f32,
                    "High" => 1.4f32,
                    _ => 1.0f32,
                };
                state.sim.set_intensity(mult);
            }

            // Splash burst
            if let Some((sx, sy, sz, impact)) = splash_impact {
                state.sim.spawn_splash_burst(sx, sz, sy, impact);
            }

            // Surrounding water 3D surge waves (reaches 8.5m into surrounding water on both sides!)
            for (rx, rz, ry, r_amp) in surrounding_surge_rings {
                state.sim.spawn_ring(rx, rz, ry, 0.35, 8.5, 2.1, r_amp, 4.0, false);
            }

            // Player wake & step/transverse ripples (reaches 4.8m into surrounding water)
            if let Some((wx, wz, wy, dx, dz, spd, _is_swim)) = wake_sample {
                state.sim.add_wake_node(wx, wz, wy, dx, dz, spd, false);
            }
            for (rx, rz, ry, r_amp) in player_rings {
                state.sim.spawn_ring(rx, rz, ry, 0.10, 4.8, 1.35, r_amp, 2.8, false);
            }

            // Ambient 3D surface wavelets (reaches 3.8m around idle entities)
            for (rx, rz, ry, r_amp) in ambient_rings {
                state.sim.spawn_ring(rx, rz, ry, 0.35, 3.8, 0.50, r_amp, 3.5, false);
            }

            // Boat wakes
            for (bx, bz, by, fwd_x, fwd_z, b_speed) in boat_samples {
                state.sim.add_wake_node(bx, bz, by, fwd_x, fwd_z, b_speed, true);
            }

            // Mobs
            for (mx, mz, my) in mob_rings {
                state.sim.spawn_ring(mx, mz, my, 0.05, 1.5, 0.95, 0.35, 1.8, false);
            }

            // Step fluid physics at live frame rate
            state.sim.step(dt);

            // Logcat profiling output every 2 seconds
            if now.duration_since(state.last_log_time) >= std::time::Duration::from_secs(2) {
                log::info!(
                    "water_physics v3.7: rings={}, player_wakes={}, boat_wakes={}",
                    state.sim.rings.len(),
                    state.sim.player_wakes.len(),
                    state.sim.boat_wakes.len()
                );
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
    let quality = game
        .native_setting(ID, "quality")
        .unwrap_or_else(|| "Smooth".into());
    let rings = game
        .native_setting(ID, "rings")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(sim::DEFAULT_MAX_RINGS);
    let intensity = game
        .native_setting(ID, "intensity")
        .unwrap_or_else(|| "Normal".into());

    buttons.push(format!("Wave Quality: {quality}"));
    buttons.push(format!("Max Wave Rings: {rings}"));
    buttons.push(format!("Wave Intensity: {intensity}"));
}

pub fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    match button {
        0 => {
            let now = game
                .native_setting(ID, "quality")
                .unwrap_or_else(|| "Smooth".into());
            let (next_str, seg) = match now.as_str() {
                "Smooth" => ("Ultra", 32),
                "Ultra" => ("Low", 16),
                _ => ("Smooth", 24),
            };
            game.native_set_setting(ID, "quality", next_str);
            with_state(|s| s.sim.set_segments(seg));
            log::info!("water_physics: quality changed to {next_str} ({seg} segments)");
        }
        1 => {
            let now = game
                .native_setting(ID, "rings")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(sim::DEFAULT_MAX_RINGS);
            let next = match now {
                12 => 18,
                18 => 24,
                _ => 12,
            };
            game.native_set_setting(ID, "rings", &next.to_string());
            with_state(|s| s.sim.set_max_rings(next));
            log::info!("water_physics: max rings changed to {next}");
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
            with_state(|s| s.sim.set_intensity(mult));
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

//! 2D Heightfield water wave simulation with leapfrog PDE and dispersion.
//!
//! Simulation PDE:
//!   h_new = 2h - h_prev + C^2 * laplacian + dispersion - damping * (h - h_prev)
//! Plus local foam decay and energy-based idle tile suspension.

pub const DEFAULT_RES: usize = 24;
pub const DEFAULT_MAX_TILES: usize = 4;
pub const TILE_SIZE: f32 = 12.0;

// Tuned for Kelvin wake half-angle (~19.47°) when source moves at boat speed ~0.35 blocks/tick.
const WAVE_SPEED_SQ: f32 = 0.28;
const DAMPING: f32 = 0.025;
const DISPERSION: f32 = 0.04;
const FOAM_DECAY: f32 = 0.94;
const ENERGY_IDLE_THRESHOLD: f32 = 0.0003;
const IDLE_TICKS_LIMIT: u32 = 35;

#[derive(Clone, Debug)]
pub struct WaveTile {
    pub center_x: f64,
    pub center_z: f64,
    pub water_y: f64,
    pub res: usize,
    pub h: Vec<f32>,
    pub h_prev: Vec<f32>,
    pub foam: Vec<f32>,
    pub active: bool,
    pub energy: f32,
    pub idle_ticks: u32,
}

impl WaveTile {
    pub fn new(center_x: f64, center_z: f64, water_y: f64, res: usize) -> Self {
        let size = res * res;
        Self {
            center_x,
            center_z,
            water_y,
            res,
            h: vec![0.0; size],
            h_prev: vec![0.0; size],
            foam: vec![0.0; size],
            active: false,
            energy: 0.0,
            idle_ticks: 0,
        }
    }

    pub fn reset(&mut self, center_x: f64, center_z: f64, water_y: f64) {
        self.center_x = center_x;
        self.center_z = center_z;
        self.water_y = water_y;
        self.h.fill(0.0);
        self.h_prev.fill(0.0);
        self.foam.fill(0.0);
        self.active = true;
        self.energy = 0.0;
        self.idle_ticks = 0;
    }

    #[inline]
    pub fn covers(&self, x: f64, z: f64) -> bool {
        let half = (TILE_SIZE * 0.5 - 0.5) as f64;
        (x - self.center_x).abs() <= half && (z - self.center_z).abs() <= half
    }

    pub fn step(&mut self) {
        if !self.active {
            return;
        }

        let res = self.res;
        let mut h_next = vec![0.0f32; res * res];
        let mut total_energy = 0.0f32;

        for j in 1..res - 1 {
            let row = j * res;
            let row_prev = (j - 1) * res;
            let row_next = (j + 1) * res;

            for i in 1..res - 1 {
                let idx = row + i;
                let c = self.h[idx];
                let p = self.h_prev[idx];

                // 4-neighbor cardinal laplacian
                let lap = self.h[idx - 1] + self.h[idx + 1] + self.h[row_prev + i] + self.h[row_next + i] - 4.0 * c;

                // 4-neighbor diagonal laplacian
                let diag = self.h[row_prev + i - 1]
                    + self.h[row_prev + i + 1]
                    + self.h[row_next + i - 1]
                    + self.h[row_next + i + 1]
                    - 4.0 * c;

                // Combined isotropic laplacian
                let lap_iso = 0.5 * lap + 0.25 * diag;

                // Dispersion term creates feathery transverse wavelets behind moving disturbance
                let disp = DISPERSION * (diag - lap);

                // Leapfrog integration step with damping
                let mut val = 2.0 * c - p + WAVE_SPEED_SQ * lap_iso + disp - DAMPING * (c - p);

                // Boundary absorption ramp
                if i <= 2 || i >= res - 3 || j <= 2 || j >= res - 3 {
                    val *= 0.80;
                }

                h_next[idx] = val;
                total_energy += val.abs() + (val - c).abs();
            }
        }

        self.h_prev = std::mem::replace(&mut self.h, h_next);

        // Decay foam mask
        for f in &mut self.foam {
            *f *= FOAM_DECAY;
            if *f < 0.005 {
                *f = 0.0;
            }
        }

        self.energy = total_energy;
        if total_energy < ENERGY_IDLE_THRESHOLD {
            self.idle_ticks += 1;
            if self.idle_ticks >= IDLE_TICKS_LIMIT {
                self.active = false;
            }
        } else {
            self.idle_ticks = 0;
        }
    }

    pub fn inject_impulse(&mut self, wx: f64, wz: f64, radius: f32, amplitude: f32, add_foam: f32) {
        let res = self.res;
        let dx = TILE_SIZE / (res as f32);
        let origin_x = self.center_x - (TILE_SIZE * 0.5) as f64;
        let origin_z = self.center_z - (TILE_SIZE * 0.5) as f64;

        let gx = ((wx - origin_x) as f32) / dx;
        let gz = ((wz - origin_z) as f32) / dx;

        let r_cells = (radius / dx).max(1.0);
        let r_sq = r_cells * r_cells;

        let min_i = (gx - r_cells * 2.0).floor().max(1.0) as usize;
        let max_i = (gx + r_cells * 2.0).ceil().min((res - 2) as f32) as usize;
        let min_j = (gz - r_cells * 2.0).floor().max(1.0) as usize;
        let max_j = (gz + r_cells * 2.0).ceil().min((res - 2) as f32) as usize;

        if min_i > max_i || min_j > max_j {
            return;
        }

        self.active = true;
        self.idle_ticks = 0;

        for j in min_j..=max_j {
            let row = j * res;
            let dj = (j as f32) - gz;
            for i in min_i..=max_i {
                let di = (i as f32) - gx;
                let d2 = di * di + dj * dj;
                if d2 <= r_sq * 4.0 {
                    let factor = (-d2 / (2.0 * r_sq)).exp();
                    let idx = row + i;
                    self.h[idx] += amplitude * factor;
                    if add_foam > 0.0 {
                        self.foam[idx] = (self.foam[idx] + add_foam * factor).min(1.0);
                    }
                }
            }
        }
    }

    pub fn inject_splash(&mut self, wx: f64, wz: f64, radius: f32, amplitude: f32, foam_amount: f32) {
        let res = self.res;
        let dx = TILE_SIZE / (res as f32);
        let origin_x = self.center_x - (TILE_SIZE * 0.5) as f64;
        let origin_z = self.center_z - (TILE_SIZE * 0.5) as f64;

        let gx = ((wx - origin_x) as f32) / dx;
        let gz = ((wz - origin_z) as f32) / dx;

        let r_cells = (radius / dx).max(1.2);
        let r_sq = r_cells * r_cells;

        let min_i = (gx - r_cells * 2.5).floor().max(1.0) as usize;
        let max_i = (gx + r_cells * 2.5).ceil().min((res - 2) as f32) as usize;
        let min_j = (gz - r_cells * 2.5).floor().max(1.0) as usize;
        let max_j = (gz + r_cells * 2.5).ceil().min((res - 2) as f32) as usize;

        if min_i > max_i || min_j > max_j {
            return;
        }

        self.active = true;
        self.idle_ticks = 0;

        for j in min_j..=max_j {
            let row = j * res;
            let dj = (j as f32) - gz;
            for i in min_i..=max_i {
                let di = (i as f32) - gx;
                let d2 = di * di + dj * dj;
                if d2 <= r_sq * 6.0 {
                    let d_norm = (d2 / r_sq).sqrt();
                    // Splash depression in center with raised shockwave rim
                    let wave = (1.0 - d_norm * d_norm) * (-d2 / (2.0 * r_sq)).exp();
                    let idx = row + i;
                    self.h[idx] += amplitude * wave;
                    self.foam[idx] = (self.foam[idx] + foam_amount * (-d2 / (2.5 * r_sq)).exp()).min(1.0);
                }
            }
        }
    }
}

pub struct WaterSimulation {
    pub tiles: Vec<WaveTile>,
    pub resolution: usize,
    pub max_tiles: usize,
    pub intensity: f32,
}

impl WaterSimulation {
    pub fn new() -> Self {
        Self {
            tiles: Vec::with_capacity(DEFAULT_MAX_TILES),
            resolution: DEFAULT_RES,
            max_tiles: DEFAULT_MAX_TILES,
            intensity: 1.0,
        }
    }

    pub fn set_resolution(&mut self, res: usize) {
        if self.resolution == res {
            return;
        }
        self.resolution = res;
        for tile in &mut self.tiles {
            *tile = WaveTile::new(tile.center_x, tile.center_z, tile.water_y, res);
        }
    }

    pub fn set_max_tiles(&mut self, max: usize) {
        self.max_tiles = max;
        if self.tiles.len() > max {
            self.tiles.truncate(max);
        }
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.4, 2.5);
    }

    pub fn get_or_create_tile(&mut self, wx: f64, wz: f64, water_y: f64) -> &mut WaveTile {
        // 1. Check if an active tile covers this position
        for i in 0..self.tiles.len() {
            if self.tiles[i].active && (self.tiles[i].water_y - water_y).abs() < 0.8 && self.tiles[i].covers(wx, wz) {
                return &mut self.tiles[i];
            }
        }

        // 2. Check for an idle tile to re-center
        for i in 0..self.tiles.len() {
            if !self.tiles[i].active {
                self.tiles[i].reset(wx, wz, water_y);
                return &mut self.tiles[i];
            }
        }

        // 3. Allocate a new tile if under capacity
        if self.tiles.len() < self.max_tiles {
            let mut tile = WaveTile::new(wx, wz, water_y, self.resolution);
            tile.active = true;
            self.tiles.push(tile);
            return self.tiles.last_mut().unwrap();
        }

        // 4. Overwrite the tile with lowest energy or furthest from disturbance
        let mut victim_idx = 0;
        let mut min_energy = f32::MAX;
        for (idx, tile) in self.tiles.iter().enumerate() {
            if tile.energy < min_energy {
                min_energy = tile.energy;
                victim_idx = idx;
            }
        }

        self.tiles[victim_idx].reset(wx, wz, water_y);
        &mut self.tiles[victim_idx]
    }

    pub fn inject_impulse(&mut self, wx: f64, wz: f64, water_y: f64, radius: f32, amplitude: f32, add_foam: f32) {
        let amp = amplitude * self.intensity;
        let tile = self.get_or_create_tile(wx, wz, water_y);
        tile.inject_impulse(wx, wz, radius, amp, add_foam);
    }

    pub fn inject_splash(&mut self, wx: f64, wz: f64, water_y: f64, radius: f32, amplitude: f32, foam_amount: f32) {
        let amp = amplitude * self.intensity;
        let tile = self.get_or_create_tile(wx, wz, water_y);
        tile.inject_splash(wx, wz, radius, amp, foam_amount);
    }

    pub fn step(&mut self) {
        for tile in &mut self.tiles {
            if tile.active {
                tile.step();
            }
        }
    }
}

// Geometric water wave and wake simulation engine (v3.3).
//
// Key features:
// 1. Concentric Ripple Rings: Smooth circular expanding torus waves for footsteps, drops, splashes, and oars.
//    Mathematically continuous harmonic envelope (smooth attack + cubic decay): ZERO spawn pop, ZERO death pop.
// 2. Separate Player and Boat Kelvin Wake Channels: Prevents cross-entity ribbon stretching.
// 3. Node continuity enforcement: Eliminates random stretched ribbons on pauses, turns, and teleports.
// 4. Zero-grid architecture: Pure analytical fluid geometry without creases or square edges.

pub const DEFAULT_MAX_RINGS: usize = 48;
pub const DEFAULT_MAX_WAKE_NODES: usize = 40;

#[derive(Clone, Debug)]
pub struct RippleRing {
    pub center_x: f64,
    pub center_z: f64,
    pub water_y: f64,
    pub radius: f32,
    pub max_radius: f32,
    pub speed: f32,
    pub amplitude: f32,
    pub age: f32,
    pub max_age: f32,
    pub has_foam: bool,
}

impl RippleRing {
    pub fn new(
        center_x: f64,
        center_z: f64,
        water_y: f64,
        radius: f32,
        max_radius: f32,
        speed: f32,
        amplitude: f32,
        max_age: f32,
        has_foam: bool,
    ) -> Self {
        Self {
            center_x,
            center_z,
            water_y,
            radius,
            max_radius,
            speed,
            amplitude,
            age: 0.0,
            max_age,
            has_foam,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.age < self.max_age
    }

    #[inline]
    pub fn step(&mut self, dt: f32) {
        self.age += dt;
        self.radius += self.speed * dt;
    }

    /// Continuous harmonic envelope: 0.0 on spawn, smoothly rises, then smoothly decays to 0.0.
    /// Completely eliminates sudden "circle pops" on appearance or expiration.
    #[inline]
    pub fn envelope(&self) -> f32 {
        if self.max_age <= 0.001 {
            return 0.0;
        }
        let p = (self.age / self.max_age).clamp(0.0, 1.0);
        // Smooth cubic decay: (1 - p)^3 reaches exactly 0.0 with 0 slope at p=1.0
        let decay = (1.0 - p) * (1.0 - p) * (1.0 - p);
        // Smooth attack over the first 12% of lifetime: smoothstep from 0 to 1
        let attack_p = (p / 0.12).min(1.0);
        let attack = attack_p * attack_p * (3.0 - 2.0 * attack_p);
        attack * decay
    }
}

#[derive(Clone, Debug)]
pub struct WakeNode {
    pub pos_x: f64,
    pub pos_z: f64,
    pub water_y: f64,
    pub dir_x: f32,
    pub dir_z: f32,
    pub left_x: f32,
    pub left_z: f32,
    pub half_width: f32,
    pub spread_speed: f32,
    pub amplitude: f32,
    pub age: f32,
    pub max_age: f32,
    pub is_boat: bool,
}

impl WakeNode {
    pub fn new(
        pos_x: f64,
        pos_z: f64,
        water_y: f64,
        dir_x: f32,
        dir_z: f32,
        speed: f32,
        is_boat: bool,
    ) -> Self {
        let left_x = -dir_z;
        let left_z = dir_x;

        let (half_width, spread_speed, amplitude, max_age) = if is_boat {
            let amp = (speed * 0.40).clamp(0.50, 1.6);
            (0.42f32, 0.65f32, amp, 3.0f32)
        } else {
            let amp = (speed * 0.40).clamp(0.40, 1.3);
            (0.22f32, 0.50f32, amp, 2.2f32)
        };

        Self {
            pos_x,
            pos_z,
            water_y,
            dir_x,
            dir_z,
            left_x,
            left_z,
            half_width,
            spread_speed,
            amplitude,
            age: 0.0,
            max_age,
            is_boat,
        }
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.age < self.max_age
    }

    #[inline]
    pub fn step(&mut self, dt: f32) {
        self.age += dt;
        self.half_width += self.spread_speed * dt;
    }

    #[inline]
    pub fn envelope(&self) -> f32 {
        if self.max_age <= 0.001 {
            return 0.0;
        }
        let p = (self.age / self.max_age).clamp(0.0, 1.0);
        let decay = (1.0 - p) * (1.0 - p);
        let attack = (p / 0.05).min(1.0);
        attack * decay
    }
}

pub struct WaterSimulation {
    pub rings: Vec<RippleRing>,
    pub player_wakes: Vec<WakeNode>,
    pub boat_wakes: Vec<WakeNode>,
    pub max_rings: usize,
    pub max_wake_nodes: usize,
    pub segments: usize,
    pub intensity: f32,
}

impl WaterSimulation {
    pub fn new() -> Self {
        Self {
            rings: Vec::with_capacity(DEFAULT_MAX_RINGS),
            player_wakes: Vec::with_capacity(DEFAULT_MAX_WAKE_NODES),
            boat_wakes: Vec::with_capacity(DEFAULT_MAX_WAKE_NODES),
            max_rings: DEFAULT_MAX_RINGS,
            max_wake_nodes: DEFAULT_MAX_WAKE_NODES,
            segments: 24, // smooth 24-gon circle
            intensity: 1.0,
        }
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.4, 2.5);
    }

    pub fn set_max_rings(&mut self, max: usize) {
        self.max_rings = max;
        if self.rings.len() > max {
            self.rings.truncate(max);
        }
    }

    pub fn set_segments(&mut self, segments: usize) {
        self.segments = segments.clamp(16, 32);
    }

    pub fn spawn_ring(
        &mut self,
        center_x: f64,
        center_z: f64,
        water_y: f64,
        initial_radius: f32,
        max_radius: f32,
        speed: f32,
        amplitude: f32,
        max_age: f32,
        has_foam: bool,
    ) {
        if self.rings.len() >= self.max_rings {
            // Drop the ring with the lowest visible amplitude to avoid visible pops
            let mut min_idx = 0;
            let mut min_val = f32::MAX;
            for (idx, r) in self.rings.iter().enumerate() {
                let val = r.envelope() * r.amplitude;
                if val < min_val {
                    min_val = val;
                    min_idx = idx;
                }
            }
            self.rings.swap_remove(min_idx);
        }
        self.rings.push(RippleRing::new(
            center_x,
            center_z,
            water_y,
            initial_radius,
            max_radius,
            speed,
            amplitude * self.intensity,
            max_age,
            has_foam,
        ));
    }

    pub fn spawn_splash_burst(
        &mut self,
        center_x: f64,
        center_z: f64,
        water_y: f64,
        impact: f32,
    ) {
        let amp = (impact * 0.70).clamp(0.35, 1.4);
        // Staggered concentric ripple rings radiating outward smoothly
        self.spawn_ring(center_x, center_z, water_y, 0.08, 2.8, 1.35, amp, 2.2, false);
        self.spawn_ring(center_x, center_z, water_y, 0.02, 1.8, 0.95, amp * 0.65, 2.0, false);
    }

    pub fn add_wake_node(
        &mut self,
        pos_x: f64,
        pos_z: f64,
        water_y: f64,
        dir_x: f32,
        dir_z: f32,
        speed: f32,
        is_boat: bool,
    ) {
        let (wake_vec, min_dist) = if is_boat {
            (&mut self.boat_wakes, 0.35f64)
        } else {
            (&mut self.player_wakes, 0.18f64)
        };

        if let Some(last) = wake_vec.last() {
            let dx = pos_x - last.pos_x;
            let dz = pos_z - last.pos_z;
            let dist_sq = dx * dx + dz * dz;
            if dist_sq < min_dist * min_dist {
                return;
            }
        }

        if wake_vec.len() >= self.max_wake_nodes {
            wake_vec.remove(0);
        }

        wake_vec.push(WakeNode::new(
            pos_x,
            pos_z,
            water_y,
            dir_x,
            dir_z,
            speed * self.intensity,
            is_boat,
        ));
    }

    pub fn step(&mut self, dt: f32) -> bool {
        let mut had_active = false;

        // Step and prune ripple rings
        for ring in &mut self.rings {
            ring.step(dt);
            had_active = true;
        }
        self.rings.retain(|r| r.is_alive());

        // Step and prune player wake nodes
        for node in &mut self.player_wakes {
            node.step(dt);
            had_active = true;
        }
        self.player_wakes.retain(|n| n.is_alive());

        // Step and prune boat wake nodes
        for node in &mut self.boat_wakes {
            node.step(dt);
            had_active = true;
        }
        self.boat_wakes.retain(|n| n.is_alive());

        had_active
    }

    pub fn any_active(&self) -> bool {
        !self.rings.is_empty() || !self.player_wakes.is_empty() || !self.boat_wakes.is_empty()
    }
}

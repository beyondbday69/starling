// Geometric water wave and wake simulation engine (v3.0).
//
// Replaces the coarse 32x32 heightfield grid with smooth analytical fluid geometry:
// 1. Concentric Ripple Rings: Smooth circular expanding torus waves for footsteps, drops, splashes, and oars.
// 2. Continuous Kelvin V-Wakes: Trailing hydrodynamic wake ribbons flaring at 19.47 degrees with foam.
// 3. Zero-grid architecture: Eliminates low-poly triangulated origami artifacts, square boundary popping,
//    and untextured Gouraud shading creases.

pub const DEFAULT_MAX_RINGS: usize = 18;
pub const DEFAULT_MAX_WAKE_NODES: usize = 32;

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
        self.age < self.max_age && self.radius < self.max_radius
    }

    #[inline]
    pub fn step(&mut self, dt: f32) {
        self.age += dt;
        self.radius += self.speed * dt;
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
            let amp = (speed * 2.2).clamp(0.4, 1.8);
            (0.65f32, 1.15f32, amp, 3.6f32)
        } else {
            let amp = (speed * 1.5).clamp(0.3, 1.4);
            (0.30f32, 0.72f32, amp, 2.5f32)
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
}

pub struct WaterSimulation {
    pub rings: Vec<RippleRing>,
    pub wake_nodes: Vec<WakeNode>,
    pub max_rings: usize,
    pub max_wake_nodes: usize,
    pub segments: usize,
    pub intensity: f32,
}

impl WaterSimulation {
    pub fn new() -> Self {
        Self {
            rings: Vec::with_capacity(DEFAULT_MAX_RINGS),
            wake_nodes: Vec::with_capacity(DEFAULT_MAX_WAKE_NODES),
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
            // Drop oldest ring
            self.rings.remove(0);
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
        let amp = (impact * 0.85).clamp(0.4, 1.8);
        // Staggered concentric ripple rings radiating outward
        self.spawn_ring(center_x, center_z, water_y, 0.15, 2.2, 1.85, amp, 2.4, true);
        self.spawn_ring(center_x, center_z, water_y, 0.05, 1.6, 1.40, amp * 0.75, 2.0, false);
        self.spawn_ring(center_x, center_z, water_y, 0.02, 1.1, 1.05, amp * 0.50, 1.6, false);
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
        if let Some(last) = self.wake_nodes.last() {
            let dx = pos_x - last.pos_x;
            let dz = pos_z - last.pos_z;
            let dist_sq = dx * dx + dz * dz;
            let min_dist = if is_boat { 0.35 } else { 0.20 };
            if dist_sq < min_dist * min_dist {
                return;
            }
        }

        if self.wake_nodes.len() >= self.max_wake_nodes {
            self.wake_nodes.remove(0);
        }

        self.wake_nodes.push(WakeNode::new(
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

        // Step and prune wake nodes
        for node in &mut self.wake_nodes {
            node.step(dt);
            had_active = true;
        }
        self.wake_nodes.retain(|n| n.is_alive());

        had_active
    }

    pub fn any_active(&self) -> bool {
        !self.rings.is_empty() || !self.wake_nodes.is_empty()
    }
}

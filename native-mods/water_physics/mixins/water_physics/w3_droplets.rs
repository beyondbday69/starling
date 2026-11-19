// w3_droplets: fixed-capacity, allocation-free ballistic droplet particle pool.
//
// The pool owns a compile-time array of `CAPACITY` particles. `spawn` writes into
// the next free slot, `update` integrates every live particle and compacts dead
// ones with a swap-remove so `iter()` can always return one contiguous slice.
// Nothing in the hot path allocates; all state is owned by the caller.
//
// Coordinate system: X/Z horizontal, Y up. Units: metres and seconds.
// Water surface height is `water_block_y + 8.0 / 9.0` (flowing water top).

/// Maximum number of simultaneously live droplets.
pub const CAPACITY: usize = 256;

/// Default exponential velocity damping rate (per second).
pub const DEFAULT_DRAG_PER_SECOND: f32 = 1.6;

/// Largest integration step accepted by `update`; larger frame gaps are clamped
/// so a particle cannot tunnel through the ground or water surface.
pub const MAX_FRAME_DT: f32 = 0.1;

/// One simulated droplet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// World position [x, y, z] in metres.
    pub pos: [f32; 3],
    /// World velocity [vx, vy, vz] in metres per second.
    pub vel: [f32; 3],
    /// Seconds of life remaining.
    pub life: f32,
    /// Lifetime at spawn, useful for fade/scale ramps.
    pub max_life: f32,
    /// Approximate droplet radius in metres.
    pub size: f32,
}

impl Particle {
    /// Empty slot value used to initialise the fixed backing array.
    pub const EMPTY: Self = Self {
        pos: [0.0; 3],
        vel: [0.0; 3],
        life: 0.0,
        max_life: 0.0,
        size: 0.0,
    };

    /// Normalised remaining life in [0, 1]; zero for expired particles.
    #[inline]
    pub fn alpha(&self) -> f32 {
        if self.max_life <= 0.0 || self.life <= 0.0 {
            return 0.0;
        }
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

/// Fixed-capacity pool of ballistic water droplets.
///
/// Live particles always occupy `particles[0..count]`. Use `iter()` to read
/// them without exposing the unused tail of the array.
pub struct DropletPool {
    particles: [Particle; CAPACITY],
    count: usize,
    /// Solid floor height; droplets die when their lower edge reaches it.
    pub ground_y: f32,
    /// Optional water surface height (water block Y + 8/9). Droplets falling
    /// back through this plane are removed; rising droplets are not.
    pub water_surface_y: Option<f32>,
    drag_per_second: f32,
}

impl Default for DropletPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl DropletPool {
    /// Creates an empty pool with `ground_y = 0.0`, no water plane and the
    /// default drag rate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            particles: [Particle::EMPTY; CAPACITY],
            count: 0,
            ground_y: 0.0,
            water_surface_y: None,
            drag_per_second: DEFAULT_DRAG_PER_SECOND,
        }
    }

    /// Sets the solid floor height.
    #[inline]
    pub fn with_ground_y(mut self, y: f32) -> Self {
        if y.is_finite() {
            self.ground_y = y;
        }
        self
    }

    /// Enables a water surface kill plane. Pass `water_block_y + 8.0 / 9.0`.
    #[inline]
    pub fn with_water_surface_y(mut self, y: f32) -> Self {
        if y.is_finite() {
            self.water_surface_y = Some(y);
        }
        self
    }

    /// Overrides the exponential drag rate. Negative or non-finite values are
    /// ignored, keeping the previous rate.
    #[inline]
    pub fn with_drag(mut self, drag_per_second: f32) -> Self {
        if drag_per_second.is_finite() && drag_per_second >= 0.0 {
            self.drag_per_second = drag_per_second;
        }
        self
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Live particles only. This is a contiguous slice, so iterating it never
    /// touches the unused capacity.
    #[inline]
    pub fn iter(&self) -> &[Particle] {
        &self.particles[..self.count]
    }

    /// Removes every live particle without deallocating the backing array.
    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Adds one droplet. Returns `false` when the pool is full or when `life`
    /// is invalid, in which case nothing is stored.
    #[inline]
    pub fn spawn(&mut self, pos: [f32; 3], vel: [f32; 3], life: f32, size: f32) -> bool {
        if self.count >= CAPACITY || !life.is_finite() || life <= 0.0 {
            return false;
        }
        if !is_finite_vec3(pos) || !is_finite_vec3(vel) {
            return false;
        }
        let life = life.min(MAX_FRAME_DT * 10_000.0);
        let size = if size.is_finite() { size.max(0.0) } else { 0.0 };
        self.particles[self.count] = Particle {
            pos,
            vel,
            life,
            max_life: life,
            size,
        };
        self.count += 1;
        true
    }

    /// Advances all live droplets by `dt` seconds under constant `gravity`
    /// (negative Y accel for Earth-like gravity) plus exponential air drag.
    /// Expired droplets and droplets that hit the ground or fall through the
    /// water surface are removed in place.
    pub fn update(&mut self, dt: f32, gravity: f32) {
        if self.count == 0 || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(MAX_FRAME_DT);
        let gravity = if gravity.is_finite() { gravity } else { 0.0 };
        let damping = (-self.drag_per_second * dt).exp().clamp(0.0, 1.0);

        let mut i = 0usize;
        while i < self.count {
            let p = &mut self.particles[i];
            p.life -= dt;
            if p.life <= 0.0 {
                self.swap_remove(i);
                continue;
            }

            p.vel[0] *= damping;
            p.vel[1] = p.vel[1] * damping + gravity * dt;
            p.vel[2] *= damping;
            p.pos[0] += p.vel[0] * dt;
            p.pos[1] += p.vel[1] * dt;
            p.pos[2] += p.vel[2] * dt;

            let hit_ground = p.pos[1] - p.size * 0.5 <= self.ground_y;
            let hit_water = match self.water_surface_y {
                Some(surface_y) => p.pos[1] <= surface_y && p.vel[1] <= 0.0,
                None => false,
            };
            if hit_ground || hit_water {
                self.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }

    /// O(1) removal that keeps the live prefix contiguous.
    #[inline]
    fn swap_remove(&mut self, i: usize) {
        let last = self.count - 1;
        if i != last {
            self.particles[i] = self.particles[last];
        }
        self.count = last;
    }
}

#[inline]
fn is_finite_vec3(v: [f32; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_steps_ballistically_and_kills_on_life_ground_and_water() {
        let mut pool = DropletPool::new()
            .with_ground_y(0.0)
            .with_water_surface_y(1.0)
            .with_drag(0.0);

        assert!(pool.spawn([0.0, 5.0, 0.0], [1.0, 2.0, 0.0], 1.0, 0.1));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.iter().len(), 1);
        assert_eq!(pool.iter()[0].pos, [0.0, 5.0, 0.0]);
        assert_eq!(pool.iter()[0].alpha(), 1.0);

        pool.update(0.1, -10.0);
        let p = pool.iter()[0];
        assert!((p.pos[0] - 0.1).abs() < 1.0e-5, "x = {}", p.pos[0]);
        assert!((p.pos[1] - 5.1).abs() < 1.0e-4, "y = {}", p.pos[1]);
        assert!((p.life - 0.9).abs() < 1.0e-5);
        assert!(p.alpha() < 1.0 && p.alpha() > 0.0);

        // Life expiry removes the particle even when it never touched a plane.
        pool.clear();
        assert!(pool.spawn([0.0, 5.0, 0.0], [0.0, 0.0, 0.0], 0.05, 0.1));
        pool.update(0.1, -10.0);
        assert!(pool.is_empty(), "expired droplet should be gone");

        // Ground kill, tested without a water plane competing to remove it.
        let mut ground_pool = DropletPool::new().with_ground_y(0.0).with_drag(0.0);
        assert!(ground_pool.spawn([0.0, 0.5, 0.0], [0.0, -8.0, 0.0], 2.0, 0.2));
        ground_pool.update(0.1, -9.81);
        assert!(ground_pool.is_empty(), "droplet below ground should be gone");

        // Water kill only while descending; an ascending droplet survives.
        assert!(pool.spawn([0.0, 1.5, 0.0], [0.0, -8.0, 0.0], 2.0, 0.1));
        pool.update(0.1, -9.81);
        assert!(pool.is_empty(), "droplet falling through water should be gone");
        assert!(pool.spawn([0.0, 1.2, 0.0], [0.0, 3.0, 0.0], 2.0, 0.1));
        pool.update(0.1, -9.81);
        assert_eq!(pool.len(), 1, "rising droplet above water must survive");
    }

    #[test]
    fn capacity_is_capped_and_slots_reopen_after_deaths() {
        let mut pool = DropletPool::new();
        assert_eq!(pool.capacity(), CAPACITY);

        for i in 0..CAPACITY {
            assert!(
                pool.spawn(
                    [i as f32 + 1.0, 10.0, 0.0],
                    [0.0, 0.0, 0.0],
                    5.0,
                    0.1,
                ),
                "spawn {} should fit",
                i
            );
        }
        assert_eq!(pool.len(), CAPACITY);
        assert_eq!(pool.iter().len(), CAPACITY);
        assert!(!pool.spawn([0.0, 10.0, 0.0], [0.0, 0.0, 0.0], 1.0, 0.1));
        assert_eq!(pool.len(), CAPACITY, "full pool must reject new spawns");

        // Expire one droplet (short life) and confirm its slot is reusable.
        pool.clear();
        assert!(pool.spawn([0.0, 10.0, 0.0], [0.0, 0.0, 0.0], 0.05, 0.1));
        pool.update(0.1, -9.81);
        assert!(pool.is_empty());
        assert!(pool.spawn([0.0, 10.0, 0.0], [0.0, 0.0, 0.0], 1.0, 0.1));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn drag_reduces_horizontal_speed_without_allocating() {
        let mut no_drag = DropletPool::new().with_drag(0.0);
        let mut heavy_drag = DropletPool::new().with_drag(8.0);
        for pool in [&mut no_drag, &mut heavy_drag] {
            assert!(pool.spawn([0.0, 5.0, 0.0], [4.0, 0.0, 0.0], 1.0, 0.1));
            pool.update(0.1, -9.81);
        }
        let free = no_drag.iter()[0].vel[0];
        let damped = heavy_drag.iter()[0].vel[0];
        assert!((free - 4.0).abs() < 1.0e-5, "free vx = {}", free);
        assert!(damped < free && damped > 0.0, "damped vx = {}", damped);
    }
}

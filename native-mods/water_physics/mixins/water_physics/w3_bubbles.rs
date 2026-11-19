// w3_bubbles: fixed-capacity, allocation-free pool of underwater bubbles.
//
// Bubbles rise under size-dependent buoyancy and trace a deterministic,
// seed-driven horizontal wobble. All state is owned by the caller; nothing in
// the hot path allocates and no globals or mutexes are used.
//
// Coordinate system: X/Z horizontal, Y up. Units: metres and seconds.
// Water surface height is `water_block_y + 8.0 / 9.0` (flowing water top).

/// Maximum number of simultaneously live bubbles.
pub const CAPACITY: usize = 128;

/// Largest integration step accepted by `update`; larger frame gaps are
/// clamped so a bubble cannot jump across the scene after a stall.
pub const MAX_FRAME_DT: f32 = 0.1;

/// Longest accepted lifetime in seconds.
pub const MAX_LIFE: f32 = 60.0;

/// Default lifetime used when a bubble has no surface plane to pop against.
pub const DEFAULT_LIFE: f32 = 4.0;

/// Rise speed of an infinitesimally small bubble, in metres per second.
pub const MIN_RISE_SPEED: f32 = 0.35;

/// Terminal rise speed approached by large bubbles, in metres per second.
pub const MAX_RISE_SPEED: f32 = 2.0;

/// Bubble radius at which rise speed reaches ~86% of its terminal value.
pub const RISE_SPEED_SCALE: f32 = 0.08;

/// Peak magnitude of the horizontal wobble offset, in metres.
pub const WOBBLE_AMPLITUDE: f32 = 0.06;

#[inline]
fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

// SplitMix64 finalizer: tiny, deterministic, no allocations or globals.
#[inline]
fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[inline]
fn phase(seed: u64, salt: u64) -> f32 {
    let v = mix64(seed ^ salt);
    (v >> 32) as f32 * (1.0 / 4_294_967_296.0) * std::f32::consts::TAU
}

/// Buoyant rise speed in metres per second for a bubble of `size` metres.
///
/// Small bubbles rise slowly because surface tension keeps them nearly rigid;
/// larger bubbles deform and accelerate toward a terminal speed. Invalid or
/// non-positive sizes return zero.
#[inline]
pub fn bubble_rise_speed(size: f32) -> f32 {
    let s = finite_or(size, 0.0);
    if s <= 0.0 {
        return 0.0;
    }
    let t = 1.0 - (-s / RISE_SPEED_SCALE).exp();
    MIN_RISE_SPEED + (MAX_RISE_SPEED - MIN_RISE_SPEED) * t
}

/// Deterministic horizontal wobble offset `[dx, dz]` in metres at time `t`.
///
/// Two incommensurate sine bands with seed-derived phases avoid a mechanical,
/// repeating zig-zag. Each component stays within `[-WOBBLE_AMPLITUDE,
/// WOBBLE_AMPLITUDE]`. Non-finite `t` yields a zero offset.
#[inline]
pub fn bubble_wobble(t: f32, seed: u64) -> [f32; 2] {
    if !t.is_finite() {
        return [0.0, 0.0];
    }
    let time = t;
    let p1 = phase(seed, 0x9e37_79b9_7f4a_7c15);
    let p2 = phase(seed, 0xbf58_476d_1ce4_e5b9);
    let dx = (time * 5.7 + p1).sin() * 0.62 + (time * 11.3 + p2).sin() * 0.38;
    let dz = (time * 4.9 + p2).cos() * 0.58 + (time * 13.1 + p1).cos() * 0.42;
    [dx * WOBBLE_AMPLITUDE, dz * WOBBLE_AMPLITUDE]
}

/// One live underwater bubble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bubble {
    /// Current world position `[x, y, z]` including the wobble offset.
    pub pos: [f32; 3],
    /// Approximate radius in metres, useful for rendering.
    pub size: f32,
    /// Seconds elapsed since spawn.
    pub age: f32,
    /// Lifetime in seconds.
    pub life: f32,
    /// Per-bubble seed used by [`bubble_wobble`].
    pub seed: u64,
    /// Unwobbled world position; wobble is applied on top of this each frame.
    base: [f32; 3],
}

impl Bubble {
    /// Empty-slot value used to initialise the fixed backing array.
    pub const EMPTY: Self = Self {
        pos: [0.0; 3],
        size: 0.0,
        age: 0.0,
        life: 0.0,
        seed: 0,
        base: [0.0; 3],
    };

    /// Current buoyant rise speed in metres per second.
    #[inline]
    pub fn rise_speed(&self) -> f32 {
        bubble_rise_speed(self.size)
    }

    /// Normalised remaining life in `[0, 1]`, zero once expired.
    #[inline]
    pub fn alpha(&self) -> f32 {
        if self.life <= 0.0 || self.age <= 0.0 || self.age >= self.life {
            return 0.0;
        }
        (1.0 - self.age / self.life).clamp(0.0, 1.0)
    }
}

/// Fixed-capacity pool of underwater bubbles.
///
/// Live bubbles always occupy `bubbles[0..count]`; `iter()` exposes only that
/// contiguous prefix, so the unused tail is never read by callers.
pub struct BubblePool {
    bubbles: [Bubble; CAPACITY],
    count: usize,
    next_seed: u64,
    /// Optional water-surface height. Bubbles reaching it are popped.
    pub surface_y: Option<f32>,
}

impl Default for BubblePool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl BubblePool {
    /// Creates an empty pool with no surface kill plane.
    #[inline]
    pub const fn new() -> Self {
        Self {
            bubbles: [Bubble::EMPTY; CAPACITY],
            count: 0,
            next_seed: 1,
            surface_y: None,
        }
    }

    /// Sets the water-surface height at which bubbles pop.
    /// Pass `water_block_y + 8.0 / 9.0`. Non-finite values are ignored.
    #[inline]
    pub fn with_surface_y(mut self, y: f32) -> Self {
        if y.is_finite() {
            self.surface_y = Some(y);
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

    /// Live bubbles only, as one contiguous slice.
    #[inline]
    pub fn iter(&self) -> &[Bubble] {
        &self.bubbles[..self.count]
    }

    /// Removes every live bubble without deallocating the backing array.
    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Adds one bubble at world `pos` with radius `size` in metres.
    ///
    /// Returns `false` when the pool is full or when any input is invalid;
    /// nothing is stored in that case. Non-positive sizes are also rejected.
    #[inline]
    pub fn spawn(&mut self, pos: [f32; 3], size: f32) -> bool {
        if self.count >= CAPACITY || !is_finite_vec3(pos) || !size.is_finite() || size <= 0.0 {
            return false;
        }
        let seed = mix64(self.next_seed ^ (size.to_bits() as u64));
        self.next_seed = self.next_seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let life = match self.surface_y {
            Some(surface) if surface > pos[1] => {
                let travel = (surface - pos[1]) / bubble_rise_speed(size);
                travel.clamp(0.1, MAX_LIFE)
            }
            _ => DEFAULT_LIFE,
        };
        let w = bubble_wobble(0.0, seed);
        self.bubbles[self.count] = Bubble {
            pos: [pos[0] + w[0], pos[1], pos[2] + w[1]],
            size,
            age: 0.0,
            life,
            seed,
            base: pos,
        };
        self.count += 1;
        true
    }

    /// Advances every live bubble by `dt` seconds under size-dependent
    /// buoyancy. Bubbles expire at the end of their life or when they reach
    /// the configured surface height.
    pub fn update(&mut self, dt: f32) {
        if self.count == 0 || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(MAX_FRAME_DT);
        let surface = self.surface_y.filter(|y| y.is_finite());

        let mut i = 0usize;
        while i < self.count {
            let b = &mut self.bubbles[i];
            b.age += dt;
            if b.age >= b.life {
                self.swap_remove(i);
                continue;
            }

            b.base[1] += bubble_rise_speed(b.size) * dt;
            let w = bubble_wobble(b.age, b.seed);
            b.pos = [b.base[0] + w[0], b.base[1], b.base[2] + w[1]];

            if let Some(surface_y) = surface {
                if b.pos[1] >= surface_y {
                    self.swap_remove(i);
                    continue;
                }
            }
            i += 1;
        }
    }

    /// O(1) removal that keeps the live prefix contiguous.
    #[inline]
    fn swap_remove(&mut self, i: usize) {
        let last = self.count - 1;
        if i != last {
            self.bubbles[i] = self.bubbles[last];
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
    fn rise_speed_and_wobble_are_deterministic_and_bounded() {
        assert_eq!(bubble_rise_speed(0.0), 0.0);
        assert_eq!(bubble_rise_speed(-1.0), 0.0);
        assert_eq!(bubble_rise_speed(f32::NAN), 0.0);
        assert_eq!(bubble_rise_speed(f32::INFINITY), 0.0);
        assert!(bubble_rise_speed(1000.0) <= MAX_RISE_SPEED);

        let small = bubble_rise_speed(0.01);
        let medium = bubble_rise_speed(0.08);
        let large = bubble_rise_speed(0.30);
        assert!(small > MIN_RISE_SPEED && small < medium);
        assert!(medium < large && large <= MAX_RISE_SPEED);

        let a = bubble_wobble(1.25, 42);
        let b = bubble_wobble(1.25, 42);
        let other_seed = bubble_wobble(1.25, 43);
        assert_eq!(a, b, "wobble must be deterministic");
        assert_ne!(a, other_seed, "different seeds should differ");
        assert_eq!(bubble_wobble(f32::NAN, 42), [0.0, 0.0]);
        for i in 0..=40 {
            let t = i as f32 * 0.1;
            let w = bubble_wobble(t, 7);
            for v in w {
                assert!(v.is_finite());
                assert!(v.abs() <= WOBBLE_AMPLITUDE + 1.0e-6, "wobble {}", v);
            }
        }
    }

    #[test]
    fn spawn_rises_wobbles_and_pops_at_surface() {
        let mut pool = BubblePool::new().with_surface_y(3.0);
        assert!(pool.spawn([1.0, 1.0, 2.0], 0.1));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.iter().len(), 1);
        let b = pool.iter()[0];
        assert!((b.base[0] - 1.0).abs() < 1.0e-6);
        assert!((b.base[1] - 1.0).abs() < 1.0e-6);
        assert!(b.pos[0] != 1.0 || b.pos[2] != 2.0, "initial wobble expected");
        assert_eq!(b.age, 0.0);
        assert_eq!(b.alpha(), 0.0);

        pool.update(0.1);
        let b = pool.iter()[0];
        let expected_y = 1.0 + bubble_rise_speed(0.1) * 0.1;
        assert!((b.base[1] - expected_y).abs() < 1.0e-5, "y = {}", b.base[1]);
        let w = bubble_wobble(0.1, b.seed);
        assert!((b.pos[0] - (1.0 + w[0])).abs() < 1.0e-6);
        assert!((b.pos[2] - (2.0 + w[1])).abs() < 1.0e-6);
        assert!(b.alpha() > 0.0 && b.alpha() <= 1.0);

        // Invalid updates leave state untouched.
        pool.update(f32::NAN);
        assert_eq!(pool.iter()[0].age, 0.1);
        pool.update(-1.0);
        assert_eq!(pool.iter()[0].age, 0.1);

        // Rise until the surface pops the bubble.
        let mut guard = 0;
        while !pool.is_empty() && guard < 128 {
            pool.update(0.1);
            guard += 1;
        }
        assert!(pool.is_empty(), "bubble should pop at the surface");
    }

    #[test]
    fn capacity_is_capped_slots_reopen_and_invalid_spawns_are_rejected() {
        let mut pool = BubblePool::new();
        assert_eq!(pool.capacity(), CAPACITY);

        assert!(!pool.spawn([f32::NAN, 0.0, 0.0], 0.1));
        assert!(!pool.spawn([0.0, 0.0, 0.0], 0.0));
        assert!(!pool.spawn([0.0, 0.0, 0.0], -0.1));
        assert!(pool.is_empty());

        for i in 0..CAPACITY {
            assert!(pool.spawn([i as f32, -2.0, 0.0], 0.05), "spawn {} should fit", i);
        }
        assert_eq!(pool.len(), CAPACITY);
        assert!(!pool.spawn([0.0, -2.0, 0.0], 0.05));
        assert_eq!(pool.len(), CAPACITY);

        // With no surface plane, bubbles expire by lifetime.
        let mut guard = 0;
        while pool.len() == CAPACITY && guard < 16 {
            pool.update(0.1);
            guard += 1;
        }
        assert!(pool.len() < CAPACITY, "a bubble should have expired");
        assert!(pool.spawn([999.0, -2.0, 0.0], 0.05));

        pool.clear();
        assert!(pool.is_empty());
        assert_eq!(pool.iter().len(), 0);
    }
}

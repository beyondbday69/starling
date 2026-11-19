// w3_foamflecks: fixed-capacity pool of floating foam flecks advected by a
// horizontal flow vector with a short fade-in and a slow fade-out.
//
// Pure Rust, allocation-free and deterministic. The caller owns all state and
// passes the water-surface Y at spawn; flecks keep that height (they float,
// they do not sink) while X/Z are advected by the flow given to `update`.
//
// Coordinate system: X/Z horizontal, Y up. Units: metres and seconds.
// Water surface height is `water_block_y + 8.0 / 9.0` (flowing water top).

/// Maximum number of simultaneously live foam flecks.
pub const CAPACITY: usize = 256;

/// Largest integration step accepted by `update`; larger frame gaps are clamped
/// so a fleck cannot jump across the scene after a stall.
pub const MAX_FRAME_DT: f32 = 0.1;

/// Longest accepted lifetime in seconds (guards against runaway `age` values).
pub const MAX_LIFE: f32 = 600.0;

/// Fraction of life used for the short fade-in that avoids a spawn pop.
pub const FADE_IN_FRACTION: f32 = 0.05;

/// Fraction of life before the slow fade-out begins.
pub const FADE_START_FRACTION: f32 = 0.20;

#[inline]
fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

#[inline]
fn smoothstep01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
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

/// Deterministic fleck radius in metres, derived from the spawn X/Z location.
#[inline]
fn fleck_size(x: f32, z: f32) -> f32 {
    let bits = (x.to_bits() as u64) ^ ((z.to_bits() as u64) << 17);
    0.030 + ((mix64(bits) >> 40) as f32 / 16_777_216.0) * 0.055
}

/// One live foam fleck.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fleck {
    /// World position [x, y, z] in metres. `y` is the water-surface height.
    pub pos: [f32; 3],
    /// Seconds elapsed since spawn.
    pub age: f32,
    /// Lifetime at spawn, in seconds.
    pub life: f32,
    /// Approximate radius in metres, useful for rendering.
    pub size: f32,
}

impl Fleck {
    /// Empty-slot value used to initialise the fixed backing array.
    pub const EMPTY: Self = Self {
        pos: [0.0; 3],
        age: 0.0,
        life: 0.0,
        size: 0.0,
    };

    /// Convenience wrapper around [`fleck_alpha`] for this fleck.
    #[inline]
    pub fn alpha(&self) -> f32 {
        fleck_alpha(self.age, self.life)
    }
}

/// Opacity of a foam fleck in [0, 1].
///
/// A very short attack avoids a spawn pop; the release then stays high for
/// most of the life and decays slowly to zero at `age == life`. `age` and
/// `life` are seconds. Invalid or expired inputs return zero.
#[inline]
pub fn fleck_alpha(age: f32, life: f32) -> f32 {
    let a = finite_or(age, 0.0);
    let l = finite_or(life, 0.0);
    if l <= 0.0 || a <= 0.0 || a >= l {
        return 0.0;
    }
    let t = (a / l).clamp(0.0, 1.0);
    let attack = smoothstep01(t / FADE_IN_FRACTION);
    let release = 1.0 - smoothstep01((t - FADE_START_FRACTION) / (1.0 - FADE_START_FRACTION));
    (attack * release).clamp(0.0, 1.0)
}

/// Fixed-capacity pool of floating foam flecks.
///
/// Live flecks always occupy `flecks[0..count]`; `iter()` exposes only that
/// contiguous prefix, so the unused tail is never read by callers.
pub struct FoamPool {
    flecks: [Fleck; CAPACITY],
    count: usize,
}

impl Default for FoamPool {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl FoamPool {
    /// Creates an empty pool without allocating.
    #[inline]
    pub const fn new() -> Self {
        Self {
            flecks: [Fleck::EMPTY; CAPACITY],
            count: 0,
        }
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

    /// Live flecks only, as one contiguous slice.
    #[inline]
    pub fn iter(&self) -> &[Fleck] {
        &self.flecks[..self.count]
    }

    /// Removes every live fleck without deallocating the backing array.
    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Adds one fleck at `(x, y, z)` with a lifetime of `life` seconds.
    ///
    /// Pass the water-surface height as `y`. Returns `false` when the pool is
    /// full or when any input is invalid; nothing is stored in that case.
    #[inline]
    pub fn spawn(&mut self, x: f32, y: f32, z: f32, life: f32) -> bool {
        if self.count >= CAPACITY
            || !x.is_finite()
            || !y.is_finite()
            || !z.is_finite()
            || !life.is_finite()
            || life <= 0.0
        {
            return false;
        }
        let life = life.min(MAX_LIFE);
        let size = fleck_size(x, z);
        self.flecks[self.count] = Fleck {
            pos: [x, y, z],
            age: 0.0,
            life,
            size,
        };
        self.count += 1;
        true
    }

    /// Advances every live fleck by `dt` seconds under horizontal flow
    /// `(flow_x, flow_z)`. Ageing is independent of the flow, so a stationary
    /// fleck still fades and is removed when its life expires.
    ///
    /// `Y` is untouched: flecks ride on the surface height supplied at spawn.
    /// Non-finite flow components are treated as zero.
    pub fn update(&mut self, dt: f32, flow_x: f32, flow_z: f32) {
        if self.count == 0 || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(MAX_FRAME_DT);
        let fx = finite_or(flow_x, 0.0);
        let fz = finite_or(flow_z, 0.0);

        let mut i = 0usize;
        while i < self.count {
            let f = &mut self.flecks[i];
            f.age += dt;
            if f.age >= f.life {
                self.swap_remove(i);
                continue;
            }
            f.pos[0] += fx * dt;
            f.pos[2] += fz * dt;
            i += 1;
        }
    }

    /// O(1) removal that keeps the live prefix contiguous.
    #[inline]
    fn swap_remove(&mut self, i: usize) {
        let last = self.count - 1;
        if i != last {
            self.flecks[i] = self.flecks[last];
        }
        self.count = last;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_advects_with_flow_and_expires_after_slow_fade() {
        let mut pool = FoamPool::new();
        assert!(pool.spawn(1.0, 2.0, 3.0, 1.0));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.iter().len(), 1);
        assert_eq!(pool.iter()[0].pos, [1.0, 2.0, 3.0]);
        assert_eq!(pool.iter()[0].alpha(), 0.0);

        pool.update(0.1, 0.5, -0.25);
        let young = pool.iter()[0];
        assert!((young.pos[0] - 1.05).abs() < 1.0e-5, "x = {}", young.pos[0]);
        assert!((young.pos[1] - 2.0).abs() < 1.0e-6, "y = {}", young.pos[1]);
        assert!((young.pos[2] - 2.975).abs() < 1.0e-5, "z = {}", young.pos[2]);
        assert!((young.age - 0.1).abs() < 1.0e-6);
        assert!(young.alpha() > 0.9 && young.alpha() <= 1.0);

        // A stationary flow still ages and fades the fleck.
        let mut guard = 0;
        while guard < 8 {
            pool.update(0.1, 0.0, 0.0);
            guard += 1;
        }
        assert_eq!(pool.len(), 1);
        let old = pool.iter()[0];
        assert!((old.pos[0] - 1.05).abs() < 1.0e-5, "stationary flow moved x");
        assert!(old.alpha() > 0.0 && old.alpha() < young.alpha());

        guard = 0;
        while !pool.is_empty() && guard < 32 {
            pool.update(0.1, 0.0, 0.0);
            guard += 1;
        }
        assert!(pool.is_empty(), "expired fleck should be gone");
    }

    #[test]
    fn capacity_is_capped_and_slots_reopen_after_expiry() {
        let mut pool = FoamPool::new();
        assert_eq!(pool.capacity(), CAPACITY);

        for i in 0..CAPACITY {
            let life = if i == 0 { 0.05 } else { 5.0 };
            assert!(pool.spawn(i as f32, 1.0, 0.0, life), "spawn {} should fit", i);
        }
        assert_eq!(pool.len(), CAPACITY);
        assert!(!pool.spawn(999.0, 1.0, 0.0, 1.0));
        assert_eq!(pool.len(), CAPACITY, "full pool must reject new spawns");

        pool.update(0.1, 0.0, 0.0);
        assert_eq!(pool.len(), CAPACITY - 1);
        assert!(pool.spawn(999.0, 1.0, 0.0, 1.0));
        assert_eq!(pool.len(), CAPACITY);
        assert_eq!(pool.iter().len(), CAPACITY);

        pool.clear();
        assert!(pool.is_empty());
    }

    #[test]
    fn fleck_alpha_fades_slowly_and_handles_invalid_inputs() {
        assert_eq!(fleck_alpha(0.0, 1.0), 0.0);
        assert_eq!(fleck_alpha(1.0, 1.0), 0.0);
        assert_eq!(fleck_alpha(2.0, 1.0), 0.0);
        assert_eq!(fleck_alpha(0.5, 0.0), 0.0);
        assert_eq!(fleck_alpha(f32::NAN, 1.0), 0.0);
        assert_eq!(fleck_alpha(0.5, f32::INFINITY), 0.0);

        let early = fleck_alpha(0.01, 1.0);
        let mid = fleck_alpha(0.5, 1.0);
        let late = fleck_alpha(0.9, 1.0);
        assert!(mid > 0.4 && mid < 0.8, "mid alpha {}", mid);
        assert!(early < mid && late < mid && late > 0.0);

        for i in 0..=20 {
            let a = fleck_alpha(i as f32 / 20.0, 1.0);
            assert!(a >= 0.0 && a <= 1.0, "alpha out of range at {}", i);
        }
    }

    #[test]
    fn invalid_spawn_and_flow_inputs_are_ignored_safely() {
        let mut pool = FoamPool::new();
        assert!(!pool.spawn(f32::NAN, 0.0, 0.0, 1.0));
        assert!(!pool.spawn(0.0, f32::INFINITY, 0.0, 1.0));
        assert!(!pool.spawn(0.0, 0.0, 0.0, 0.0));
        assert!(!pool.spawn(0.0, 0.0, 0.0, f32::NAN));
        assert!(pool.is_empty());

        assert!(pool.spawn(0.0, 1.0, 0.0, 1.0));
        pool.update(f32::NAN, 1.0, 1.0);
        assert!((pool.iter()[0].age - 0.0).abs() < 1.0e-6);
        pool.update(0.1, f32::INFINITY, f32::NAN);
        let f = pool.iter()[0];
        assert!(f.pos.iter().all(|v| (*v).is_finite()));
        assert!((f.pos[0] - 0.0).abs() < 1.0e-6, "x = {}", f.pos[0]);
        assert!((f.pos[2] - 0.0).abs() < 1.0e-6, "z = {}", f.pos[2]);
    }
}

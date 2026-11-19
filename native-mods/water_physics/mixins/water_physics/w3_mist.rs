// Rising waterfall mist particle model for Water Physics v4.
//
// Fixed-capacity, allocation-free pool of buoyant mist puffs. Each puff drifts
// through a deterministic curl-like field derived from its position, age, and
// seed, then expands and fades as it rises from a waterfall plunge pool.

pub const CAPACITY: usize = 96;
pub const MAX_FRAME_DT: f32 = 0.1;
pub const MIN_INTENSITY: f32 = 0.0;
pub const MAX_INTENSITY: f32 = 8.0;

const CURL_SEED_A: u32 = 0xA341_316C;
const CURL_SEED_B: u32 = 0x8E7C_93F1;
const CURL_SEED_C: u32 = 0xC2B2_AE35;

#[inline]
fn hash_unit(v: u32) -> f32 {
    let mut h = v.wrapping_add(0x9E37_79B9);
    h = (h ^ (h >> 16)).wrapping_mul(0x85EB_CA6B);
    h = (h ^ (h >> 13)).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    ((h >> 8) as f32) * (1.0 / 16_777_216.0)
}

#[inline]
fn signed_unit(v: u32) -> f32 {
    hash_unit(v) * 2.0 - 1.0
}

/// Smooth, continuous scalar noise without tables or allocations.
#[inline]
fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor();
    let iy = y.floor();
    let iz = z.floor();
    let fx = x - ix;
    let fy = y - iy;
    let fz = z - iz;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let w = fz * fz * (3.0 - 2.0 * fz);

    let bx = ix as i32 as u32;
    let by = iy as i32 as u32;
    let bz = iz as i32 as u32;
    let h = |dx: u32, dy: u32, dz: u32| {
        signed_unit(bx.wrapping_add(dx.wrapping_mul(0x9E37_79B9))
            ^ by.wrapping_add(dy.wrapping_mul(0x85EB_CA6B)).rotate_left(9)
            ^ bz.wrapping_add(dz.wrapping_mul(0xC2B2_AE35)).rotate_left(17)
            ^ seed)
    };

    let x00 = h(0, 0, 0) + (h(1, 0, 0) - h(0, 0, 0)) * u;
    let x10 = h(0, 1, 0) + (h(1, 1, 0) - h(0, 1, 0)) * u;
    let x01 = h(0, 0, 1) + (h(1, 0, 1) - h(0, 0, 1)) * u;
    let x11 = h(0, 1, 1) + (h(1, 1, 1) - h(0, 1, 1)) * u;
    let y0 = x00 + (x10 - x00) * v;
    let y1 = x01 + (x11 - x01) * v;
    y0 + (y1 - y0) * w
}

/// Divergence-free curl of a smooth vector field. Used for mist drift so puffs
/// swirl around the waterfall base instead of moving in straight lines.
#[inline]
fn curl(x: f32, y: f32, z: f32, seed: u32) -> [f32; 3] {
    let e = 0.18;
    let inv = 1.0 / (2.0 * e);

    let dp_dy = (noise3(x, y + e, z, CURL_SEED_A) - noise3(x, y - e, z, CURL_SEED_A)) * inv;
    let dp_dz = (noise3(x, y, z + e, CURL_SEED_A) - noise3(x, y, z - e, CURL_SEED_A)) * inv;
    let dq_dx = (noise3(x + e, y, z, CURL_SEED_B) - noise3(x - e, y, z, CURL_SEED_B)) * inv;
    let dq_dz = (noise3(x, y, z + e, CURL_SEED_B) - noise3(x, y, z - e, CURL_SEED_B)) * inv;
    let dr_dx = (noise3(x + e, y, z, CURL_SEED_C) - noise3(x - e, y, z, CURL_SEED_C)) * inv;
    let dr_dy = (noise3(x, y + e, z, CURL_SEED_C) - noise3(x, y - e, z, CURL_SEED_C)) * inv;

    [dr_dy - dq_dz, dp_dz - dr_dx, dq_dx - dp_dy]
}

#[derive(Clone, Copy, Debug)]
pub struct MistPuff {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub age: f32,
    pub life: f32,
    pub size: f32,
    pub seed: u64,
}

impl MistPuff {
    #[inline]
    pub fn alive(&self) -> bool {
        self.age < self.life
    }

    #[inline]
    pub fn alpha(&self) -> f32 {
        mist_alpha(self.age, self.life)
    }
}

#[derive(Clone, Debug)]
pub struct MistPool {
    puffs: [MistPuff; CAPACITY],
    len: usize,
    next_seed: u64,
}

impl Default for MistPool {
    fn default() -> Self {
        Self::new()
    }
}

impl MistPool {
    pub fn new() -> Self {
        let blank = MistPuff {
            pos: [0.0; 3],
            vel: [0.0; 3],
            age: 0.0,
            life: 1.0,
            size: 0.0,
            seed: 0,
        };
        Self {
            puffs: [blank; CAPACITY],
            len: 0,
            next_seed: 0x9E37_79B9_7F4A_7C15,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn iter(&self) -> &[MistPuff] {
        &self.puffs[..self.len]
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    fn next_rand(&mut self) -> u64 {
        let mut x = self.next_seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.next_seed = x;
        x
    }

    /// Spawn one puff at the waterfall plunge pool. Returns false if full.
    pub fn spawn(&mut self, x: f32, y: f32, z: f32, intensity: f32) -> bool {
        if self.len >= CAPACITY {
            return false;
        }
        let i = self.next_rand();
        let unit = |shift: u32| ((i >> shift) & 0xFFFF) as f32 * (1.0 / 65_535.0);
        let strength = intensity.clamp(MIN_INTENSITY, MAX_INTENSITY) / MAX_INTENSITY;
        let angle = unit(0) * std::f32::consts::TAU;
        let radius = 0.12 + unit(16) * (0.22 + strength * 0.28);
        let puff = MistPuff {
            pos: [x + angle.cos() * radius, y + 0.05, z + angle.sin() * radius],
            vel: [angle.cos() * 0.12, 0.35 + strength * 0.35, angle.sin() * 0.12],
            age: 0.0,
            life: mist_lifetime(intensity),
            size: 0.10 + strength * 0.25,
            seed: i,
        };
        self.puffs[self.len] = puff;
        self.len += 1;
        true
    }

    /// Advance all live puffs with curl drift, buoyant rise, drag, and expansion.
    pub fn update(&mut self, dt: f32) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, MAX_FRAME_DT)
        } else {
            0.0
        };
        let mut i = 0;
        while i < self.len {
            let puff = &mut self.puffs[i];
            puff.age += dt;
            if !puff.alive() {
                self.puffs.swap(i, self.len - 1);
                self.len -= 1;
                continue;
            }

            let seed = puff.seed as u32;
            let t = puff.age;
            let field = curl(
                puff.pos[0] * 0.55 + t * 0.07,
                puff.pos[1] * 0.70 - t * 0.12,
                puff.pos[2] * 0.55,
                seed,
            );
            let swirl = 0.55 + 0.20 * noise3(t * 0.9, 0.0, 0.0, seed ^ 0x5BD1_E995);

            puff.vel[0] += field[0] * swirl * dt;
            puff.vel[1] += 0.10 * dt;
            puff.vel[2] += field[2] * swirl * dt;

            let drag = 1.0 - (1.2 * dt).min(0.9);
            puff.vel[0] *= drag;
            puff.vel[1] *= drag;
            puff.vel[2] *= drag;

            puff.pos[0] += puff.vel[0] * dt;
            puff.pos[1] += puff.vel[1] * dt;
            puff.pos[2] += puff.vel[2] * dt;
            puff.size += 0.12 * dt;
            i += 1;
        }
    }
}

/// Life in seconds. Stronger waterfalls produce longer-lived mist.
#[inline]
pub fn mist_lifetime(intensity: f32) -> f32 {
    if !intensity.is_finite() {
        return 1.0;
    }
    (1.2 + intensity.clamp(MIN_INTENSITY, MAX_INTENSITY) * 0.35).clamp(1.0, 4.0)
}

/// Smooth attack/decay envelope in `[0, 1]`.
#[inline]
pub fn mist_alpha(age: f32, life: f32) -> f32 {
    if !age.is_finite() || !life.is_finite() || life <= 0.0 || age <= 0.0 || age >= life {
        return 0.0;
    }
    let t = age / life;
    let attack = (t / 0.15).min(1.0);
    let decay = 1.0 - t;
    (attack * decay * decay).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetime_and_alpha_envelope() {
        assert_eq!(mist_lifetime(f32::NAN), 1.0);
        assert!(mist_lifetime(8.0) > mist_lifetime(0.0));
        assert_eq!(mist_alpha(0.0, 2.0), 0.0);
        assert_eq!(mist_alpha(2.0, 2.0), 0.0);
        assert_eq!(mist_alpha(1.0, f32::NAN), 0.0);
        let early = mist_alpha(0.1, 2.0);
        let mid = mist_alpha(1.0, 2.0);
        assert!(mid > early);
        assert!(mid > 0.0 && mid <= 1.0);
    }

    #[test]
    fn pool_spawn_rise_expand_and_expire() {
        let mut pool = MistPool::new();
        assert!(pool.is_empty());
        assert!(pool.spawn(1.0, 5.0, 2.0, 2.0));
        assert_eq!(pool.len(), 1);
        let y0 = pool.iter()[0].pos[1];
        let size0 = pool.iter()[0].size;
        pool.update(0.1);
        let puff = pool.iter()[0];
        assert!(puff.pos[1] > y0);
        assert!(puff.size > size0);
        for _ in 0..60 {
            pool.update(0.1);
        }
        assert!(pool.is_empty());
    }

    #[test]
    fn fixed_capacity_returns_false_when_full() {
        let mut pool = MistPool::new();
        for _ in 0..CAPACITY {
            assert!(pool.spawn(0.0, 0.0, 0.0, 1.0));
        }
        assert!(!pool.spawn(0.0, 0.0, 0.0, 1.0));
        assert_eq!(pool.len(), CAPACITY);
        pool.clear();
        assert!(pool.is_empty());
    }

    #[test]
    fn curl_field_is_bounded_and_divergence_free() {
        let c = curl(0.3, 1.2, -0.8, 123);
        assert!(c.iter().all(|v| v.is_finite() && v.abs() < 8.0));
        // Same step as the curl stencil: mixed finite differences commute,
        // so the discrete divergence cancels to floating-point noise.
        let e = 0.18;
        let divergence = (curl(0.3 + e, 1.2, -0.8, 123)[0] - curl(0.3 - e, 1.2, -0.8, 123)[0])
            / (2.0 * e)
            + (curl(0.3, 1.2 + e, -0.8, 123)[1] - curl(0.3, 1.2 - e, -0.8, 123)[1])
                / (2.0 * e)
            + (curl(0.3, 1.2, -0.8 + e, 123)[2] - curl(0.3, 1.2, -0.8 - e, 123)[2])
                / (2.0 * e);
        assert!(divergence.abs() < 0.01);
    }
}

//! Deterministic, dependency-free noise primitives for water v4.
//!
//! All functions are pure: the same `(x, z, seed)` always produces the same
//! value on every platform. No crates, no globals, no heap allocation.
//! Coordinates are world-space X/Z (Y up); `seed` decorrelates independent
//! fields (waves, foam, caustics, particles, ...).
//!
//! Public API:
//! * [`hash2`] / [`hash3`] - seeded lattice hashes in `[0, 1)`.
//! * [`smoothstep`] - Hermite interpolation factor, clamped to `[0, 1]`.
//! * [`value2`] / [`value3`] - seeded value noise in `[0, 1]`.
//! * [`fbm2`] / [`fbm3`] - normalized fractal noise in `[0, 1]`.
//! * [`fbm2_4`] / [`fbm3_4`] - common four-octave convenience wrappers.
//! * [`cellular2`] / [`cellular3`] - worley-ish nearest-feature distance in `[0, 1]`.

/// Mix a 32-bit integer deterministically (splitmix-style finalizer).
#[inline]
fn mix32(mut h: u32) -> u32 {
    h = h.wrapping_add(0x9E37_79B9);
    h = (h ^ (h >> 16)).wrapping_mul(0x85EB_CA6B);
    h = (h ^ (h >> 13)).wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

/// Convert a mixed integer to a deterministic f32 in `[0, 1)`.
/// The 24-bit shift keeps the result exactly representable in f32.
#[inline]
fn unit(h: u32) -> f32 {
    ((h >> 8) as f32) * (1.0 / 16777216.0)
}

/// Seeded hash of a 2D integer lattice point, in `[0, 1)`.
#[inline]
pub fn hash2(ix: i32, iz: i32, seed: u32) -> f32 {
    let h = (ix as u32) ^ (iz as u32).rotate_left(16) ^ seed.rotate_left(8);
    unit(mix32(h))
}

/// Seeded hash of a 3D integer lattice point, in `[0, 1)`.
#[inline]
pub fn hash3(ix: i32, iy: i32, iz: i32, seed: u32) -> f32 {
    let h = (ix as u32)
        ^ (iy as u32).rotate_left(8)
        ^ (iz as u32).rotate_left(16)
        ^ seed.rotate_left(24);
    unit(mix32(h))
}

/// Clamped Hermite smoothstep. `smoothstep(0)=0`, `smoothstep(1)=1`.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Seeded 2D value noise in `[0, 1]`, C1-continuous at cell boundaries.
#[inline]
pub fn value2(x: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor();
    let iz = z.floor();
    let x0 = ix as i32;
    let z0 = iz as i32;
    let fx = x - ix;
    let fz = z - iz;

    let u = smoothstep(fx);
    let v = smoothstep(fz);

    let a = hash2(x0, z0, seed);
    let b = hash2(x0 + 1, z0, seed);
    let c = hash2(x0, z0 + 1, seed);
    let d = hash2(x0 + 1, z0 + 1, seed);

    let lo = a + (b - a) * u;
    let hi = c + (d - c) * u;
    lo + (hi - lo) * v
}

/// Seeded 3D value noise in `[0, 1]`, C1-continuous at cell boundaries.
#[inline]
pub fn value3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor();
    let iy = y.floor();
    let iz = z.floor();
    let x0 = ix as i32;
    let y0 = iy as i32;
    let z0 = iz as i32;
    let fx = x - ix;
    let fy = y - iy;
    let fz = z - iz;

    let u = smoothstep(fx);
    let v = smoothstep(fy);
    let w = smoothstep(fz);

    let c000 = hash3(x0, y0, z0, seed);
    let c100 = hash3(x0 + 1, y0, z0, seed);
    let c010 = hash3(x0, y0 + 1, z0, seed);
    let c110 = hash3(x0 + 1, y0 + 1, z0, seed);
    let c001 = hash3(x0, y0, z0 + 1, seed);
    let c101 = hash3(x0 + 1, y0, z0 + 1, seed);
    let c011 = hash3(x0, y0 + 1, z0 + 1, seed);
    let c111 = hash3(x0 + 1, y0 + 1, z0 + 1, seed);

    let x00 = c000 + (c100 - c000) * u;
    let x10 = c010 + (c110 - c010) * u;
    let x01 = c001 + (c101 - c001) * u;
    let x11 = c011 + (c111 - c011) * u;

    let y0v = x00 + (x10 - x00) * v;
    let y1v = x01 + (x11 - x01) * v;
    y0v + (y1v - y0v) * w
}

/// Octave seed offset. Keeps FBM octaves decorrelated when frequencies align.
#[inline]
fn octave_seed(seed: u32, octave: u32) -> u32 {
    seed.wrapping_add(octave.wrapping_mul(0x9E37_79B9))
}

/// Fractal 2D value noise in `[0, 1]` (lacunarity 2, gain 0.5).
/// `octaves == 0` returns the neutral value `0.5`; values above 12 are capped.
#[inline]
pub fn fbm2(x: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    if octaves == 0 {
        return 0.5;
    }
    let octaves = octaves.min(12);
    let mut freq = 1.0f32;
    let mut amp = 1.0f32;
    let mut sum = 0.0f32;
    let mut norm = 0.0f32;
    for o in 0..octaves {
        sum += value2(x * freq, z * freq, octave_seed(seed, o)) * amp;
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Four-octave [`fbm2`] convenience wrapper.
#[inline]
pub fn fbm2_4(x: f32, z: f32, seed: u32) -> f32 {
    fbm2(x, z, seed, 4)
}

/// Fractal 3D value noise in `[0, 1]` (lacunarity 2, gain 0.5).
/// `octaves == 0` returns the neutral value `0.5`; values above 12 are capped.
#[inline]
pub fn fbm3(x: f32, y: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    if octaves == 0 {
        return 0.5;
    }
    let octaves = octaves.min(12);
    let mut freq = 1.0f32;
    let mut amp = 1.0f32;
    let mut sum = 0.0f32;
    let mut norm = 0.0f32;
    for o in 0..octaves {
        sum += value3(x * freq, y * freq, z * freq, octave_seed(seed, o)) * amp;
        norm += amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Four-octave [`fbm3`] convenience wrapper.
#[inline]
pub fn fbm3_4(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    fbm3(x, y, z, seed, 4)
}

/// Worley-ish 2D cellular noise: nearest jittered feature-point distance,
/// normalized into `[0, 1]` (small near a feature point). Searches a 3x3
/// neighbourhood, so it is O(1) per sample and allocation-free.
#[inline]
pub fn cellular2(x: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fz = z - iz as f32;

    let mut best2 = f32::INFINITY;
    for dz in -1..=1 {
        for dx in -1..=1 {
            let cx = ix + dx;
            let cz = iz + dz;
            let px = dx as f32 + 0.15 + 0.70 * hash2(cx, cz, seed);
            let pz = dz as f32 + 0.15 + 0.70 * hash2(cx, cz, seed ^ 0xA5A5_A5A5);
            let ex = px - fx;
            let ez = pz - fz;
            let d2 = ex * ex + ez * ez;
            if d2 < best2 {
                best2 = d2;
            }
        }
    }
    best2.sqrt().clamp(0.0, 1.0)
}

/// Worley-ish 3D cellular noise: nearest jittered feature-point distance in
/// `[0, 1]`. Searches a 3x3x3 neighbourhood; prefer [`cellular2`] for water
/// surfaces where the extra dimension is unnecessary.
#[inline]
pub fn cellular3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let fz = z - iz as f32;

    let mut best2 = f32::INFINITY;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cx = ix + dx;
                let cy = iy + dy;
                let cz = iz + dz;
                let px = dx as f32 + 0.15 + 0.70 * hash3(cx, cy, cz, seed);
                let py = dy as f32 + 0.15 + 0.70 * hash3(cx, cy, cz, seed ^ 0xA5A5_A5A5);
                let pz = dz as f32 + 0.15 + 0.70 * hash3(cx, cy, cz, seed ^ 0x5A5A_5A5A);
                let ex = px - fx;
                let ey = py - fy;
                let ez = pz - fz;
                let d2 = ex * ex + ey * ey + ez * ez;
                if d2 < best2 {
                    best2 = d2;
                }
            }
        }
    }
    best2.sqrt().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_seed_sensitive() {
        let x = 12.375f32;
        let z = -7.25f32;
        let seed = 0x1234_5678u32;

        assert_eq!(hash2(3, -4, seed), hash2(3, -4, seed));
        assert_eq!(value2(x, z, seed), value2(x, z, seed));
        assert_eq!(fbm2_4(x, z, seed), fbm2_4(x, z, seed));
        assert_eq!(value3(x, 2.5, z, seed), value3(x, 2.5, z, seed));
        assert_eq!(fbm3_4(x, 2.5, z, seed), fbm3_4(x, 2.5, z, seed));
        assert_eq!(cellular2(x, z, seed), cellular2(x, z, seed));
        assert_eq!(cellular3(x, 2.5, z, seed), cellular3(x, 2.5, z, seed));

        assert_ne!(value2(x, z, seed), value2(x, z, seed.wrapping_add(1)));
        assert_ne!(hash2(3, -4, seed), hash2(3, -4, seed.wrapping_add(1)));
        assert_ne!(fbm2_4(x, z, seed), fbm2_4(x, z, seed.wrapping_add(1)));
    }

    #[test]
    fn bounded_range_and_lattice_exactness() {
        let seed = 99u32;
        for i in -8..=8 {
            for j in -8..=8 {
                let x = i as f32 * 0.375;
                let z = j as f32 * 0.625;
                for v in [
                    hash2(i, j, seed),
                    hash3(i, j, -j, seed),
                    smoothstep(i as f32 * 0.1),
                    value2(x, z, seed),
                    value3(x, j as f32 * 0.25, z, seed),
                    fbm2(x, z, seed, 4),
                    fbm3(x, j as f32 * 0.25, z, seed, 4),
                    cellular2(x, z, seed),
                    cellular3(x, j as f32 * 0.25, z, seed),
                ] {
                    assert!(v.is_finite(), "non-finite noise value");
                    assert!((0.0..=1.0).contains(&v), "out of range: {v}");
                }
                // Integer lattice points reproduce the seeded hash exactly.
                assert_eq!(value2(i as f32, j as f32, seed), hash2(i, j, seed));
                assert_eq!(
                    value3(i as f32, j as f32, -j as f32, seed),
                    hash3(i, j, -j, seed)
                );
            }
        }
        assert_eq!(fbm2(1.0, 2.0, seed, 0), 0.5);
        assert_eq!(fbm3(1.0, 2.0, 3.0, seed, 0), 0.5);
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(2.0), 1.0);
    }

    #[test]
    fn continuous_across_cell_boundaries() {
        let seed = 0xDEAD_BEEFu32;
        let eps = 1.0e-3f32;
        let mut max_jump2 = 0.0f32;
        let mut max_jump3 = 0.0f32;
        let mut max_jump_fbm = 0.0f32;
        let mut max_jump_cell = 0.0f32;

        for i in -4..=4 {
            for j in -4..=4 {
                let x = i as f32;
                let z = j as f32 * 0.5;
                let y = j as f32 * 0.25;

                // Cross an integer boundary in X from both sides.
                let a2 = value2(x - eps, z, seed);
                let b2 = value2(x + eps, z, seed);
                max_jump2 = max_jump2.max((a2 - b2).abs());

                let a3 = value3(x - eps, y, z, seed);
                let b3 = value3(x + eps, y, z, seed);
                max_jump3 = max_jump3.max((a3 - b3).abs());

                let af = fbm2_4(x - eps, z, seed);
                let bf = fbm2_4(x + eps, z, seed);
                max_jump_fbm = max_jump_fbm.max((af - bf).abs());

                let ac = cellular2(x - eps, z, seed);
                let bc = cellular2(x + eps, z, seed);
                max_jump_cell = max_jump_cell.max((ac - bc).abs());
            }
        }

        assert!(max_jump2 < 0.01, "2D value noise discontinuity: {max_jump2}");
        assert!(max_jump3 < 0.01, "3D value noise discontinuity: {max_jump3}");
        assert!(max_jump_fbm < 0.02, "FBM discontinuity: {max_jump_fbm}");
        assert!(max_jump_cell < 0.02, "cellular discontinuity: {max_jump_cell}");
    }
}

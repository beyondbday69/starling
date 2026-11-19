// w3_caustics.rs - Procedural animated caustic light intensity (water v4).
//
// Caustics are modeled as a bright web of light created where light rays focus.
// In this approximation each layer is a cellular (Voronoi-like) field whose
// feature points drift with time. The brightest lines occur on cell borders,
// where the distances to the two nearest feature points converge (F2 - F1 -> 0).
// Three drifting octaves are layered to avoid a regular, repeating grid look.
//
// Public API:
//   caustic(x, z, t, seed) -> f32   intensity in 0..2 (deterministic, allocation-free)
//   caustic_rgb(intensity) -> [f32; 3]  linear RGB in 0..1 for that intensity
//
// Coordinates follow the project contract: X/Z horizontal, Y up, world units
// are Minecraft blocks. `t` is time in seconds. No globals, no unsafe, no deps.

/// Upper bound of `caustic()`. Useful when scaling the value into a shader.
pub const CAUSTIC_MAX: f32 = 2.0;

const INV_65536: f32 = 1.0 / 65536.0;
const INV_256: f32 = 1.0 / 256.0;
const TAU: f32 = 6.28318530717958647692_f32;

/// SplitMix64-style integer mixer. Deterministic on every platform.
#[inline]
fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    x
}

/// Stable hash for one integer lattice cell of one octave.
#[inline]
fn cell_hash(ix: i32, iz: i32, layer: u32, seed: u64) -> u64 {
    let h = (ix as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (iz as u64).wrapping_mul(0xbf58476d1ce4e5b9)
        ^ (layer as u64).wrapping_mul(0x94d049bb133111eb)
        ^ seed;
    mix64(h)
}

/// Compact rational ridge: 1.0 at zero separation, falls off smoothly.
#[inline]
fn ridge(separation: f32, sharpness: f32) -> f32 {
    let q = separation * sharpness;
    1.0 / (1.0 + q * q)
}

/// Distance gap (F2 - F1) to the two closest drifting feature points.
///
/// `x`/`z` are already scaled/drifting layer coordinates. Only a 3x3 lattice
/// neighborhood is examined because jitter + motion keep points inside a cell.
#[inline]
fn voronoi_gap(x: f32, z: f32, t: f32, layer: u32, seed: u64) -> f32 {
    let base_x = x.floor();
    let base_z = z.floor();
    let ix = base_x as i32;
    let iz = base_z as i32;
    let fx = x - base_x;
    let fz = z - base_z;

    let mut nearest = f32::INFINITY;
    let mut second = f32::INFINITY;

    let mut dz = -1i32;
    while dz <= 1 {
        let mut dx = -1i32;
        while dx <= 1 {
            let cx = ix + dx;
            let cz = iz + dz;
            let h = cell_hash(cx, cz, layer, seed);

            // Jitter keeps the point inside a safe band of its cell; the low
            // bits drive animated wobble, speed and phase.
            let jx = 0.18 + 0.64 * ((h & 0xffff) as f32 * INV_65536);
            let jz = 0.18 + 0.64 * (((h >> 16) & 0xffff) as f32 * INV_65536);
            let phase = ((h >> 32) & 0xffff) as f32 * INV_65536 * TAU;
            let amp = 0.08 + 0.16 * (((h >> 48) & 0xff) as f32 * INV_256);
            let speed = 0.6 + 0.5 * (((h >> 56) & 0xff) as f32 * INV_256);

            let ox = (phase + t * speed).sin() * amp;
            let oz = (phase * 1.37 + t * speed * 0.83).cos() * amp;

            let px = dx as f32 + jx + ox - fx;
            let pz = dz as f32 + jz + oz - fz;
            let dist_sq = px * px + pz * pz;

            if dist_sq < nearest {
                second = nearest;
                nearest = dist_sq;
            } else if dist_sq < second {
                second = dist_sq;
            }

            dx += 1;
        }
        dz += 1;
    }

    if second.is_finite() {
        second.sqrt() - nearest.sqrt()
    } else {
        1.0
    }
}

/// Procedural caustic intensity in `0..2` at horizontal position `(x, z)` and
/// time `t` (seconds). `seed` selects a stable, repeatable pattern.
#[inline]
pub fn caustic(x: f32, z: f32, t: f32, seed: u64) -> f32 {
    // Slow domain drift prevents the bright web from feeling locked in place.
    let drift_x = t * 0.07;
    let drift_z = t * -0.05;

    let gap0 = voronoi_gap(x * 0.55 + drift_x, z * 0.55 + drift_z, t * 0.90, 0, seed);
    let gap1 = voronoi_gap(x * 1.10 - drift_x * 1.7, z * 1.10 + drift_z * 0.8, t * 1.25, 1, seed);
    let gap2 = voronoi_gap(x * 2.20 + drift_x * 0.5, z * 2.20 - drift_z * 1.4, t * 1.70, 2, seed);

    let r0 = ridge(gap0, 8.0);
    let r1 = ridge(gap1, 10.0);
    let r2 = ridge(gap2, 12.0);

    // Squaring makes bright lines thin, like real caustics, while the base
    // keeps unlit areas a dim blue-green rather than pure black.
    let intensity = 0.18 + 1.25 * r0 * r0 + 0.55 * r1 * r1 + 0.22 * r2 * r2;
    intensity.clamp(0.0, CAUSTIC_MAX)
}

/// Map a caustic intensity (`0..2`) to linear RGB in `0..1`.
///
/// Dim values are deep teal, mid values cyan, peaks wash out toward white.
#[inline]
pub fn caustic_rgb(intensity: f32) -> [f32; 3] {
    let v = if intensity.is_finite() {
        intensity.clamp(0.0, CAUSTIC_MAX)
    } else {
        0.0
    };
    let u = v * 0.5;
    let s = u * u * (3.0 - 2.0 * u);
    // Extra whitening only in the top half of the range.
    let w = ((v - 1.0).max(0.0)).clamp(0.0, 1.0);
    let w = w * w * (3.0 - 2.0 * w);

    let r = 0.05 + 0.80 * s + 0.15 * w;
    let g = 0.26 + 0.72 * s;
    let b = 0.38 + 0.62 * s;
    [r.min(1.0), g.min(1.0), b.min(1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_inputs() {
        let a = caustic(3.75, -8.125, 2.4, 0x1234_5678_9abc_def0);
        let b = caustic(3.75, -8.125, 2.4, 0x1234_5678_9abc_def0);
        assert_eq!(a.to_bits(), b.to_bits());

        // Seed, position and time each must change the pattern.
        assert_ne!(a.to_bits(), caustic(3.75, -8.125, 2.4, 0x1234_5678_9abc_def1).to_bits());
        assert_ne!(a.to_bits(), caustic(3.80, -8.125, 2.4, 0x1234_5678_9abc_def0).to_bits());
        assert_ne!(a.to_bits(), caustic(3.75, -8.125, 3.1, 0x1234_5678_9abc_def0).to_bits());
    }

    #[test]
    fn intensity_stays_in_range_and_finite() {
        let seeds = [0u64, 1, 42, u64::MAX];
        for seed in seeds {
            for zi in -8..=8 {
                for xi in -8..=8 {
                    let x = xi as f32 * 0.37;
                    let z = zi as f32 * 0.41;
                    for ti in 0..=8 {
                        let t = ti as f32 * 0.75;
                        let v = caustic(x, z, t, seed);
                        assert!(v.is_finite(), "non-finite caustic at {x}, {z}, {t}");
                        assert!((0.0..=CAUSTIC_MAX).contains(&v), "out of range: {v}");
                    }
                }
            }
        }
    }

    #[test]
    fn animation_changes_and_is_continuous() {
        let seed = 99u64;
        let a = caustic(1.25, 2.5, 0.0, seed);
        let b = caustic(1.25, 2.5, 1.0, seed);
        assert!((a - b).abs() > 1e-4, "pattern should move over one second");

        let c = caustic(1.25, 2.5, 0.001, seed);
        assert!((a - c).abs() < 0.05, "pattern should not jump between adjacent frames");
    }

    #[test]
    fn rgb_mapping_is_bounded_and_ramped() {
        let dark = caustic_rgb(0.0);
        let bright = caustic_rgb(2.0);
        for i in 0..3 {
            assert!((0.0..=1.0).contains(&dark[i]));
            assert!((0.0..=1.0).contains(&bright[i]));
            assert!(bright[i] > dark[i]);
        }
        assert!(bright[1] > bright[0]);
        assert!(bright[2] > bright[0]);

        // Out-of-range and non-finite inputs are clamped safely.
        assert_eq!(caustic_rgb(-5.0), dark);
        assert_eq!(caustic_rgb(99.0), bright);
        assert_eq!(caustic_rgb(f32::NAN), dark);
    }
}

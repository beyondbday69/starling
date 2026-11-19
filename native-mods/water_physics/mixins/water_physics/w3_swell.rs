//! Large-scale, slow swell modulation for water v4.
//!
//! This module supplies a pair of pure, allocation-free helpers that add a
//! gentle, long-wavelength rise and fall on top of the finer ripples generated
//! elsewhere. It is render-agnostic: callers add [`swell_height`] to the local
//! water level (which lives at `water_y + 8.0/9.0`) before shading or displacement.
//!
//! Design:
//! * Two directional plane waves with wavelengths of roughly 30-72 blocks,
//!   i.e. much longer than normal ripples, so the result reads as swell.
//! * Directions, wavelengths, speeds and phases are derived deterministically
//!   from a `u64` seed via a splitmix64-style finalizer. No globals, no RNG
//!   state to pass around.
//! * [`swell_amplitude`] is a slow "breathing" envelope (two incommensurate
//!   periods) that stays positive and bounded, periodically making the swell
//!   more or less pronounced.
//!
//! Public API:
//! * [`swell_height`] - signed Y offset in blocks for a world X/Z position at time `t`.
//! * [`swell_amplitude`] - positive breathing multiplier, approximately `[0.77, 1.23]`.

/// 2*PI as f32.
const TAU: f32 = std::f32::consts::TAU;

/// Splitmix64 finalizer. Maps a 64-bit seed to a well-distributed 64-bit value.
#[inline]
fn mix64(mut h: u64) -> u64 {
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h
}

/// Deterministic scalar parameter in `[0, 1)` derived from `seed` and a salt.
/// The 24-bit shift maps exactly onto f32 precision.
#[inline]
fn param(seed: u64, salt: u64) -> f32 {
    let h = mix64(seed.wrapping_add(salt));
    ((h >> 40) as f32) * (1.0 / 16777216.0)
}

/// Slow breathing envelope applied to the swell waves.
///
/// Two incommensurate sine periods (about 84 s and 273 s) avoid an obvious
/// repetition while keeping the value positive and bounded in `[0.77, 1.23]`.
/// Non-finite `t` yields the neutral value `1.0`.
#[inline]
pub fn swell_amplitude(t: f32) -> f32 {
    if !t.is_finite() {
        return 1.0;
    }
    let tide = (t * 0.075).sin();
    let beat = (t * 0.023 + 0.6).sin();
    1.0 + 0.16 * tide + 0.07 * beat
}

/// Signed vertical swell offset (blocks) at world position `(x, z)` and time `t`.
///
/// The field is the sum of two low-frequency directional waves, scaled by
/// [`swell_amplitude`]. It is deterministic for a given `(x, z, t, seed)` and
/// magnitude is bounded by roughly `0.4` blocks. Non-finite inputs return `0.0`.
#[inline]
pub fn swell_height(x: f32, z: f32, t: f32, seed: u64) -> f32 {
    if !(x.is_finite() && z.is_finite() && t.is_finite()) {
        return 0.0;
    }

    // Wave 0: long, fairly fast swell.
    let angle0 = param(seed, 0x9E37_79B9_7F4A_7C15) * TAU;
    let wavelength0 = 48.0 + param(seed, 0xD1B5_4A32_D192_ED03) * 24.0;
    let k0 = TAU / wavelength0;
    let speed0 = 2.2 + param(seed, 0xE703_7ED1_A0B4_28DB) * 1.6;
    let phase0 = param(seed, 0x8A5C_D539_3D10_4F82) * TAU;
    let height0 = 0.15 + param(seed, 0x1B56_C4E8_853C_DF31) * 0.05;

    // Wave 1: shorter, slower swell travelling at a distinctly different angle.
    let angle1 = angle0 + 1.1 + param(seed, 0x7F4A_7C15_9E37_79B9) * 2.5;
    let wavelength1 = 30.0 + param(seed, 0xED03_D1B5_4A32_7C15) * 18.0;
    let k1 = TAU / wavelength1;
    let speed1 = 1.6 + param(seed, 0x28DB_E703_7ED1_A0B4) * 1.2;
    let phase1 = param(seed, 0x4F82_8A5C_D539_3D10) * TAU;
    let height1 = 0.08 + param(seed, 0xDF31_1B56_C4E8_853C) * 0.04;

    let proj0 = angle0.cos() * x + angle0.sin() * z;
    let proj1 = angle1.cos() * x + angle1.sin() * z;

    let wave0 = height0 * (k0 * proj0 - (k0 * speed0) * t + phase0).sin();
    let wave1 = height1 * (k1 * proj1 - (k1 * speed1) * t + phase1).sin();

    swell_amplitude(t) * (wave0 + wave1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_seed_sensitive_and_finite() {
        let (x, z, t) = (12.5f32, -7.25f32, 3.75f32);
        let seed = 0x1234_5678_9ABC_DEF0u64;

        let h = swell_height(x, z, t, seed);
        assert_eq!(h, swell_height(x, z, t, seed));
        assert!(h.is_finite());

        // A different seed must produce a visibly different field somewhere.
        let mut seed_changed = false;
        // Time must advance the field.
        let mut time_changed = false;
        for i in 0..32 {
            let xx = x + i as f32 * 3.7;
            if (swell_height(xx, z, t, seed) - swell_height(xx, z, t, seed.wrapping_add(1))).abs()
                > 1.0e-4
            {
                seed_changed = true;
            }
            if (swell_height(x, z, t + i as f32, seed) - h).abs() > 1.0e-4 {
                time_changed = true;
            }
        }
        assert!(seed_changed, "seed must decorrelate the swell field");
        assert!(time_changed, "swell must evolve over time");

        assert_eq!(swell_height(f32::NAN, z, t, seed), 0.0);
        assert_eq!(swell_height(x, f32::INFINITY, t, seed), 0.0);
        assert_eq!(swell_height(x, z, f32::NEG_INFINITY, seed), 0.0);
    }

    #[test]
    fn amplitude_is_positive_bounded_and_breathing() {
        let base = swell_amplitude(0.0);
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;

        for i in 0..2000 {
            let t = i as f32 * 0.25; // 0 .. 500 seconds
            let a = swell_amplitude(t);
            assert!(a.is_finite());
            assert!(a > 0.0, "breathing envelope must stay positive");
            min = min.min(a);
            max = max.max(a);
        }

        assert!((0.7..=1.3).contains(&min), "min = {min}");
        assert!((0.7..=1.3).contains(&max), "max = {max}");
        assert!(max - min > 0.1, "envelope must visibly breathe");
        assert!((swell_amplitude(0.0) - base).abs() < 1.0e-6);
        assert_eq!(swell_amplitude(f32::NAN), 1.0);
    }

    #[test]
    fn height_is_bounded_smooth_and_near_zero_mean() {
        let seed = 42u64;
        let t = 10.0f32;
        let mut sum = 0.0f64;
        let mut samples = 0u32;
        let mut max_abs = 0.0f32;
        let mut max_jump = 0.0f32;

        for ix in -30..=30 {
            for iz in -30..=30 {
                let x = ix as f32 * 2.0;
                let z = iz as f32 * 2.0;
                let h = swell_height(x, z, t, seed);
                assert!(h.is_finite());
                max_abs = max_abs.max(h.abs());
                sum += h as f64;
                samples += 1;

                let h_next = swell_height(x + 0.01, z, t, seed);
                max_jump = max_jump.max((h - h_next).abs());
            }
        }

        assert!(max_abs <= 0.45, "swell too large: {max_abs}");
        assert!(max_jump < 0.01, "swell not spatially smooth: {max_jump}");
        let mean = sum / samples as f64;
        assert!(mean.abs() < 0.2, "swell should be near zero-mean: {mean}");
    }
}

// Procedural waterfall curtain geometry and dissolve profiles (water v4).
//
// Local vertical convention (Y is up, in Minecraft blocks):
//   y =  0      waterfall lip / source edge
//   y < 0      falling sheet, e.g. y = -height at the plunge pool
//
// All functions are pure, allocation-free, and deterministic for a given seed.

const TAU: f32 = 6.283185307179586476925286766559_f32;

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
    let v = mix64(seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    ((v >> 40) as f32) * (TAU / 16_777_216.0)
}

#[inline]
fn smoothstep01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Turbulence ramp from the lip (0.0) to a fully developed fall (1.0).
#[inline]
fn fall_envelope(fall: f32) -> f32 {
    fall / (fall + 3.0)
}

/// Horizontal X/Z displacement for one curtain vertex.
///
/// `y` is local vertical position relative to the lip (0 at the lip, negative
/// while falling). `t` is time in seconds. `seed` decorrelates neighbouring
/// waterfalls. Returned offsets are in world blocks and stay below ~0.15.
#[inline]
pub fn curtain_offset(y: f32, t: f32, seed: u64) -> (f32, f32) {
    if !y.is_finite() || !t.is_finite() {
        return (0.0, 0.0);
    }

    let fall = (-y).max(0.0);
    let env = fall_envelope(fall);
    let gust = 1.0 + 0.18 * (t * 0.43 + phase(seed, 6)).sin();

    let a = t * 1.70 + fall * 0.65 + phase(seed, 1);
    let b = t * 2.90 - fall * 1.10 + phase(seed, 2);
    let c = t * 6.30 + fall * 2.35 + phase(seed, 3);
    let d = t * 3.70 - fall * 0.80 + phase(seed, 4);
    let e = t * 8.10 + fall * 3.10 + phase(seed, 5);

    // Small source wobble exists even at the lip; the travelling waves grow
    // with fall distance as the sheet accelerates and breaks into ribbons.
    let lip = 0.010 * (t * 0.90 + phase(seed, 0)).sin();
    let x = lip + env * gust * (0.052 * a.sin() + 0.026 * b.sin() + 0.010 * c.sin());
    let z = lip * 0.5 + env * gust * (0.044 * d.cos() + 0.021 * b.cos() + 0.009 * e.sin());

    (x, z)
}

/// Vertical opacity profile for the falling sheet.
///
/// `height` is the positive drop from lip to plunge pool. Alpha ramps in just
/// below the lip, thins with fall distance, then dissolves into mist at the
/// base. Returns 0.0 for invalid heights or positions outside the sheet.
#[inline]
pub fn curtain_alpha(y: f32, height: f32) -> f32 {
    if !y.is_finite() || !height.is_finite() || height <= 0.0 {
        return 0.0;
    }

    let p = (-y / height).clamp(0.0, 1.0);
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }

    let attack = smoothstep01(p / 0.05);
    let thinning = 1.0 - 0.30 * p;
    let dissolve = 1.0 - smoothstep01((p - 0.82) / 0.18);

    (0.94 * attack * thinning * dissolve).clamp(0.0, 1.0)
}

/// Spray/mist density near the plunge pool.
///
/// Zero above the lip and through the upper sheet, rising to a dense band at
/// the base, then decaying below the pool surface. `height` is the drop.
#[inline]
pub fn mist_density(y: f32, height: f32) -> f32 {
    if !y.is_finite() || !height.is_finite() || height <= 0.0 {
        return 0.0;
    }

    let fall = -y;
    if fall <= 0.0 {
        return 0.0;
    }

    let p = (fall / height).clamp(0.0, 1.0);
    let mut density = 0.88 * smoothstep01((p - 0.58) / 0.42);

    if fall > height {
        let below = fall - height;
        density /= 1.0 + below / (0.28 * height);
    }

    density.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curtain_offset_is_deterministic_seeded_bounded_and_animated() {
        let seed = 0x1234_5678_u64;
        let a = curtain_offset(-8.0, 1.25, seed);
        let b = curtain_offset(-8.0, 1.25, seed);
        assert_eq!(a, b);

        assert!(a.0.is_finite() && a.1.is_finite());
        assert!(a.0.abs() < 0.2 && a.1.abs() < 0.2);

        let moved = curtain_offset(-8.0, 1.55, seed);
        assert!((a.0 - moved.0).abs() > 1.0e-4 || (a.1 - moved.1).abs() > 1.0e-4);

        let other = curtain_offset(-8.0, 1.25, 0x9876_5432_u64);
        assert!((a.0 - other.0).abs() > 1.0e-4 || (a.1 - other.1).abs() > 1.0e-4);

        assert_eq!(curtain_offset(f32::NAN, 1.0, seed), (0.0, 0.0));
        assert_eq!(curtain_offset(-1.0, f32::INFINITY, seed), (0.0, 0.0));
    }

    #[test]
    fn fall_envelope_grows_then_saturates() {
        assert_eq!(fall_envelope(0.0), 0.0);
        assert!((fall_envelope(3.0) - 0.5).abs() < 1.0e-6);
        let far = fall_envelope(30.0);
        assert!(far > 0.9 && far <= 1.0);
        assert!(fall_envelope(10.0) > fall_envelope(1.0));
    }

    #[test]
    fn curtain_alpha_fades_at_lip_and_plunge() {
        let h = 12.0_f32;
        assert_eq!(curtain_alpha(0.0, h), 0.0);
        assert_eq!(curtain_alpha(1.0, h), 0.0);
        assert_eq!(curtain_alpha(-h, h), 0.0);
        assert_eq!(curtain_alpha(-h - 1.0, h), 0.0);
        assert_eq!(curtain_alpha(-1.0, 0.0), 0.0);

        let mid = curtain_alpha(-h * 0.5, h);
        assert!(mid > 0.6 && mid <= 1.0);
        assert!(curtain_alpha(-h * 0.95, h) < mid);
        assert!(curtain_alpha(-h * 0.10, h) < mid);
    }

    #[test]
    fn mist_density_peaks_at_base_and_decays_below() {
        let h = 10.0_f32;
        assert_eq!(mist_density(0.0, h), 0.0);
        assert_eq!(mist_density(-5.0, h), 0.0);
        assert_eq!(mist_density(-1.0, -1.0), 0.0);

        let base = mist_density(-h, h);
        let below = mist_density(-h * 1.5, h);
        assert!(base > 0.8 && base <= 1.0);
        assert!(below > 0.0 && below < base);
        assert!(mist_density(-h * 4.0, h) < below);
    }
}

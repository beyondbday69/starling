// w3_sky.rs
//
// Cheap procedural sky reflection color for Starling Water v4.
//
// Render-agnostic, allocation-free, O(1) per sample. Given a reflected view ray
// and a pair of sky colors, this synthesizes a plausible reflected sky color:
//
//   * Vertical gradient: the reflected ray's Y component blends between the
//     `horizon` color (y <= 0) and the `top` color (y >= 1), with a smoothstep
//     so the horizon band is soft rather than a hard seam. Rays pointing below
//     the horizon are clamped to the horizon color (no ground color is known).
//   * Sun tint: a tight cosine lobe around `sun_dir` adds a warm additive glow,
//     concentrated wherever the reflected ray points toward the sun. Because a
//     low sun naturally sits near the horizon, this doubles as a cheap
//     horizon/sunset tint without any extra parameters.
//
// Coordinate system: X/Z horizontal, Y up. All math is f32, no external crates.

/// Warm additive sun/sunset tint (RGB). Red is strongest, blue weakest.
pub const SUN_TINT: [f32; 3] = [1.0, 0.86, 0.62];

/// Cosine-lobe exponent for the sun glow. Higher means a smaller, sharper sun.
/// Implemented with exponentiation by squaring, so any small integer is cheap.
pub const SUN_GLOW_POWER: u32 = 16;

/// Overall strength of the additive sun glow on top of the sky gradient.
pub const SUN_GLOW_STRENGTH: f32 = 0.35;

/// Squared-length threshold below which a direction vector is treated as zero.
const MIN_DIR_LEN2: f32 = 1.0e-8;

/// Clamped Hermite smoothstep: 0 at t<=0, 1 at t>=1, smooth in between.
#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Integer exponent via exponentiation by squaring (avoid a libm `powf` call).
#[inline]
fn pow_u32(mut base: f32, mut exp: u32) -> f32 {
    let mut result = 1.0f32;
    while exp > 0 {
        if exp & 1 == 1 {
            result *= base;
        }
        base *= base;
        exp >>= 1;
    }
    result
}

/// Compute a cheap procedural reflected-sky color.
///
/// # Parameters
/// * `refl_dir` - reflected ray direction in world space ([x, y, z], Y up).
///   Its Y component drives the horizon -> top gradient; its full vector is
///   compared against `sun_dir` for the sun glow. Need not be pre-normalized.
/// * `top` - sky color at the zenith (used when `refl_dir.y >= 1`).
/// * `horizon` - sky color at/below the horizon (used when `refl_dir.y <= 0`).
/// * `sun_dir` - direction toward the sun. Need not be pre-normalized; a zero
///   or non-finite vector simply disables the sun glow.
///
/// # Returns
/// `[r, g, b]` reflected sky color. A non-finite `refl_dir` falls back to
/// `horizon`. No heap allocation, no globals, no external crates.
#[inline]
pub fn sky_reflect(
    refl_dir: [f32; 3],
    top: [f32; 3],
    horizon: [f32; 3],
    sun_dir: [f32; 3],
) -> [f32; 3] {
    let rx = refl_dir[0];
    let ry = refl_dir[1];
    let rz = refl_dir[2];

    // Degenerate reflected ray: fall back to the flat horizon color.
    if !rx.is_finite() || !ry.is_finite() || !rz.is_finite() {
        return horizon;
    }

    // Vertical gradient, clamped below the horizon and above the zenith.
    let t = smoothstep(ry);
    let mut out = [
        horizon[0] + (top[0] - horizon[0]) * t,
        horizon[1] + (top[1] - horizon[1]) * t,
        horizon[2] + (top[2] - horizon[2]) * t,
    ];

    // Sun glow: normalize both directions, take a cosine lobe, add a warm tint.
    // Reject a non-finite sun direction so the dot product can never become NaN.
    let sx = sun_dir[0];
    let sy = sun_dir[1];
    let sz = sun_dir[2];
    if !sx.is_finite() || !sy.is_finite() || !sz.is_finite() {
        return out;
    }
    let rlen2 = rx * rx + ry * ry + rz * rz;
    let slen2 = sx * sx + sy * sy + sz * sz;
    if rlen2 > MIN_DIR_LEN2 && slen2 > MIN_DIR_LEN2 {
        let inv_len = 1.0 / (rlen2.sqrt() * slen2.sqrt());
        let cos_theta = (rx * sx + ry * sy + rz * sz) * inv_len;
        let glow = pow_u32(cos_theta.max(0.0), SUN_GLOW_POWER) * SUN_GLOW_STRENGTH;
        out[0] += SUN_TINT[0] * glow;
        out[1] += SUN_TINT[1] * glow;
        out[2] += SUN_TINT[2] * glow;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-5
    }

    fn approx_vec(a: [f32; 3], b: [f32; 3]) -> bool {
        approx(a[0], b[0]) && approx(a[1], b[1]) && approx(a[2], b[2])
    }

    // A downward sun cannot glow against an upward ray, keeping gradient tests clean.
    const NO_SUN: [f32; 3] = [0.0, -1.0, 0.0];

    #[test]
    fn zenith_is_top_and_horizon_below_is_horizon() {
        let top = [0.20, 0.45, 0.85];
        let horizon = [0.75, 0.70, 0.62];

        assert!(approx_vec(
            sky_reflect([0.0, 1.0, 0.0], top, horizon, NO_SUN),
            top
        ));
        assert!(approx_vec(
            sky_reflect([0.0, 0.0, 1.0], top, horizon, NO_SUN),
            horizon
        ));
        // Rays pointing under the water surface are clamped to the horizon color.
        assert!(approx_vec(
            sky_reflect([0.0, -1.0, 0.0], top, horizon, NO_SUN),
            horizon
        ));
    }

    #[test]
    fn vertical_gradient_interpolates_and_is_monotonic() {
        let top = [0.10, 0.40, 0.90];
        let horizon = [0.80, 0.70, 0.50];
        let mid = sky_reflect([0.0, 0.5, 1.0], top, horizon, NO_SUN);

        // smoothstep(0.5) == 0.5, so the midpoint is the plain average.
        assert!(approx_vec(
            mid,
            [
                (top[0] + horizon[0]) * 0.5,
                (top[1] + horizon[1]) * 0.5,
                (top[2] + horizon[2]) * 0.5,
            ]
        ));

        // Rising ray Y smoothly decreases red and increases blue.
        let mut prev_red = f32::INFINITY;
        let mut prev_blue = f32::NEG_INFINITY;
        for i in 0..=10 {
            let y = i as f32 / 10.0;
            let c = sky_reflect([0.6, y, 0.8], top, horizon, NO_SUN);
            assert!(c[0] <= prev_red + 1.0e-6, "red should fall as ray rises");
            assert!(c[2] >= prev_blue - 1.0e-6, "blue should rise as ray rises");
            prev_red = c[0];
            prev_blue = c[2];
        }
    }

    #[test]
    fn sun_glow_is_warm_and_direction_sensitive() {
        let top = [0.20, 0.45, 0.85];
        let horizon = [0.75, 0.70, 0.62];
        let sun = [0.0, 1.0, 0.0];

        // Reflected ray aligned with the sun vs. pointed away at the same height.
        let toward = sky_reflect([0.0, 1.0, 0.0], top, horizon, sun);
        let away = sky_reflect([0.0, 1.0, 0.0], top, horizon, [0.0, -1.0, 0.0]);

        assert!(toward[0] > away[0], "sun should brighten the reflection");
        assert!(toward[1] > away[1]);
        assert!(toward[2] > away[2]);
        // Warm tint: red gains more than blue.
        assert!(
            (toward[0] - away[0]) > (toward[2] - away[2]),
            "sun tint should be warmer than neutral"
        );
        assert!(toward.iter().all(|c| c.is_finite()));
    }

    #[test]
    fn degenerate_inputs_are_safe() {
        let top = [0.2, 0.4, 0.8];
        let horizon = [0.7, 0.6, 0.5];

        assert_eq!(
            sky_reflect([f32::NAN, 1.0, 0.0], top, horizon, [0.0, 1.0, 0.0]),
            horizon
        );
        assert_eq!(
            sky_reflect([0.0, 1.0, f32::INFINITY], top, horizon, [0.0, 1.0, 0.0]),
            horizon
        );
        // Zero-length directions simply disable the glow, never produce NaN.
        let zero = sky_reflect([0.0, 0.5, 0.0], top, horizon, [0.0, 0.0, 0.0]);
        assert!(zero.iter().all(|c| c.is_finite()));
        assert!(approx_vec(
            zero,
            sky_reflect([0.0, 0.5, 0.0], top, horizon, [0.0, -1.0, 0.0])
        ));

        // A non-finite sun direction returns the pure gradient, never NaN.
        let nan_sun = sky_reflect(
            [0.0, 0.5, 0.0],
            top,
            horizon,
            [f32::NAN, 1.0, 0.0],
        );
        assert!(nan_sun.iter().all(|c| c.is_finite()));
        assert!(approx_vec(nan_sun, zero));
    }
}

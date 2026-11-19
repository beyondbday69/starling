// Beer-Lambert depth absorption for water overlays (water v4).
//
// Light travelling through water is attenuated exponentially with path length:
//
// ```text
// T(channel) = exp(-sigma(channel) * depth)
// ```
//
// Water absorbs long wavelengths (red) fastest and blue slowest. The biome
// water tint additionally biases attenuation per channel, so tropical water
// keeps more green/cyan while swamp water shifts toward dark olive-green.
//
// The base RGB is treated as linear reflectance/light arriving at the water
// surface. `absorb` returns the light that survives to the visible surface or
// seabed as linear RGB in `0.0..=1.0`. `depth_fog` returns the matching
// exponential fog factor for blending toward an underwater fog colour.
//
// Pure math, no allocation, no globals, no unsafe. Safe to call per vertex.

/// Absorption coefficient baseline for red light (per metre).
const SIGMA_R: f32 = 0.46;
/// Absorption coefficient baseline for green light (per metre).
const SIGMA_G: f32 = 0.16;
/// Absorption coefficient baseline for blue light (per metre).
const SIGMA_B: f32 = 0.085;

/// How strongly the biome tint can shift each absorption coefficient.
/// A darker tint channel means that colour is absorbed more quickly.
const TINT_INFLUENCE: f32 = 0.55;

/// Maximum depth in metres considered by `absorb`. Deeper water is saturated.
const MAX_DEPTH: f32 = 96.0;

/// Fog density baseline for `depth_fog` (per metre).
const FOG_DENSITY: f32 = 0.085;

/// Clamp a scalar to `0.0..=1.0` without pulling in `f32::clamp` overhead.
#[inline]
fn sat(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

/// Sanitize a raw depth value from a caller into a finite non-negative metre
/// count. Non-finite and negative depths become zero; huge depths saturate.
#[inline]
fn sanitize_depth(depth: f32) -> f32 {
    if !depth.is_finite() || depth <= 0.0 {
        0.0
    } else if depth > MAX_DEPTH {
        MAX_DEPTH
    } else {
        depth
    }
}

/// Effective absorption coefficient for one channel.
///
/// `tint` is a normalized biome water tint channel in `0.0..=1.0`. Bright
/// tint channels reduce absorption (they let that wavelength travel farther),
/// while dark channels increase it. The result is always positive.
#[inline]
fn sigma(base: f32, tint: f32) -> f32 {
    let t = sat(tint);
    (base * (1.0 + TINT_INFLUENCE * (1.0 - t))).max(0.001)
}

/// Apply Beer-Lambert transmission to an RGB colour.
///
/// * `base_rgb` - incoming linear RGB in `0.0..=1.0` (or any finite range).
/// * `depth`    - optical path length through the water in metres.
/// * `tint`     - biome water tint as normalized `[r, g, b]` in `0.0..=1.0`,
///                for example from `w3_color::unpack_rgb(0x3F76E4)`.
///
/// Returns transmitted linear RGB clamped to `0.0..=1.0`. Zero depth returns
/// the clamped base colour; greater depth darkens red fastest, then green,
/// then blue. All inputs are sanitized, so untrusted values are safe.
#[inline]
pub fn absorb(base_rgb: [f32; 3], depth: f32, tint: [f32; 3]) -> [f32; 3] {
    let d = sanitize_depth(depth);
    let mut out = [0.0; 3];
    for i in 0..3 {
        let base = if base_rgb[i].is_finite() {
            sat(base_rgb[i])
        } else {
            0.0
        };
        let base_sigma = match i {
            0 => SIGMA_R,
            1 => SIGMA_G,
            _ => SIGMA_B,
        };
        // exp(-x) is in 0..=1 for non-negative x, so no extra clamp is needed.
        out[i] = base * (-sigma(base_sigma, tint[i]) * d).exp();
    }
    out
}

/// Exponential underwater fog factor for a given depth.
///
/// * `depth`   - optical path length through water in metres.
/// * `fog_end` - depth (metres) at which water becomes nearly opaque;
///               non-positive or non-finite values fall back to a safe default.
///
/// Returns `0.0` at the surface and approaches `1.0` at and beyond `fog_end`.
/// The curve matches `absorb`'s exponential falloff, making it suitable for
/// `mix(color, fog_color, fog)` blending in the renderer.
#[inline]
pub fn depth_fog(depth: f32, fog_end: f32) -> f32 {
    let d = sanitize_depth(depth);
    let end = if fog_end.is_finite() && fog_end > 0.0 {
        fog_end.min(MAX_DEPTH)
    } else {
        MAX_DEPTH
    };
    let density = if end > 0.0 { FOG_DENSITY * (MAX_DEPTH / end) } else { FOG_DENSITY };
    let fog = 1.0 - (-density * d).exp();
    sat(fog)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Normalized ocean-ish tint used by `w3_color::unpack_rgb(0x3F76E4)`.
    fn ocean_tint() -> [f32; 3] {
        [
            0x3F as f32 / 255.0,
            0x76 as f32 / 255.0,
            0xE4 as f32 / 255.0,
        ]
    }

    #[test]
    fn zero_depth_returns_base_and_clamps_channels() {
        let out = absorb([1.0, 0.5, 0.0], 0.0, ocean_tint());
        assert_eq!(out, [1.0, 0.5, 0.0]);

        // Raw values outside 0..1 are clamped even at the surface.
        let clamped = absorb([-1.0, 2.0, f32::NAN], 0.0, ocean_tint());
        assert_eq!(clamped, [0.0, 1.0, 0.0]);

        // Negative and non-finite depths behave as the surface.
        assert_eq!(absorb([0.8, 0.7, 0.6], -5.0, ocean_tint()), [0.8, 0.7, 0.6]);
        assert_eq!(absorb([0.8, 0.7, 0.6], f32::NAN, ocean_tint()), [0.8, 0.7, 0.6]);
    }

    #[test]
    fn deeper_water_darkens_red_faster_than_blue() {
        let base = [1.0, 1.0, 1.0];
        let shallow = absorb(base, 1.0, ocean_tint());
        let deep = absorb(base, 12.0, ocean_tint());

        for i in 0..3 {
            assert!(deep[i] < shallow[i], "channel {i} should dim with depth");
            assert!((0.0..=1.0).contains(&deep[i]));
        }
        assert!(deep[0] < deep[1], "red should attenuate faster than green");
        assert!(deep[1] < deep[2], "green should attenuate faster than blue");
        assert!(deep[2] > 0.0, "blue should still transmit in clear water");
    }

    #[test]
    fn tint_biases_absorption_per_channel() {
        let base = [1.0, 1.0, 1.0];
        // A tropical tint is bright green/cyan; a swamp tint is dark olive.
        let tropical = absorb(base, 6.0, [0.25, 0.85, 0.80]);
        let swamp = absorb(base, 6.0, [0.10, 0.25, 0.15]);
        for i in 0..3 {
            assert!(tropical[i] >= swamp[i], "channel {i}: brighter tint transmits more");
        }
    }

    #[test]
    fn depth_fog_is_monotonic_and_bounded() {
        assert_eq!(depth_fog(0.0, 20.0), 0.0);
        let near = depth_fog(2.0, 20.0);
        let mid = depth_fog(10.0, 20.0);
        let far = depth_fog(20.0, 20.0);
        assert!(near > 0.0 && mid > near && far > mid);
        assert!(far < 1.0);
        assert!(depth_fog(10_000.0, 20.0) <= 1.0);

        // Invalid fog_end falls back to a finite, bounded result.
        for end in [0.0, -10.0, f32::NAN, f32::INFINITY] {
            let f = depth_fog(8.0, end);
            assert!(f.is_finite() && (0.0..=1.0).contains(&f));
        }
    }

    #[test]
    fn absorption_matches_beer_lambert_law() {
        let tint = [0.5, 0.5, 0.5];
        let base = [1.0, 1.0, 1.0];
        let d = 3.5;
        let expected_red = (-SIGMA_R * (1.0 + TINT_INFLUENCE * 0.5) * d).exp();
        let got = absorb(base, d, tint);
        assert!((got[0] - expected_red).abs() < 1e-6);
    }
}

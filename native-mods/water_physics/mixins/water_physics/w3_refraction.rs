// w3_refraction.rs
//
// Cheap screen-space refraction offsets for Starling Water v4.
//
// This module is intentionally render-agnostic and allocation-free. It turns an
// upward-facing water-surface normal plus a caller supplied "strength" (depth in
// blocks folded together with any global refraction scale) into a small screen UV
// offset, and provides a per-channel dispersion (chromatic aberration) split.
//
// Model (kept deliberately O(1) and mobile friendly):
//   * The water surface is a height field in X/Z with Y up, so a valid normal has
//     normal.y > 0. Flat water is [0, 1, 0] and produces zero offset.
//   * Snell's law for air -> water (eta = 1 / IOR) bends the transmitted ray
//     *opposite* to the horizontal tilt of the normal. For small slopes the bend
//     magnitude is (1 - eta) ~ 0.25. A 1 / normal.y factor cheaply amplifies the
//     offset on steeper wave faces, matching the growing tangent of the tilt.
//   * The result is clamped to a sane screen-space range so a degenerate or
//     extreme input can never push a texture fetch outside the framebuffer.
//
// Coordinate system: X/Z horizontal, Y up. All math is f32, no external crates.

/// Index of refraction of water relative to air (used for the small-angle gain).
pub const WATER_IOR: f32 = 1.333;

/// Small-angle horizontal refraction gain: 1 - (air / water) = 1 - 1/IOR ~ 0.25.
pub const REFRACT_GAIN: f32 = 1.0 - 1.0 / WATER_IOR;

/// Smallest accepted normal.y. Steeper / downward normals are clamped to this so
/// the 1 / normal.y term cannot explode.
pub const MIN_NORMAL_Y: f32 = 0.25;

/// Largest absolute screen-UV offset per axis (keeps sampling in a sane range).
pub const MAX_UV_OFFSET: f32 = 0.25;

/// Red channel dispersion factor (red is bent slightly less than green).
pub const RED_DISPERSION: f32 = 0.94;

/// Blue channel dispersion factor (blue is bent slightly more than green).
pub const BLUE_DISPERSION: f32 = 1.06;

/// Clamp helper kept private; every public API item above is `pub`.
#[inline]
fn clamp_offset(v: f32) -> f32 {
    v.clamp(-MAX_UV_OFFSET, MAX_UV_OFFSET)
}

/// Compute a cheap screen-space UV offset for refraction through a wavy surface.
///
/// # Parameters
/// * `normal` - upward-facing water normal in world space ([x, y, z], X/Z
///   horizontal, Y up). Flat water is `[0.0, 1.0, 0.0]`.
/// * `strength` - combined depth/scale factor (e.g. water depth in blocks times
///   a global refraction tweak). Non-positive or non-finite values yield no
///   offset.
///
/// # Returns
/// `[offset_u, offset_v]` to add to a screen-space UV when sampling the
/// background/underwater color. The offset is opposite to the normal's horizontal
/// tilt (bending toward -N), as dictated by Snell's law for air -> water.
#[inline]
pub fn refract_offset(normal: [f32; 3], strength: f32) -> [f32; 2] {
    let nx = normal[0];
    let ny = normal[1];
    let nz = normal[2];

    // Reject degenerate / inverted / non-finite inputs up front. This keeps the
    // division and the final clamp free of NaN and infinity.
    if !strength.is_finite()
        || strength <= 0.0
        || !nx.is_finite()
        || !ny.is_finite()
        || !nz.is_finite()
        || ny <= 0.0
    {
        return [0.0, 0.0];
    }

    let inv_y = 1.0 / ny.max(MIN_NORMAL_Y);
    let scale = -REFRACT_GAIN * inv_y * strength;
    [clamp_offset(nx * scale), clamp_offset(nz * scale)]
}

/// Split a base refraction offset into per-channel (R, G, B) UV offsets for a
/// cheap chromatic aberration / dispersion effect.
///
/// Green keeps the original offset, red is scaled down and blue scaled up, so the
/// three samples stay collinear with the base offset and differ only in
/// magnitude. Return order is `(red, green, blue)`.
#[inline]
pub fn chromatic_offsets(base: [f32; 2]) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let red = [base[0] * RED_DISPERSION, base[1] * RED_DISPERSION];
    let blue = [base[0] * BLUE_DISPERSION, base[1] * BLUE_DISPERSION];
    (red, base, blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn flat_surface_has_zero_offset() {
        let offset = refract_offset([0.0, 1.0, 0.0], 0.5);
        assert!(approx(offset[0], 0.0));
        assert!(approx(offset[1], 0.0));
    }

    #[test]
    fn offset_opposes_horizontal_tilt_and_is_mirrored() {
        // Normal tilted toward +X (nx > 0) must bend the sample toward -X, and a
        // mirrored normal must give a mirrored offset with unchanged V component.
        let pos = refract_offset([0.5, 0.866_025_4, 0.0], 0.1);
        let neg = refract_offset([-0.5, 0.866_025_4, 0.0], 0.1);
        assert!(pos[0] < 0.0, "offset should oppose +X tilt");
        assert!(approx(pos[1], 0.0));
        assert!(approx(pos[0], -neg[0]));
        assert!(approx(pos[1], neg[1]));
    }

    #[test]
    fn offset_scales_linearly_and_is_bounded() {
        let n = [0.6, 0.8, 0.0];
        let a = refract_offset(n, 0.01);
        let b = refract_offset(n, 0.02);
        assert!(approx(b[0], 2.0 * a[0]));
        assert!(approx(b[1], 2.0 * a[1]));

        let huge = refract_offset(n, 1.0e6);
        assert!(huge[0].is_finite() && huge[1].is_finite());
        assert!(huge[0].abs() <= MAX_UV_OFFSET + 1e-6);
        assert!(huge[1].abs() <= MAX_UV_OFFSET + 1e-6);
        assert!(approx(huge[0], -MAX_UV_OFFSET));
    }

    #[test]
    fn degenerate_inputs_are_safe() {
        assert_eq!(refract_offset([0.0, 1.0, 0.0], 0.0), [0.0, 0.0]);
        assert_eq!(refract_offset([0.0, 1.0, 0.0], -2.0), [0.0, 0.0]);
        assert_eq!(refract_offset([1.0, 0.0, 0.0], 0.1), [0.0, 0.0]);
        assert_eq!(refract_offset([f32::NAN, 1.0, 0.0], 0.1), [0.0, 0.0]);
        assert_eq!(refract_offset([0.0, 1.0, 0.0], f32::INFINITY), [0.0, 0.0]);
    }

    #[test]
    fn chromatic_split_brackets_green_and_stays_collinear() {
        let base = [0.10_f32, -0.04_f32];
        let (red, green, blue) = chromatic_offsets(base);

        assert_eq!(green, base);
        assert!(red[0] < green[0] && green[0] < blue[0]);
        assert!(approx(red[0], base[0] * RED_DISPERSION));
        assert!(approx(blue[1], base[1] * BLUE_DISPERSION));

        // All three offsets lie on the same ray from the origin.
        assert!(approx(red[0] * green[1] - red[1] * green[0], 0.0));
        assert!(approx(blue[0] * green[1] - blue[1] * green[0], 0.0));
    }

    #[test]
    fn chromatic_zero_base_stays_zero() {
        let (red, green, blue) = chromatic_offsets([0.0, 0.0]);
        assert_eq!(red, [0.0, 0.0]);
        assert_eq!(green, [0.0, 0.0]);
        assert_eq!(blue, [0.0, 0.0]);
    }
}

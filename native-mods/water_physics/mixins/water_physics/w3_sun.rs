// w3_sun.rs
//
// GGX-ish specular sun glint for Starling Water v4.
//
// The glint is evaluated from the half vector between the view and sun
// directions. A water facet reflects the sun toward the viewer when its normal
// is aligned with that half vector, so the cosine between the surface normal
// and the half vector drives a compact microfacet lobe. Roughness widens the
// lobe by increasing the GGX slope variance; smooth water produces a tight,
// bright highlight and choppy water produces a broad, dimmer sparkle band.
//
// Coordinate system: X/Z horizontal, Y up. Directions need not be normalized;
// zero or non-finite vectors are handled safely. Pure f32 math, no globals,
// no unsafe, no allocation, no external crates.

/// Maximum returned glint intensity (before cosine/geometry factors).
pub const GLINT_MAX: f32 = 1.0;

/// Warm sunlight tint (RGB) for mapping intensity to a highlight colour.
pub const GLINT_TINT: [f32; 3] = [1.0, 0.94, 0.80];

/// Minimum GGX slope variance, keeping mirrors from becoming a singularity.
const MIN_ALPHA: f32 = 0.015;
/// Maximum GGX slope variance, keeping the lobe finite for extreme roughness.
const MAX_ALPHA: f32 = 1.0;
/// Squared-length threshold below which a vector is treated as degenerate.
const MIN_DIR_LEN2: f32 = 1.0e-8;

/// Clamp a scalar to `0.0..=1.0`.
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

/// Normalize a 3-vector, returning `None` for zero or non-finite inputs.
#[inline]
fn unit(v: [f32; 3]) -> Option<[f32; 3]> {
    let x = v[0];
    let y = v[1];
    let z = v[2];
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return None;
    }
    let len_sq = x * x + y * y + z * z;
    if len_sq < MIN_DIR_LEN2 {
        return None;
    }
    let inv_len = 1.0 / len_sq.sqrt();
    Some([x * inv_len, y * inv_len, z * inv_len])
}

/// Map roughness in `0..=1` to a clamped GGX slope variance.
#[inline]
fn alpha_from_roughness(roughness: f32) -> f32 {
    let r = if roughness.is_finite() { sat(roughness) } else { 1.0 };
    (r * r).clamp(MIN_ALPHA, MAX_ALPHA)
}

/// GGX-ish specular sun glint intensity in `0..=1`.
///
/// * `normal` - upward water surface normal (need not be normalized).
/// * `view` - direction from the surface toward the viewer/camera.
/// * `sun` - direction from the surface toward the sun.
/// * `roughness` - `0.0` is mirror-calm, `1.0` is very choppy.
///
/// The half vector `normalize(view + sun)` is the mirror normal required for a
/// specular reflection. Its cosine with the surface normal feeds a GGX-style
/// microfacet distribution, and a Schlick-ish grazing term suppresses glints
/// seen at extreme angles. A small broad lobe is added so rough water still
/// reads as a visible scattering highlight rather than isolated pinpricks.
/// Returns `0.0` for any degenerate or invalid direction.
#[inline]
pub fn sun_glint(normal: [f32; 3], view: [f32; 3], sun: [f32; 3], roughness: f32) -> f32 {
    let n = match unit(normal) {
        Some(v) => v,
        None => return 0.0,
    };
    let v = match unit(view) {
        Some(v) => v,
        None => return 0.0,
    };
    let s = match unit(sun) {
        Some(v) => v,
        None => return 0.0,
    };

    // Half vector between the view and light directions. Opposite directions
    // cannot produce a reflection for an above-surface normal.
    let hx = v[0] + s[0];
    let hy = v[1] + s[1];
    let hz = v[2] + s[2];
    let h = match unit([hx, hy, hz]) {
        Some(v) => v,
        None => return 0.0,
    };

    let n_dot_h = n[0] * h[0] + n[1] * h[1] + n[2] * h[2];
    if n_dot_h <= 0.0 {
        return 0.0;
    }

    let n_dot_v = (n[0] * v[0] + n[1] * v[1] + n[2] * v[2]).max(0.0);
    let n_dot_s = (n[0] * s[0] + n[1] * s[1] + n[2] * s[2]).max(0.0);
    if n_dot_v <= 0.0 || n_dot_s <= 0.0 {
        return 0.0;
    }

    // Suppress reflection when either ray grazes the surface. This is a cheap
    // Fresnel/geometry approximation and keeps back-face normals dark.
    let grazing = (1.0 - n_dot_v) * (1.0 - n_dot_s);
    let visibility = n_dot_v * n_dot_s / (1.0 + 2.0 * grazing);

    let alpha = alpha_from_roughness(roughness);
    let alpha2 = alpha * alpha;
    let cos2 = n_dot_h * n_dot_h;
    let denom = cos2 * (alpha2 - 1.0) + 1.0;

    // GGX microfacet distribution, peak-normalized so the aligned case is 1.0
    // for every roughness: `alpha2 / denom` equals 1.0 at n_dot_h == 1 and
    // falls off away from the mirror direction. `denom >= alpha2` always, so
    // the ratio stays finite even as roughness approaches zero.
    let nd = if denom > 1.0e-6 {
        sat(alpha2 / denom)
    } else {
        1.0
    };

    // Choppier water spreads the same energy over a wider lobe, lowering the
    // peak. This keeps calm water as a tight, bright sparkle.
    let peak_gain = 1.0 / (1.0 + 2.0 * alpha);
    let spec = nd * nd * visibility * peak_gain;

    // Broad scattering lobe so rough water still reads as a visible highlight
    // rather than isolated pinpricks. It scales with peak_gain too, so adding
    // roughness widens the glint without making its peak brighter.
    let broad = (alpha2 / (alpha2 + 0.35 * (1.0 - cos2))) * 0.20 * visibility * peak_gain;
    sat(spec + broad).min(GLINT_MAX)
}

/// Map a glint intensity to a warm linear RGB highlight colour in `0..=1`.
///
/// Output is additive: multiply by the desired sun brightness before adding to
/// the water colour. Inputs are clamped, so non-finite values return black.
#[inline]
pub fn glint_color(intensity: f32) -> [f32; 3] {
    let i = if intensity.is_finite() { sat(intensity) } else { 0.0 };
    [
        GLINT_TINT[0] * i,
        GLINT_TINT[1] * i,
        GLINT_TINT[2] * i,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn mirror_alignment_peaks_and_misalignment_is_dark() {
        let n = [0.0, 1.0, 0.0];
        // Symmetric view/sun directions have an upward half vector equal to N.
        let v = [-0.6, 0.8, 0.0];
        let s = [0.6, 0.8, 0.0];
        let aligned = sun_glint(n, v, s, 0.05);
        assert!(aligned > 0.5, "expected strong mirror glint, got {aligned}");

        // A normal tilted far away from H should produce no highlight.
        let tilted = sun_glint([0.9_f32.sqrt(), 0.0, 0.0], v, s, 0.05);
        assert_eq!(tilted, 0.0);
    }

    #[test]
    fn roughness_widens_lobe_and_lowers_peak() {
        let n = [0.0, 1.0, 0.0];
        let v = [-0.6, 0.8, 0.0];
        let s = [0.6, 0.8, 0.0];

        let calm = sun_glint(n, v, s, 0.02);
        let choppy = sun_glint(n, v, s, 0.6);
        assert!(calm > choppy, "calm water should be sharper/brighter: {calm} vs {choppy}");

        // A facet tilted 30 degrees away from H catches light when water is rough.
        let tilted_n = [0.5, 0.866_025_4, 0.0];
        let off_calm = sun_glint(tilted_n, v, s, 0.02);
        let off_choppy = sun_glint(tilted_n, v, s, 0.6);
        assert!(off_choppy > off_calm, "roughness should widen the glint lobe");
    }

    #[test]
    fn result_is_bounded_and_handles_degenerate_inputs() {
        let n = [0.0, 1.0, 0.0];
        let v = [0.0, 0.7, 0.7];
        let s = [0.0, 0.9, -0.4];
        for r in [0.0, 0.01, 0.25, 0.5, 1.0, -3.0, 9.0, f32::NAN] {
            let g = sun_glint(n, v, s, r);
            assert!(g.is_finite(), "non-finite glint for roughness {r}");
            assert!((0.0..=GLINT_MAX).contains(&g), "out of range: {g}");
        }

        assert_eq!(sun_glint([0.0, 0.0, 0.0], v, s, 0.3), 0.0);
        assert_eq!(sun_glint(n, [0.0, 0.0, 0.0], s, 0.3), 0.0);
        assert_eq!(sun_glint(n, v, [0.0, 0.0, 0.0], 0.3), 0.0);
        assert_eq!(sun_glint([f32::NAN, 1.0, 0.0], v, s, 0.3), 0.0);
    }

    #[test]
    fn glint_color_is_warm_bounded_and_safe() {
        let black = glint_color(0.0);
        let full = glint_color(1.0);

        assert_eq!(black, [0.0, 0.0, 0.0]);
        assert!(approx(full[0], 1.0, 1e-6));
        assert!(full[0] > full[1] && full[1] > full[2], "highlight should be warm");
        for c in full {
            assert!(c.is_finite() && (0.0..=1.0).contains(&c));
        }

        assert_eq!(glint_color(-2.0), black);
        assert_eq!(glint_color(5.0), full);
        assert_eq!(glint_color(f32::NAN), black);
    }
}

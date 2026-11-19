// w3_fresnel.rs
//
// Schlick Fresnel reflectance for Starling Water v4.
//
// Pure, allocation-free, render-agnostic f32 math. No external crates, no globals,
// no unsafe. The module answers a single question used by the reflection/refraction
// pass: "how much of the incoming view ray is reflected by the water surface?"
//
// Model:
//   * Water/air interface with IOR = 1.333 gives a normal-incidence reflectance
//     F0 = ((1 - ior) / (1 + ior))^2 ~= 0.0204 (about 2% of light reflected when
//     looking straight down).
//   * Schlick's approximation adds the grazing boost term (1 - cos(theta))^5,
//     which drives reflectance smoothly to 1.0 as the view ray approaches a
//     tangent to the surface. That is why water looks mirror-like near the horizon.
//   * `fresnel_water` derives cos(theta) from the caller's normal and view
//     direction, normalizing both so raw (unnormalized) vectors are safe, and
//     taking the absolute value so either view-direction convention works.
//   * All inputs are validated: NaN / infinity / zero-length vectors fall back
//     to the neutral F0 value, and every result is clamped into [0, 1].
//
// Coordinate system: X/Z horizontal, Y up. Flat water normal is [0, 1, 0].
// Angles are implicit in the dot product; no radians are exposed here.

/// Index of refraction of water relative to air.
pub const WATER_IOR: f32 = 1.333;

/// Normal-incidence reflectance for air -> water: `((1 - IOR) / (1 + IOR))^2`.
/// For IOR = 1.333 this is about 0.0204 (~2%).
pub const WATER_F0: f32 = ((1.0 - WATER_IOR) / (1.0 + WATER_IOR))
    * ((1.0 - WATER_IOR) / (1.0 + WATER_IOR));

/// Compute Schlick's grazing-boost factor `(1 - cos(theta))^5`.
///
/// `cos_theta` is clamped to [0, 1] first. Non-finite input returns 0.0 (no
/// boost), which is the neutral choice. Exposed so the render pass can reuse the
/// same grazing ramp for foam/spray highlights without duplicating the polynomial.
#[inline]
pub fn grazing_boost(cos_theta: f32) -> f32 {
    if !cos_theta.is_finite() {
        return 0.0;
    }
    let g = 1.0 - cos_theta.clamp(0.0, 1.0);
    let g2 = g * g;
    let g4 = g2 * g2;
    g4 * g
}

/// Schlick Fresnel reflectance for the water/air interface.
///
/// `cos_theta` is the cosine of the angle between the surface normal and the
/// view direction. It is clamped to [0, 1] before evaluation:
/// * `cos_theta = 1.0` (looking straight down) returns `WATER_F0` ~= 0.02.
/// * `cos_theta = 0.0` (grazing) returns 1.0.
/// * Negative or non-finite values are treated as the neutral normal-incidence
///   case for non-finite input, and as grazing for negative finite input.
///
/// The returned value is always finite and in [0, 1].
#[inline]
pub fn fresnel_schlick(cos_theta: f32) -> f32 {
    if !cos_theta.is_finite() {
        return WATER_F0;
    }
    let f0 = WATER_F0;
    let f = f0 + (1.0 - f0) * grazing_boost(cos_theta);
    f.clamp(0.0, 1.0)
}

/// Fresnel reflectance from a world-space water normal and a view direction.
///
/// Both vectors are normalized internally, so callers may pass raw deltas
/// (e.g. `camera_pos - surface_pos`) without pre-normalizing. The absolute value
/// of the dot product is used, so the function works whether `view_dir` points
/// from the surface toward the camera or from the camera toward the surface.
///
/// Returns `WATER_F0` for degenerate (zero-length or non-finite) inputs and the
/// clamped Schlick reflectance otherwise. Always finite, always in [0, 1].
#[inline]
pub fn fresnel_water(normal: [f32; 3], view_dir: [f32; 3]) -> f32 {
    let nx = normal[0];
    let ny = normal[1];
    let nz = normal[2];
    let vx = view_dir[0];
    let vy = view_dir[1];
    let vz = view_dir[2];

    if !nx.is_finite() || !ny.is_finite() || !nz.is_finite()
        || !vx.is_finite() || !vy.is_finite() || !vz.is_finite()
    {
        return WATER_F0;
    }

    let n_len_sq = nx * nx + ny * ny + nz * nz;
    let v_len_sq = vx * vx + vy * vy + vz * vz;
    if n_len_sq <= 0.0 || v_len_sq <= 0.0 {
        return WATER_F0;
    }

    let inv_n = 1.0 / n_len_sq.sqrt();
    let inv_v = 1.0 / v_len_sq.sqrt();
    let cos_theta = (nx * inv_n) * (vx * inv_v)
        + (ny * inv_n) * (vy * inv_v)
        + (nz * inv_n) * (vz * inv_v);

    fresnel_schlick(cos_theta.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn normal_incidence_is_about_two_percent() {
        let f = fresnel_schlick(1.0);
        assert!(approx(f, 0.02, 0.002), "F0 should be ~0.02, got {}", f);
        assert!(approx(f, WATER_F0, 1e-6));

        // Flat surface viewed from directly above.
        let fw = fresnel_water([0.0, 1.0, 0.0], [0.0, 5.0, 0.0]);
        assert!(approx(fw, WATER_F0, 1e-5));
    }

    #[test]
    fn grazing_angle_reflects_almost_completely() {
        let f = fresnel_schlick(0.0);
        assert!(approx(f, 1.0, 1e-6), "grazing F should be 1.0, got {}", f);

        // View ray tangent to a flat surface.
        let fw = fresnel_water([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(approx(fw, 1.0, 1e-6));
    }

    #[test]
    fn reflectance_rises_monotonically_toward_grazing() {
        let mut prev = 2.0;
        for i in 0..=20 {
            let c = i as f32 / 20.0; // 0 -> 1: cos grows, reflectance must shrink
            let f = fresnel_schlick(c);
            assert!(f <= prev + 1e-6, "F must not increase as cos grows");
            assert!((0.0..=1.0).contains(&f));
            prev = f;
        }
        // Endpoint sanity: grazing (cos 0) >= normal incidence (cos 1).
        assert!(fresnel_schlick(0.0) > fresnel_schlick(1.0));
    }

    #[test]
    fn grazing_boost_matches_polynomial_and_is_bounded() {
        assert!(approx(grazing_boost(1.0), 0.0, 1e-6));
        assert!(approx(grazing_boost(0.0), 1.0, 1e-6));
        assert!(approx(grazing_boost(0.5), 0.03125, 1e-6));
        assert!(approx(grazing_boost(-3.0), 1.0, 1e-6));
        assert!(approx(grazing_boost(f32::NAN), 0.0, 1e-6));
    }

    #[test]
    fn inputs_are_clamped_and_safe() {
        // Out-of-range cosines are clamped, never produce NaN or >1 values.
        assert!(approx(fresnel_schlick(2.0), WATER_F0, 1e-6));
        assert!(approx(fresnel_schlick(-1.0), 1.0, 1e-6));
        assert!(fresnel_schlick(f32::NAN).is_finite());
        assert!(fresnel_schlick(f32::INFINITY).is_finite());

        // Degenerate vectors fall back to F0 instead of dividing by zero.
        assert!(approx(fresnel_water([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]), WATER_F0, 1e-6));
        assert!(approx(fresnel_water([0.0, 1.0, 0.0], [0.0, 0.0, 0.0]), WATER_F0, 1e-6));
        assert!(approx(fresnel_water([f32::NAN, 1.0, 0.0], [0.0, 1.0, 0.0]), WATER_F0, 1e-6));
    }

    #[test]
    fn view_direction_convention_does_not_matter() {
        let a = fresnel_water([0.0, 1.0, 0.0], [0.6, 0.8, 0.0]);
        let b = fresnel_water([0.0, 1.0, 0.0], [-0.6, -0.8, 0.0]);
        assert!(approx(a, b, 1e-5));
        assert!(a > WATER_F0 && a < 1.0);
    }
}

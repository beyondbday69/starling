// w3_normals.rs
// Finite-difference surface normals for the v4 water height field.
//
// Coordinate system: X/Z horizontal, Y up, Minecraft blocks as metres.
// The surface is described by a height function h(x, z). The upward-pointing
// normal is proportional to (-dh/dx, 1, -dh/dz).
//
// No globals, no unsafe, no allocation. All state is passed explicitly.

/// Returns `[0, 1, 0]` for zero/invalid vectors and a unit vector otherwise.
#[inline]
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !len_sq.is_finite() || len_sq <= 0.0 {
        return [0.0, 1.0, 0.0];
    }
    let inv_len = 1.0 / len_sq.sqrt();
    [v[0] * inv_len, v[1] * inv_len, v[2] * inv_len]
}

/// Substitutes `fallback` for any non-finite value (NaN / infinity).
#[inline]
fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

/// Computes an upward surface normal from a height closure using central
/// differences with step `eps` in both X and Z.
///
/// `f(x, z)` returns the water height in blocks at horizontal position
/// `(x, z)`. `eps` is the sampling distance; values `<= 0` or non-finite are
/// replaced by a small default (`1e-3`). The returned vector is always
/// normalized and has a positive Y component unless the surface is vertical
/// or the inputs are degenerate, in which case `[0, 1, 0]` is returned.
#[inline]
pub fn normal_from_height<F>(f: F, x: f32, z: f32, eps: f32) -> [f32; 3]
where
    F: Fn(f32, f32) -> f32,
{
    let step = if eps.is_finite() && eps > 0.0 { eps } else { 1e-3 };
    let h_left = f(x - step, z);
    let h_right = f(x + step, z);
    let h_down = f(x, z - step);
    let h_up = f(x, z + step);
    normal_from_grid(h_left, h_right, h_down, h_up, step)
}

/// Computes an upward surface normal from four height samples on an
/// axis-aligned grid with spacing `spacing`.
///
/// `h_left` / `h_right` are the heights at `x - spacing` / `x + spacing`.
/// `h_down` / `h_up` are the heights at `z - spacing` / `z + spacing`.
/// Non-positive or non-finite spacing falls back to `1.0`. Non-finite
/// heights are treated as `0.0`. The result is normalized and Y-up.
#[inline]
pub fn normal_from_grid(
    h_left: f32,
    h_right: f32,
    h_down: f32,
    h_up: f32,
    spacing: f32,
) -> [f32; 3] {
    let step = if spacing.is_finite() && spacing > 0.0 {
        spacing
    } else {
        1.0
    };
    let hl = finite_or(h_left, 0.0);
    let hr = finite_or(h_right, 0.0);
    let hd = finite_or(h_down, 0.0);
    let hu = finite_or(h_up, 0.0);

    // Normal proportional to (-dh/dx, 1, -dh/dz). Multiplying through by 2*spacing
    // removes the division while preserving direction.
    normalize3([hl - hr, 2.0 * step, hd - hu])
}

/// Applies a small ripple height field, given by its slopes `ripple_dx` and
/// `ripple_dz`, on top of an existing surface normal `base`.
///
/// The base normal is converted back to its implied height-field slopes and
/// the ripple slopes are added before rebuilding the normal. This composes
/// slopes correctly even on steep waves. If `base` is not Y-up enough
/// (Y <= 1e-6), a simple additive tilt is used as a safe fallback. The result
/// is always normalized and Y-up.
#[inline]
pub fn perturb_normal(base: [f32; 3], ripple_dx: f32, ripple_dz: f32) -> [f32; 3] {
    let base = normalize3(base);
    let rdx = finite_or(ripple_dx, 0.0);
    let rdz = finite_or(ripple_dz, 0.0);

    if base[1] > 1e-6 {
        let slope_x = -base[0] / base[1] + rdx;
        let slope_z = -base[2] / base[1] + rdz;
        normalize3([-slope_x, 1.0, -slope_z])
    } else {
        normalize3([base[0] - rdx, base[1], base[2] - rdz])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline]
    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[inline]
    fn assert_normal_eq(actual: [f32; 3], expected: [f32; 3], tol: f32) {
        assert!(
            approx(actual[0], expected[0], tol)
                && approx(actual[1], expected[1], tol)
                && approx(actual[2], expected[2], tol),
            "expected {:?}, got {:?}",
            expected,
            actual
        );
    }

    #[test]
    fn flat_surface_is_y_up() {
        let n = normal_from_height(|_, _| 0.0, 3.0, -7.0, 0.01);
        assert_normal_eq(n, [0.0, 1.0, 0.0], 1e-6);

        let g = normal_from_grid(1.5, 1.5, 1.5, 1.5, 0.25);
        assert_normal_eq(g, [0.0, 1.0, 0.0], 1e-6);
    }

    #[test]
    fn planar_slope_matches_analytic_normal() {
        // h(x, z) = 0.5*x - 0.25*z  =>  dh/dx = 0.5, dh/dz = -0.25
        // Upward normal ∝ (-0.5, 1, 0.25).
        let f = |x: f32, z: f32| 0.5 * x - 0.25 * z;
        let n = normal_from_height(f, 2.0, -3.0, 0.001);
        let len = (0.5_f32 * 0.5 + 1.0 + 0.25 * 0.25).sqrt();
        let expected = [-0.5 / len, 1.0 / len, 0.25 / len];
        assert_normal_eq(n, expected, 1e-4);

        // Grid helper must agree with the closure helper.
        let eps = 0.001;
        let g = normal_from_grid(
            f(2.0 - eps, -3.0),
            f(2.0 + eps, -3.0),
            f(2.0, -3.0 - eps),
            f(2.0, -3.0 + eps),
            eps,
        );
        assert_normal_eq(g, expected, 1e-4);
    }

    #[test]
    fn normal_is_unit_length_and_y_up_for_curved_surface() {
        let n = normal_from_height(|x, z| (x * x + z * z) * 0.1, 1.0, 2.0, 0.01);
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!(approx(len, 1.0, 1e-5), "length was {}", len);
        assert!(n[1] > 0.0, "expected a Y-up normal, got {:?}", n);
    }

    #[test]
    fn perturb_normal_composes_ripple_slope_on_flat_base() {
        let flat = [0.0, 1.0, 0.0];
        let n = perturb_normal(flat, 1.0, 0.0);
        let inv = 2.0_f32.sqrt().recip();
        assert_normal_eq(n, [-inv, inv, 0.0], 1e-6);

        let n2 = perturb_normal(flat, 0.0, -2.0);
        let inv5 = 5.0_f32.sqrt().recip();
        assert_normal_eq(n2, [0.0, inv5, 2.0 * inv5], 1e-6);
    }

    #[test]
    fn perturb_normal_preserves_steep_base_direction() {
        // Base h = 10*x gives slope 10, normal ≈ (-10, 1, 0)/sqrt(101).
        let base = normal_from_height(|x, _| 10.0 * x, 0.0, 0.0, 0.001);
        let n = perturb_normal(base, 0.0, 0.0);
        assert_normal_eq(n, base, 1e-5);

        // Adding a small Z ripple should not materially change the X tilt.
        let nudged = perturb_normal(base, 0.0, 0.1);
        assert!(nudged[0] < -0.9, "unexpected X component: {:?}", nudged);
    }

    #[test]
    fn degenerate_inputs_are_finite_and_y_up() {
        let n = normal_from_grid(f32::NAN, f32::INFINITY, f32::NAN, f32::NAN, 0.0);
        assert_normal_eq(n, [0.0, 1.0, 0.0], 1e-6);

        let p = perturb_normal([f32::NAN, 0.0, 0.0], f32::NAN, f32::INFINITY);
        assert!(p.iter().all(|v| v.is_finite()), "got {:?}", p);
        assert_normal_eq(p, [0.0, 1.0, 0.0], 1e-6);
    }
}

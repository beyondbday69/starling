// w3_viscosity.rs
// Fluid damping and friction helpers for water entry, wading and ripple decay.
//
// Coordinate system: X/Z horizontal, Y up, Minecraft blocks as metres.
// Pure, stateless and allocation-free. No globals, no unsafe.
// All functions return dimensionless values that callers can use as
// velocity/attenuation multipliers or per-step loss coefficients.

/// Fraction of motion lost to fluid drag for a body moving through water.
///
/// The result is a smooth, saturating coefficient in `[0, 1)`:
/// * `0.0` means no damping (still air, zero speed or zero viscosity);
/// * values near `1.0` mean the fluid removes almost all velocity in one step.
///
/// Both a linear (Stokes-like) and a quadratic (inertial) term are used, then
/// the sum is mapped through `q / (1 + q)` so the value stays bounded even for
/// very fast or very viscous motion.
#[inline]
pub fn drag_factor(speed: f32, viscosity: f32) -> f32 {
    if !speed.is_finite() || !viscosity.is_finite() {
        return 0.0;
    }
    let s = speed.abs();
    let v = viscosity.max(0.0);
    if s == 0.0 || v == 0.0 {
        return 0.0;
    }
    // Linear plus quadratic resistance. The 0.05 coefficient keeps the
    // quadratic term small at walking speeds but dominant during fast entry.
    let q = v * s + 0.05 * v * s * s;
    if !q.is_finite() {
        return 1.0 - f32::EPSILON;
    }
    // `min` keeps the result strictly below 1.0 even when f32 rounding would
    // otherwise collapse `q / (1 + q)` to exactly 1 for huge q.
    (q / (1.0 + q)).min(1.0 - f32::EPSILON)
}

/// Amplitude multiplier for a ripple or wake after `age` seconds.
///
/// Returns a value in `(0, 1]`:
/// * `1.0` at `age == 0` (fresh disturbance);
/// * exponentially smaller as `age` or `viscosity` grows.
///
/// Thicker fluid kills waves faster. Invalid inputs are treated as fully
/// damped (`0.0`) so a bad value can never resurrect a dead ripple.
#[inline]
pub fn wave_damping(age: f32, viscosity: f32) -> f32 {
    if !age.is_finite() || !viscosity.is_finite() {
        return 0.0;
    }
    let t = age.max(0.0);
    let v = viscosity.max(0.0);
    if t == 0.0 || v == 0.0 {
        return 1.0;
    }
    let exponent = -v * t;
    let decay = exponent.exp();
    if decay.is_finite() { decay.clamp(0.0, 1.0) } else { 0.0 }
}

/// Depth-dependent drag coefficient for wading or submerged entities.
///
/// `depth` is the vertical distance below the water surface in metres.
/// The result is a smooth saturating value in `[0, 1)` with a characteristic
/// depth of one metre: shallow water barely slows movement, while deeper
/// water approaches full drag. Negative and non-finite depths yield `0.0`.
#[inline]
pub fn depth_drag(depth: f32) -> f32 {
    if !depth.is_finite() || depth <= 0.0 {
        return 0.0;
    }
    let d = depth;
    d / (1.0 + d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_factor_is_zero_at_rest_and_grows_with_speed_and_viscosity() {
        assert_eq!(drag_factor(0.0, 1.0), 0.0);
        assert_eq!(drag_factor(5.0, 0.0), 0.0);

        let slow = drag_factor(0.5, 0.8);
        let fast = drag_factor(4.0, 0.8);
        let thick = drag_factor(4.0, 2.0);
        assert!(slow > 0.0 && slow < fast, "{slow} vs {fast}");
        assert!(fast < thick, "{fast} vs {thick}");

        // Always bounded below one, even for extreme inputs.
        let extreme = drag_factor(1.0e6, 1.0e6);
        assert!(extreme >= 0.0 && extreme < 1.0, "extreme was {extreme}");
    }

    #[test]
    fn wave_damping_decays_with_age_and_viscosity() {
        assert_eq!(wave_damping(0.0, 1.0), 1.0);
        assert_eq!(wave_damping(2.0, 0.0), 1.0);

        let young = wave_damping(0.25, 0.9);
        let old = wave_damping(2.0, 0.9);
        let viscous = wave_damping(2.0, 2.5);
        assert!(young > old, "{young} vs {old}");
        assert!(old > viscous, "{old} vs {viscous}");

        let expected = (-0.9f32 * 2.0).exp();
        assert!((old - expected).abs() < 1e-6, "old was {old}, expected {expected}");
        assert!(viscous > 0.0);
    }

    #[test]
    fn depth_drag_saturates_with_depth_and_rejects_shallow_values() {
        assert_eq!(depth_drag(-1.0), 0.0);
        assert_eq!(depth_drag(0.0), 0.0);

        let shallow = depth_drag(0.25);
        let one_metre = depth_drag(1.0);
        let deep = depth_drag(8.0);
        assert!((one_metre - 0.5).abs() < 1e-6, "one metre was {one_metre}");
        assert!(shallow < one_metre && one_metre < deep);
        assert!(deep < 1.0 && deep > 0.8, "deep was {deep}");
    }

    #[test]
    fn helpers_are_robust_to_non_finite_inputs() {
        assert_eq!(drag_factor(f32::NAN, 1.0), 0.0);
        assert_eq!(drag_factor(1.0, f32::INFINITY), 0.0);
        assert_eq!(wave_damping(f32::NAN, 1.0), 0.0);
        assert_eq!(wave_damping(1.0, f32::INFINITY), 0.0);
        assert_eq!(depth_drag(f32::NAN), 0.0);
        assert_eq!(depth_drag(f32::INFINITY), 0.0);
    }
}

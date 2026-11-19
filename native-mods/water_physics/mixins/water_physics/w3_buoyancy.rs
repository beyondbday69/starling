// w3_buoyancy.rs
// Buoyancy and wave-following response for floating entities.
//
// Coordinate system: X/Z horizontal, Y up, Minecraft blocks as metres.
// No globals, no allocation, no unsafe. All state is passed explicitly.

/// Upward buoyancy for a partially submerged volume.
///
/// `submerged` is the submerged fraction (0.0 = dry, 1.0 = fully underwater).
/// `volume` is the entity volume in cubic blocks and `gravity` is the gravity
/// magnitude. The result is a specific force assuming fluid density matches
/// the reference density used by the entity mass model.
#[inline]
pub fn buoyancy_force(submerged: f32, volume: f32, gravity: f32) -> f32 {
    if !submerged.is_finite() || !volume.is_finite() || !gravity.is_finite() {
        return 0.0;
    }
    let fraction = submerged.clamp(0.0, 1.0);
    let displaced = fraction * volume.max(0.0);
    (displaced * gravity.abs()).max(0.0)
}

/// Explicit velocity state for a damped-spring wave follower.
///
/// Keep one instance per floating entity and update it with
/// [`WaveFollowState::wave_follow`] each simulation step.
#[derive(Clone, Copy, Debug, Default)]
pub struct WaveFollowState {
    pub velocity: f32,
}

impl WaveFollowState {
    #[inline]
    pub fn new() -> Self {
        Self { velocity: 0.0 }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.velocity = 0.0;
    }

    /// Alias for [`WaveFollowState::wave_follow`] for terse call sites.
    #[inline]
    pub fn step(
        &mut self,
        y: f32,
        target: f32,
        dt: f32,
        stiffness: f32,
        damping: f32,
    ) -> f32 {
        self.wave_follow(y, target, dt, stiffness, damping)
    }

    /// Advances a unit-mass damped spring toward `target` and returns the new Y.
    ///
    /// Uses the analytical solution of `y'' = k(target - y) - c y'`, so it
    /// remains stable even with large `dt` or stiff springs.
    pub fn wave_follow(
        &mut self,
        y: f32,
        target: f32,
        dt: f32,
        stiffness: f32,
        damping: f32,
    ) -> f32 {
        if !y.is_finite() || !target.is_finite() || !dt.is_finite() || dt <= 0.0 {
            return y;
        }

        let k = if stiffness.is_finite() {
            stiffness.max(0.0)
        } else {
            0.0
        };
        let c = if damping.is_finite() {
            damping.max(0.0)
        } else {
            0.0
        };
        let v0 = if self.velocity.is_finite() {
            self.velocity
        } else {
            0.0
        };

        let (offset, velocity) = integrate_spring(y - target, v0, dt, k, c);
        self.velocity = if velocity.is_finite() { velocity } else { 0.0 };
        let next_y = target + offset;
        if next_y.is_finite() {
            next_y
        } else {
            target
        }
    }
}

/// Stateless convenience response with zero initial velocity.
///
/// For continuous entity motion, hold a [`WaveFollowState`] and call
/// [`WaveFollowState::wave_follow`] instead, so velocity is preserved.
#[inline]
pub fn wave_follow(y: f32, target: f32, dt: f32, stiffness: f32, damping: f32) -> f32 {
    WaveFollowState::new().wave_follow(y, target, dt, stiffness, damping)
}

/// Exact damped-spring step for displacement `x` relative to the target.
#[inline]
fn integrate_spring(x0: f32, v0: f32, dt: f32, k: f32, c: f32) -> (f32, f32) {
    if k <= 0.0 {
        if c <= 0.0 {
            return (x0 + v0 * dt, v0);
        }
        let decay = (-c * dt).exp();
        return (x0 + v0 * (1.0 - decay) / c, v0 * decay);
    }

    let omega = k.sqrt();
    let zeta = c / (2.0 * omega);
    let half_c = zeta * omega;

    if zeta < 1.0 {
        let wd = omega * (1.0 - zeta * zeta).sqrt();
        let phase = wd * dt;
        let cos_t = phase.cos();
        let sin_t = phase.sin();
        let b = (v0 + half_c * x0) / wd;
        let base = x0 * cos_t + b * sin_t;
        let decay = (-half_c * dt).exp();
        (
            decay * base,
            decay * (-half_c * base - x0 * wd * sin_t + b * wd * cos_t),
        )
    } else if zeta <= 1.0001 {
        let decay = (-omega * dt).exp();
        let b = v0 + omega * x0;
        let base = x0 + b * dt;
        (decay * base, decay * (b - omega * base))
    } else {
        let root = omega * (zeta * zeta - 1.0).sqrt();
        let r1 = -half_c + root;
        let r2 = -half_c - root;
        let denom = r1 - r2;
        let a = (v0 - r2 * x0) / denom;
        let b = (r1 * x0 - v0) / denom;
        let e1 = (r1 * dt).exp();
        let e2 = (r2 * dt).exp();
        (a * e1 + b * e2, a * r1 * e1 + b * r2 * e2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buoyancy_scales_with_submerged_fraction_and_volume() {
        let full = buoyancy_force(1.0, 2.0, 9.81);
        let half = buoyancy_force(0.5, 2.0, 9.81);
        assert!((full - 19.62).abs() < 1e-3, "full force was {full}");
        assert!((half - 9.81).abs() < 1e-3, "half force was {half}");
        assert_eq!(buoyancy_force(2.0, 2.0, 9.81), full);
        assert_eq!(buoyancy_force(-0.25, 2.0, 9.81), 0.0);
        assert_eq!(buoyancy_force(f32::NAN, 2.0, 9.81), 0.0);
    }

    #[test]
    fn wave_follow_state_converges_to_water_surface() {
        let mut state = WaveFollowState::new();
        let mut y = 0.0;
        for _ in 0..240 {
            y = state.wave_follow(y, 1.0, 1.0 / 60.0, 36.0, 12.0);
        }
        assert!((y - 1.0).abs() < 1e-3, "settled y was {y}");
        assert!(state.velocity.abs() < 1e-2, "velocity was {}", state.velocity);
    }

    #[test]
    fn wave_follow_state_stays_finite_for_stiff_springs() {
        let mut state = WaveFollowState::new();
        let mut y = 0.0;
        for _ in 0..30 {
            y = state.wave_follow(y, 1.0, 0.05, 400.0, 40.0);
        }
        assert!(y.is_finite(), "y diverged to {y}");
        assert!(y > -0.01 && y < 1.5, "y overshot to {y}");
        assert!(state.velocity.is_finite());
    }

    #[test]
    fn free_wave_follow_is_stateless_and_handles_zero_dt() {
        assert_eq!(wave_follow(0.5, 1.0, 0.0, 10.0, 2.0), 0.5);
        assert!(wave_follow(0.0, 1.0, 1.0 / 60.0, 25.0, 8.0) > 0.0);
    }
}

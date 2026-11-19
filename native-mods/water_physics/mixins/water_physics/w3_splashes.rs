// Physical splash event model for Water Physics v4.
//
// Converts an impact velocity and disturbing mass into deterministic droplet
// burst parameters (count, speed, spread, ring amplitude) and per-droplet
// launch velocities. Pure math, allocation-free, mobile friendly.

/// Bounded splash burst parameters derived from one impact event.
#[derive(Clone, Copy, Debug)]
pub struct SplashParams {
    /// Number of droplets to emit (bounded 4..=32).
    pub count: usize,
    /// Initial outward/upward speed in blocks per second (bounded).
    pub speed: f32,
    /// Azimuthal spread in radians (bounded 0..=TAU).
    pub spread: f32,
    /// Amplitude passed to the geometric ripple ring (bounded).
    pub ring_amp: f32,
}

pub const TAU: f32 = std::f32::consts::PI * 2.0;
pub const MIN_COUNT: usize = 4;
pub const MAX_COUNT: usize = 32;
pub const MIN_SPEED: f32 = 0.8;
pub const MAX_SPEED: f32 = 6.0;
pub const MIN_RING_AMP: f32 = 0.30;
pub const MAX_RING_AMP: f32 = 1.60;

#[inline]
fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

#[inline]
fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    finite_or(v, lo).max(lo).min(hi)
}

/// Build splash parameters from an impact speed (blocks/s, positive down) and
/// a disturbing mass estimate in arbitrary but stable units (0.25..=8 typical).
#[inline]
pub fn from_impact(velocity: f32, mass: f32) -> SplashParams {
    let v = clamp(velocity, 0.0, 24.0);
    let m = clamp(mass, 0.1, 16.0);

    // Energy-ish scaling with diminishing returns.
    let energy = (v * 0.08 + m.sqrt() * 0.35).clamp(0.0, 4.0);
    let norm = (energy / 4.0).clamp(0.0, 1.0);

    let count_f = MIN_COUNT as f32 + norm * (MAX_COUNT - MIN_COUNT) as f32;
    let count = count_f.round() as usize;
    let count = count.clamp(MIN_COUNT, MAX_COUNT);

    let speed = (MIN_SPEED + norm * (MAX_SPEED - MIN_SPEED)).clamp(MIN_SPEED, MAX_SPEED);
    let spread = (0.6 + norm * (TAU - 0.6)).clamp(0.0, TAU);
    let ring_amp = (MIN_RING_AMP + norm * (MAX_RING_AMP - MIN_RING_AMP))
        .clamp(MIN_RING_AMP, MAX_RING_AMP);

    SplashParams { count, speed, spread, ring_amp }
}

/// Deterministic launch velocity for droplet `i` of `count` in a burst.
/// Returns [vx, vy, vz] in blocks/s. Index 0 is straight up; others fan out.
#[inline]
pub fn droplet_velocity(i: usize, count: usize, p: &SplashParams) -> [f32; 3] {
    if count == 0 || p.count == 0 {
        return [0.0, 0.0, 0.0];
    }
    let idx = i.min(p.count.saturating_sub(1));
    let n = count.max(1);

    if idx == 0 {
        return [0.0, p.speed, 0.0];
    }

    let t = idx as f32 / n as f32;
    let angle = t * p.spread + (idx as f32 * 2.3999632); // golden-angle jitter
    let radial = p.speed * (0.35 + 0.65 * (1.0 - t));
    let up = p.speed * (0.55 + 0.45 * (1.0 - t));

    [angle.cos() * radial, up, angle.sin() * radial]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_impact_is_bounded_and_monotonic() {
        let a = from_impact(0.0, 0.1);
        let b = from_impact(8.0, 1.0);
        let c = from_impact(24.0, 16.0);

        assert!(a.count >= MIN_COUNT && a.count <= MAX_COUNT);
        assert!(c.count <= MAX_COUNT);
        assert!(a.speed >= MIN_SPEED && a.speed <= MAX_SPEED);
        assert!(c.speed <= MAX_SPEED);
        assert!(a.ring_amp <= b.ring_amp);
        assert!(b.ring_amp <= c.ring_amp);
        assert!(a.spread >= 0.0 && c.spread <= TAU + 0.0001);
    }

    #[test]
    fn from_impact_handles_non_finite() {
        let p = from_impact(f32::NAN, f32::INFINITY);
        assert!(p.count >= MIN_COUNT && p.count <= MAX_COUNT);
        assert!(p.speed.is_finite());
        assert!(p.ring_amp.is_finite());
    }

    #[test]
    fn droplet_velocity_is_deterministic_and_finite() {
        let p = from_impact(10.0, 2.0);
        let v1 = droplet_velocity(3, p.count, &p);
        let v2 = droplet_velocity(3, p.count, &p);
        assert_eq!(v1, v2);
        for c in v1 {
            assert!(c.is_finite());
        }
    }

    #[test]
    fn first_droplet_goes_straight_up() {
        let p = from_impact(5.0, 1.0);
        let v = droplet_velocity(0, p.count, &p);
        assert_eq!(v[0], 0.0);
        assert_eq!(v[2], 0.0);
        assert!(v[1] > 0.0);
    }
}

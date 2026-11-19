//! Procedural foam mask generation for water v4.
//!
//! Foam is derived from four caller-supplied, normalized factors:
//! * `steepness`  - wave steepness; high values indicate breaking crests.
//! * `shore`      - shoreline factor (`0` in deep water, `1` at the shore).
//! * `turbulence` - local churn from currents, wakes or collisions.
//! * `noise`      - spatial noise used to break up flat foam edges.
//!
//! The factors are combined with smooth thresholds and a soft union so that
//! overlapping foam sources saturate gracefully instead of producing hard
//! seams. Every function is pure, deterministic, allocation-free and safe to
//! call per vertex. Coordinates are world-space X/Z with Y up; all inputs are
//! expected in `0..=1` but are clamped (non-finite values become `0`).

/// Wave steepness where breaking foam starts to appear.
const STEEP_START: f32 = 0.42;
/// Wave steepness where breaking foam reaches full strength.
const STEEP_FULL: f32 = 0.92;
/// Shore factor where surf foam starts to appear.
const SHORE_START: f32 = 0.30;
/// Shore factor where surf foam reaches full strength.
const SHORE_FULL: f32 = 0.90;
/// Turbulence where churn foam starts to appear.
const TURB_START: f32 = 0.25;
/// Turbulence where churn foam reaches full strength.
const TURB_FULL: f32 = 0.95;

/// Foam base colour as `0x00RRGGBB`: a near-white with a faint cool cast.
const FOAM_RGB: u32 = 0x00_F2_F7_FF;

/// Clamp to `0.0..=1.0`, mapping non-finite inputs to zero.
#[inline]
fn sat(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Hermite smoothstep over an arbitrary `[edge0, edge1]` interval.
#[inline]
fn smoothstep_range(x: f32, edge0: f32, edge1: f32) -> f32 {
    let t = sat((x - edge0) / (edge1 - edge0));
    t * t * (3.0 - 2.0 * t)
}

/// Foam coverage in `0.0..=1.0` from wave, shore, turbulence and noise factors.
///
/// The three foam sources are detected with smooth thresholds, modulated by
/// noise, then merged with a soft union (`1 - prod(1 - source)`). Coverage is
/// monotone in every input, so increasing any factor can never reduce foam.
#[inline]
pub fn foam_amount(steepness: f32, shore: f32, turbulence: f32, noise: f32) -> f32 {
    let s = sat(steepness);
    let sh = sat(shore);
    let t = sat(turbulence);
    let n = sat(noise);

    // Breaking crests dominate; shore surf spreads more evenly; turbulence
    // alone contributes less unless noise breaks it into visible patches.
    let breaking = smoothstep_range(s, STEEP_START, STEEP_FULL) * (0.55 + 0.45 * n);
    let surf = smoothstep_range(sh, SHORE_START, SHORE_FULL) * (0.70 + 0.30 * n);
    let churn = smoothstep_range(t, TURB_START, TURB_FULL) * (0.35 + 0.65 * n);

    let remaining = (1.0 - breaking.min(1.0)) * (1.0 - surf.min(1.0)) * (1.0 - churn.min(1.0));
    sat(1.0 - remaining)
}

/// Pack a foam colour with the given alpha in the `ABGR` u32 layout used by
/// `horizon_client::mesher::Vertex::color`.
///
/// The foam tint is constant (`0xF2F7FF` in RGB), so only opacity varies.
#[inline]
pub fn foam_color(alpha: u8) -> u32 {
    let r = FOAM_RGB & 0xFF;
    let g = (FOAM_RGB >> 8) & 0xFF;
    let b = (FOAM_RGB >> 16) & 0xFF;
    ((alpha as u32) << 24) | (b << 16) | (g << 8) | r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_inputs_are_dry_and_max_inputs_are_full_foam() {
        assert_eq!(foam_amount(0.0, 0.0, 0.0, 0.0), 0.0);
        assert_eq!(foam_amount(1.0, 1.0, 1.0, 1.0), 1.0);

        // A single fully active source still reaches exactly one.
        assert_eq!(foam_amount(1.0, 0.0, 0.0, 1.0), 1.0);
        assert_eq!(foam_amount(0.0, 1.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn monotone_bounded_and_deterministic() {
        // Sample a coarse grid and verify clamping, finiteness and monotone
        // response to each factor independently.
        let values = [0.0f32, 0.25, 0.5, 0.75, 1.0];
        for &s in &values {
            for &sh in &values {
                for &t in &values {
                    for &n in &values {
                        let a = foam_amount(s, sh, t, n);
                        assert!(a.is_finite());
                        assert!((0.0..=1.0).contains(&a), "out of range: {a}");
                        assert_eq!(a, foam_amount(s, sh, t, n));
                        assert!(foam_amount(s + 0.1, sh, t, n) >= a);
                        assert!(foam_amount(s, sh + 0.1, t, n) >= a);
                        assert!(foam_amount(s, sh, t + 0.1, n) >= a);
                        assert!(foam_amount(s, sh, t, n + 0.1) >= a);
                    }
                }
            }
        }

        // Out-of-range and non-finite inputs are clamped safely.
        let base = foam_amount(0.5, 0.5, 0.5, 0.5);
        assert_eq!(foam_amount(-2.0, -2.0, -2.0, -2.0), 0.0);
        assert_eq!(foam_amount(9.0, 9.0, 9.0, 9.0), 1.0);
        assert_eq!(foam_amount(f32::NAN, f32::NAN, f32::NAN, f32::NAN), 0.0);
        assert!(base > 0.0 && base < 1.0);
    }

    #[test]
    fn thresholds_are_smooth_not_stepped() {
        let eps = 1.0e-3f32;
        let mut max_jump = 0.0f32;
        for i in 0..1000 {
            let x = i as f32 * 0.001;
            // Cross each threshold region from both sides.
            for (s, sh, t) in [
                (x, 0.0, 0.0),
                (0.0, x, 0.0),
                (0.0, 0.0, x),
                (x, x, x),
            ] {
                let a = foam_amount(s, sh, t, 0.5);
                let b = foam_amount(s + eps, sh + eps, t + eps, 0.5);
                max_jump = max_jump.max((a - b).abs());
            }
        }
        assert!(max_jump < 0.01, "foam mask discontinuity: {max_jump}");
    }

    #[test]
    fn foam_color_packs_abgr_bytes() {
        assert_eq!(foam_color(0), 0x00_FF_F7_F2);
        assert_eq!(foam_color(255), 0xFF_FF_F7_F2);
        assert_eq!(foam_color(128), 0x80_FF_F7_F2);

        let packed = foam_color(200);
        assert_eq!((packed >> 24) as u8, 200); // A
        assert_eq!((packed >> 16) as u8, 0xFF); // B
        assert_eq!((packed >> 8) as u8, 0xF7); // G
        assert_eq!((packed & 0xFF) as u8, 0xF2); // R
    }
}

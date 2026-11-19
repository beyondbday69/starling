// w3_ripple3d.rs
// Analytic 3D ripple-ring height and surface normals sampled at ANY XZ point.
//
// This module is deliberately independent of `sim.rs`. `sim::RippleRing` (or any
// other ring representation) is adapted through the small [`RingLike`] trait, so
// the root integration can simply add:
//
//   impl RingLike for sim::RippleRing {
//       fn center_x(&self) -> f64 { self.center_x }
//       fn center_z(&self) -> f64 { self.center_z }
//       fn radius(&self) -> f32 { self.radius }
//       fn amplitude(&self) -> f32 { self.amplitude }
//       fn envelope(&self) -> f32 { self.envelope() }
//   }
//
// Alternatively, a caller can fill in the plain-data [`RingDesc`] adapter.
//
// Coordinate system: X/Z horizontal, Y up, Minecraft blocks (1 block = 1 m).
// The height returned by [`ring_height`] is a displacement above the flat water
// surface, not an absolute world Y. No globals, no allocation, no unsafe.

/// Peak crest displacement per unit of effective amplitude (matches render.rs).
pub const CREST_SCALE: f32 = 0.045;
/// Hard cap on crest displacement in blocks (matches render.rs).
pub const CREST_MAX: f32 = 0.060;
/// Radial half-width base of the ripple band.
pub const RING_WIDTH_BASE: f32 = 0.14;
/// Radial half-width growth per block of ring radius.
pub const RING_WIDTH_PER_RADIUS: f32 = 0.16;
pub const RING_WIDTH_MIN: f32 = 0.18;
pub const RING_WIDTH_MAX: f32 = 0.48;

/// Minimal adapter between this module and any simulation ring type.
///
/// The trait is object safe and uses only `&self`, so callers may use static
/// dispatch (`&ConcreteRing`) or a `&dyn RingLike` interchangeably.
pub trait RingLike {
    fn center_x(&self) -> f64;
    fn center_z(&self) -> f64;
    fn radius(&self) -> f32;
    fn amplitude(&self) -> f32;
    /// Lifetime envelope in `[0, 1]`, as provided by `sim::RippleRing::envelope`.
    fn envelope(&self) -> f32;
}

/// Plain-data [`RingLike`] implementation for callers that do not want to add a
/// trait impl to their simulation type. Copying these five scalars is cheap.
#[derive(Clone, Copy, Debug, Default)]
pub struct RingDesc {
    pub center_x: f64,
    pub center_z: f64,
    pub radius: f32,
    pub amplitude: f32,
    pub envelope: f32,
}

impl RingLike for RingDesc {
    #[inline]
    fn center_x(&self) -> f64 {
        self.center_x
    }
    #[inline]
    fn center_z(&self) -> f64 {
        self.center_z
    }
    #[inline]
    fn radius(&self) -> f32 {
        self.radius
    }
    #[inline]
    fn amplitude(&self) -> f32 {
        self.amplitude
    }
    #[inline]
    fn envelope(&self) -> f32 {
        self.envelope
    }
}

/// Radial half-width of the ripple band for a given ring radius.
/// Matches the geometry used by the existing overlay renderer.
#[inline]
pub fn ring_width(radius: f32) -> f32 {
    if !radius.is_finite() {
        return RING_WIDTH_MIN;
    }
    (RING_WIDTH_BASE + RING_WIDTH_PER_RADIUS * radius.max(0.0)).clamp(RING_WIDTH_MIN, RING_WIDTH_MAX)
}

/// Effective (envelope-scaled) crest height for a ring.
#[inline]
pub fn ring_crest_height(amplitude: f32, envelope: f32) -> f32 {
    let amp = (amplitude * envelope).max(0.0);
    if !amp.is_finite() {
        return 0.0;
    }
    (CREST_SCALE * amp).min(CREST_MAX)
}

/// Vertical displacement of the ring surface at world position `(x, z)`.
///
/// Returns a height ABOVE the flat water plane. Zero outside the smooth band
/// around the ring radius and while the envelope is zero.
#[inline]
pub fn ring_height<R: RingLike + ?Sized>(ring: &R, x: f64, z: f64) -> f32 {
    sample(ring, x, z).0
}

/// Analytic slope of the height field: `(dH/dx, dH/dz)` in blocks per block.
#[inline]
pub fn ring_slope<R: RingLike + ?Sized>(ring: &R, x: f64, z: f64) -> (f32, f32) {
    let (_, sx, sz) = sample(ring, x, z);
    (sx, sz)
}

/// Height and analytic slope in a single pass.
#[inline]
pub fn ring_height_and_slope<R: RingLike + ?Sized>(ring: &R, x: f64, z: f64) -> (f32, f32, f32) {
    sample(ring, x, z)
}

/// Upward unit surface normal of the ripple height field at `(x, z)`.
///
/// For `y = H(x, z)` the normal is `normalize([-dH/dx, 1, -dH/dz])`.
/// Flat regions return exactly `[0, 1, 0]`.
#[inline]
pub fn ring_normal<R: RingLike + ?Sized>(ring: &R, x: f64, z: f64) -> [f32; 3] {
    let (_, sx, sz) = sample(ring, x, z);
    let inv_len = 1.0 / (1.0 + sx * sx + sz * sz).sqrt();
    [-sx * inv_len, inv_len, -sz * inv_len]
}

/// Core sampler. The profile is a raised cosine over `radius +/- width`:
///
///   H(d) = crest * 0.5 * (1 + cos(pi * (d - radius) / width))
///
/// It has zero value AND zero radial slope at both band edges, and value `crest`
/// with zero slope at `d == radius`. The derivative is analytically differentiable
/// everywhere except exactly at the ring centre, where the radial direction is
/// undefined and a zero slope is returned.
#[inline]
fn sample<R: RingLike + ?Sized>(ring: &R, x: f64, z: f64) -> (f32, f32, f32) {
    let (amp, radius) = (ring.amplitude(), ring.radius());
    let crest = ring_crest_height(amp, ring.envelope());
    if crest <= 0.0 || !radius.is_finite() || radius < 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let w = ring_width(radius);
    if w <= 0.0 {
        return (0.0, 0.0, 0.0);
    }

    let (cx, cz) = (ring.center_x(), ring.center_z());
    if !cx.is_finite() || !cz.is_finite() || !x.is_finite() || !z.is_finite() {
        return (0.0, 0.0, 0.0);
    }

    let dx = x - cx;
    let dz = z - cz;
    let d2 = dx * dx + dz * dz;
    let outer = (radius + w) as f64;
    if d2 > outer * outer {
        return (0.0, 0.0, 0.0);
    }

    // At the exact centre there is no radial direction. The profile is
    // nevertheless well defined; return its value with zero slope.
    if d2 <= 1.0e-18 {
        let u = -radius / w;
        if u <= -1.0 || u >= 1.0 {
            return (0.0, 0.0, 0.0);
        }
        let h = crest * 0.5 * (1.0 + (std::f32::consts::PI * u).cos());
        return (h, 0.0, 0.0);
    }

    let d = d2.sqrt();
    let u = ((d as f32) - radius) / w;
    if u <= -1.0 || u >= 1.0 {
        return (0.0, 0.0, 0.0);
    }

    let pi_u = std::f32::consts::PI * u;
    let height = crest * 0.5 * (1.0 + pi_u.cos());
    // dH/dd = crest * -0.5*pi*sin(pi*u)/w
    let dh_dd = crest * (-0.5 * std::f32::consts::PI * pi_u.sin() / w);
    let inv_d = 1.0 / d;
    let slope_x = (dh_dd as f64 * dx * inv_d) as f32;
    let slope_z = (dh_dd as f64 * dz * inv_d) as f32;
    (height, slope_x, slope_z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct TestRing {
        center_x: f64,
        center_z: f64,
        radius: f32,
        amplitude: f32,
        envelope: f32,
    }

    impl RingLike for TestRing {
        fn center_x(&self) -> f64 {
            self.center_x
        }
        fn center_z(&self) -> f64 {
            self.center_z
        }
        fn radius(&self) -> f32 {
            self.radius
        }
        fn amplitude(&self) -> f32 {
            self.amplitude
        }
        fn envelope(&self) -> f32 {
            self.envelope
        }
    }

    fn ring() -> TestRing {
        TestRing {
            center_x: 12.5,
            center_z: -7.25,
            radius: 2.0,
            amplitude: 1.0,
            envelope: 1.0,
        }
    }

    #[test]
    fn crest_and_zero_support_match_render_geometry() {
        let r = ring();
        let w = ring_width(r.radius);
        assert!((w - 0.46).abs() < 1e-6, "width was {}", w);

        // Peak on the crest circle.
        let h = ring_height(&r, r.center_x + r.radius as f64, r.center_z);
        assert!((h - CREST_SCALE).abs() < 1e-6, "crest was {}", h);

        // Flat inside the inner edge, outside the outer edge, and at the centre.
        let inner = r.radius - w;
        let outer = r.radius + w;
        assert_eq!(ring_height(&r, r.center_x + inner as f64 - 0.01, r.center_z), 0.0);
        assert_eq!(ring_height(&r, r.center_x + outer as f64 + 0.01, r.center_z), 0.0);
        assert_eq!(ring_height(&r, r.center_x, r.center_z), 0.0);

        // The band edges themselves have zero height and zero slope.
        assert!(ring_height(&r, r.center_x + inner as f64, r.center_z).abs() < 1e-6);
        assert!(ring_height(&r, r.center_x + outer as f64, r.center_z).abs() < 1e-6);
        let (sx_edge, sz_edge) = ring_slope(&r, r.center_x + inner as f64, r.center_z);
        assert!(sx_edge.abs() < 1e-6 && sz_edge.abs() < 1e-6);
    }

    #[test]
    fn normals_are_unit_length_and_point_uphill_correctly() {
        let r = ring();
        let w = ring_width(r.radius);

        let flat = ring_normal(&r, r.center_x + 8.0, r.center_z);
        assert_eq!(flat, [0.0, 1.0, 0.0]);

        // Inner flank: height increases outward, so the normal tilts inward.
        let d_in = r.radius - 0.5 * w;
        let n_in = ring_normal(&r, r.center_x + d_in as f64, r.center_z);
        assert!(n_in[0] < 0.0, "inner flank normal x = {}", n_in[0]);
        assert!(n_in[1] > 0.9 && n_in[1] <= 1.0, "inner flank normal y = {}", n_in[1]);

        // Outer flank: height decreases outward, so the normal tilts outward.
        let d_out = r.radius + 0.5 * w;
        let n_out = ring_normal(&r, r.center_x + d_out as f64, r.center_z);
        assert!(n_out[0] > 0.0, "outer flank normal x = {}", n_out[0]);
        assert!(n_out[1] > 0.9 && n_out[1] <= 1.0, "outer flank normal y = {}", n_out[1]);

        for &n in [n_in, n_out].iter() {
            let len2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
            assert!((len2 - 1.0).abs() < 1e-5, "normal length^2 = {}", len2);
        }
    }

    #[test]
    fn analytic_slope_matches_central_difference() {
        let r = ring();
        let d = r.radius + 0.13; // a point on the outer flank, away from band edges
        let x = r.center_x + d as f64 * 0.8;
        let z = r.center_z + d as f64 * 0.6;
        let eps = 1.0e-3f64;

        let (sx, sz) = ring_slope(&r, x, z);
        let nx = (ring_height(&r, x + eps, z) - ring_height(&r, x - eps, z)) as f64 / (2.0 * eps);
        let nz = (ring_height(&r, x, z + eps) - ring_height(&r, x, z - eps)) as f64 / (2.0 * eps);

        assert!((sx as f64 - nx).abs() < 2.0e-3, "dH/dx: analytic {} vs numeric {}", sx, nx);
        assert!((sz as f64 - nz).abs() < 2.0e-3, "dH/dz: analytic {} vs numeric {}", sz, nz);
    }

    #[test]
    fn envelope_and_adapter_behaviour() {
        let mut r = ring();
        r.envelope = 0.0; // spawning or expired ring: fully flat
        assert_eq!(ring_height(&r, r.center_x + r.radius as f64, r.center_z), 0.0);
        assert_eq!(ring_normal(&r, r.center_x + r.radius as f64, r.center_z), [0.0, 1.0, 0.0]);

        r.envelope = 0.5;
        let half = ring_height(&r, r.center_x + r.radius as f64, r.center_z);
        assert!((half - 0.5 * CREST_SCALE).abs() < 1e-6, "half-envelope crest = {}", half);

        // The plain-data adapter and a trait object both work with the same API.
        let desc = RingDesc {
            center_x: r.center_x,
            center_z: r.center_z,
            radius: r.radius,
            amplitude: r.amplitude,
            envelope: r.envelope,
        };
        let via_desc = ring_height(&desc, r.center_x + r.radius as f64, r.center_z);
        assert!((via_desc - half).abs() < 1e-6);
        let dyn_ref: &dyn RingLike = &desc;
        let via_dyn = ring_height(dyn_ref, r.center_x + r.radius as f64, r.center_z);
        assert!((via_dyn - half).abs() < 1e-6);
    }
}

// w3_shore.rs - Shoreline proximity, wave shoaling and wet-edge model (water v4).
//
// Depth is used as a cheap proxy for distance to the shoreline: a gently
// sloping bed means the water gets shallower as the coast approaches. This
// module never touches the world; callers pass the water depth in blocks
// (`water_surface_y - bed_y`) and, optionally, a wavelength in blocks.
//
// Public API:
//   ShoreSampler::shore_factor(depth)               -> f32 in 0..1
//       0 in deep water, 1 at / above the waterline. Exponential falloff with
//       a configurable depth scale (default 1.5 blocks).
//   ShoreSampler::shoal_wave_height(depth, wavelength) -> f32 in 0..max_gain
//       Height multiplier for a unit deep-water wave. Deep water (depth >=
//       wavelength/2) returns 1.0. As waves enter shallow water Green's law
//       (h^-1/4) amplifies them up to `max_shoal_gain`; at the very edge the
//       wave breaks and the multiplier tapers to 0.
//   ShoreSampler::wet_edge_alpha(depth)             -> f32 in 0..1
//       0 in deep water, ramps to 1 within `wet_edge_depth` of the waterline.
//
// Module-level convenience functions with the same names use a default
// sampler, so callers can choose either `sampler.shoal_wave_height(..)` or
// `w3_shore::shoal_wave_height(..)`.
//
// Contract: X/Z horizontal, Y up, 1 block = 1 m. No globals, no unsafe, no
// heap allocation, no external crates, O(1) per sample.

/// Smallest positive value used as a denominator guard.
const EPS: f32 = 1.0e-4;

/// Treat NaN as "deep / dry" (no shoreline influence), keep infinities intact.
#[inline]
fn sanitize_depth(depth: f32) -> f32 {
    if depth.is_nan() {
        f32::INFINITY
    } else {
        depth.max(0.0)
    }
}

#[inline]
fn clamp01(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

#[inline]
fn smoothstep01(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Tunable, allocation-free sampler for shoreline effects.
///
/// Construct with `ShoreSampler::new()` / `ShoreSampler::default()` and adjust
/// the public fields if a particular water body needs different scales.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShoreSampler {
    /// Depth (blocks) over which `shore_factor` decays significantly.
    pub shore_depth_scale: f32,
    /// Upper bound of the shoaling amplification (Green's law is capped here).
    pub max_shoal_gain: f32,
    /// Depth (blocks) below which waves are considered broken / dissipated.
    pub break_depth: f32,
    /// Depth (blocks) over which the wet edge fades from 1 to 0.
    pub wet_edge_depth: f32,
}

impl ShoreSampler {
    pub const DEFAULT_SHORE_DEPTH_SCALE: f32 = 1.5;
    pub const DEFAULT_MAX_SHOAL_GAIN: f32 = 2.2;
    pub const DEFAULT_BREAK_DEPTH: f32 = 0.15;
    pub const DEFAULT_WET_EDGE_DEPTH: f32 = 0.5;

    #[inline]
    pub fn new() -> Self {
        Self {
            shore_depth_scale: Self::DEFAULT_SHORE_DEPTH_SCALE,
            max_shoal_gain: Self::DEFAULT_MAX_SHOAL_GAIN,
            break_depth: Self::DEFAULT_BREAK_DEPTH,
            wet_edge_depth: Self::DEFAULT_WET_EDGE_DEPTH,
        }
    }

    /// 0 in deep water, approaching 1 at the waterline.
    ///
    /// `depth <= 0` (dry / exactly at the edge) yields 1; NaN yields 0 so a
    /// bad sample cannot create a spurious shoreline.
    #[inline]
    pub fn shore_factor(&self, depth: f32) -> f32 {
        let d = sanitize_depth(depth);
        let scale = self.shore_depth_scale.max(EPS);
        clamp01((-d / scale).exp())
    }

    /// Shoaled height multiplier for a unit deep-water wave.
    ///
    /// Waves only feel the bed when the depth is below roughly half a
    /// wavelength. There, Green's law amplifies the height by `(h_ref / h)^0.25`
    /// with `h_ref = wavelength * 0.5`, capped at `max_shoal_gain`. Within
    /// `break_depth` of the waterline the wave breaks and the multiplier fades
    /// smoothly to 0. Deep water, NaN depths and NaN wavelengths return 1.0.
    #[inline]
    pub fn shoal_wave_height(&self, depth: f32, wavelength: f32) -> f32 {
        let d = sanitize_depth(depth);
        let lambda = if wavelength.is_nan() {
            0.0
        } else {
            wavelength.clamp(EPS, 128.0)
        };
        let reference = lambda * 0.5;
        if d >= reference {
            // Deep water (or NaN depth/wavelength): no bottom influence.
            return 1.0;
        }

        let max_gain = if self.max_shoal_gain.is_finite() {
            self.max_shoal_gain.max(1.0)
        } else {
            1.0
        };

        let green = (reference / d.max(EPS)).powf(0.25);
        let gain = green.min(max_gain);
        let taper = smoothstep01(d / self.break_depth.max(EPS));
        clamp01(gain * taper / max_gain) * max_gain
    }

    /// Alpha of the wet / foam edge: 0 in deep water, 1 near the waterline.
    ///
    /// NaN and infinite depths return 0; depths at or below 0 return 1.
    #[inline]
    pub fn wet_edge_alpha(&self, depth: f32) -> f32 {
        let d = sanitize_depth(depth);
        let edge = self.wet_edge_depth.max(EPS);
        smoothstep01(1.0 - d / edge)
    }

    /// Estimate horizontal distance to the shoreline (blocks) from depth and
    /// an assumed linear bed slope (`depth / slope`). Non-finite or negative
    /// slopes fall back to `1.0`; NaN depth yields 0, infinite depth is capped.
    #[inline]
    pub fn distance_to_shore(&self, depth: f32, bed_slope: f32) -> f32 {
        if depth.is_nan() {
            return 0.0;
        }
        let d = depth.max(0.0);
        let slope = if bed_slope.is_finite() && bed_slope > 0.0 {
            bed_slope
        } else {
            1.0
        };
        (d / slope).min(1.0e6)
    }
}

impl Default for ShoreSampler {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn shore_factor(depth: f32) -> f32 {
    ShoreSampler::default().shore_factor(depth)
}

#[inline]
pub fn shoal_wave_height(depth: f32, wavelength: f32) -> f32 {
    ShoreSampler::default().shoal_wave_height(depth, wavelength)
}

#[inline]
pub fn wet_edge_alpha(depth: f32) -> f32 {
    ShoreSampler::default().wet_edge_alpha(depth)
}

#[inline]
pub fn distance_to_shore(depth: f32, bed_slope: f32) -> f32 {
    ShoreSampler::default().distance_to_shore(depth, bed_slope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shore_factor_is_monotonic_and_clamped() {
        let sampler = ShoreSampler::new();
        let mut previous = f32::INFINITY;
        for i in 0..=400 {
            let depth = -2.0 + i as f32 * 0.05;
            let f = sampler.shore_factor(depth);
            assert!(f.is_finite(), "non-finite shore factor at {depth}");
            assert!((0.0..=1.0).contains(&f), "out of range: {f}");
            assert!(f <= previous + 1.0e-6, "not monotonic at depth {depth}");
            previous = f;
        }
        assert_eq!(sampler.shore_factor(-1.0), 1.0);
        assert_eq!(sampler.shore_factor(0.0), 1.0);
        assert!(sampler.shore_factor(1.0) < sampler.shore_factor(0.5));
        assert!(sampler.shore_factor(20.0) < 0.001);
        assert_eq!(sampler.shore_factor(f32::INFINITY), 0.0);
        assert_eq!(sampler.shore_factor(f32::NAN), 0.0);
    }

    #[test]
    fn shoaling_amplifies_then_breaks_and_clamps() {
        let sampler = ShoreSampler::new();
        let wavelength = 16.0;

        assert_eq!(sampler.shoal_wave_height(9.0, wavelength), 1.0);
        assert_eq!(sampler.shoal_wave_height(8.0, wavelength), 1.0);

        let shallow = sampler.shoal_wave_height(1.0, wavelength);
        assert!(shallow > 1.0, "expected shoaling amplification, got {shallow}");
        assert!(shallow <= sampler.max_shoal_gain);

        // Past the break depth the gain decays monotonically with depth as the
        // bed influence fades, until it reaches 1.0 in deep water.
        let mut previous = f32::INFINITY;
        let mut d = sampler.break_depth;
        while d < 8.0 {
            let h = sampler.shoal_wave_height(d, wavelength);
            assert!(h.is_finite());
            assert!(h <= previous + 1.0e-6, "gain rose at depth {d}");
            assert!(h <= sampler.max_shoal_gain + 1.0e-6);
            previous = h;
            d += 0.05;
        }

        assert_eq!(sampler.shoal_wave_height(0.0, wavelength), 0.0);
        assert_eq!(sampler.shoal_wave_height(-1.0, wavelength), 0.0);
        assert_eq!(sampler.shoal_wave_height(f32::NAN, wavelength), 1.0);
        assert_eq!(sampler.shoal_wave_height(1.0, f32::NAN), 1.0);
        assert_eq!(sampler.shoal_wave_height(0.0, 0.0), 0.0);
    }

    #[test]
    fn wet_edge_alpha_is_monotonic_and_clamped() {
        let sampler = ShoreSampler::new();
        let mut previous = f32::INFINITY;
        for i in 0..=200 {
            let depth = -1.0 + i as f32 * 0.01;
            let a = sampler.wet_edge_alpha(depth);
            assert!(a.is_finite());
            assert!((0.0..=1.0).contains(&a), "out of range: {a}");
            assert!(a <= previous + 1.0e-6, "not monotonic at depth {depth}");
            previous = a;
        }
        assert_eq!(sampler.wet_edge_alpha(-0.5), 1.0);
        assert_eq!(sampler.wet_edge_alpha(0.0), 1.0);
        assert_eq!(sampler.wet_edge_alpha(0.5), 0.0);
        assert_eq!(sampler.wet_edge_alpha(10.0), 0.0);
        assert_eq!(sampler.wet_edge_alpha(f32::NAN), 0.0);
        assert_eq!(sampler.wet_edge_alpha(f32::INFINITY), 0.0);
    }

    #[test]
    fn extreme_inputs_stay_finite_and_free_functions_match() {
        let sampler = ShoreSampler::new();
        let odd_depths = [f32::MIN, -1.0e20, 0.0, 1.0e-8, 1.0e8, f32::MAX, f32::NAN, f32::INFINITY];
        let odd_waves = [0.0, -1.0, 1.0e-8, 1.0e8, f32::MAX, f32::NAN, f32::INFINITY];
        for depth in odd_depths {
            for wave in odd_waves {
                let f = sampler.shore_factor(depth);
                let h = sampler.shoal_wave_height(depth, wave);
                let a = sampler.wet_edge_alpha(depth);
                assert!(f.is_finite() && h.is_finite() && a.is_finite());
                assert!((0.0..=1.0).contains(&f));
                assert!((0.0..=sampler.max_shoal_gain).contains(&h));
                assert!((0.0..=1.0).contains(&a));
            }
        }

        assert_eq!(shore_factor(0.5), sampler.shore_factor(0.5));
        assert_eq!(shoal_wave_height(0.5, 8.0), sampler.shoal_wave_height(0.5, 8.0));
        assert_eq!(wet_edge_alpha(0.5), sampler.wet_edge_alpha(0.5));
        assert_eq!(distance_to_shore(2.0, 0.25), 8.0);
        assert_eq!(distance_to_shore(2.0, 0.0), 2.0);
    }
}

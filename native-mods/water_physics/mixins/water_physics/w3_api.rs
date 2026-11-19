// w3_api.rs
//
// WaterV4 facade: a single, self-contained entry point for the v4 water
// surface. This module deliberately does NOT depend on the richer w3_gerstner,
// w3_ripple3d, w3_wake3d or w3_normals modules. Root wires those in later.
// Keeping the facade standalone means it compiles and tests on its own.
//
// Public API:
//   * WaterV4::new(quality)          - build a facade from a QualitySettings.
//   * step(dt, cam)                  - advance time and age active disturbances.
//   * surface_height(x, z)           - displacement above the flat water plane.
//   * surface_normal(x, z)           - unit, upward-pointing surface normal.
//   * push_ripple(x, z, amplitude)   - inject one expanding circular ripple.
//   * push_wake(x, z, dir_x, dir_z, speed, is_boat) - inject one wake node.
//
// Coordinate system: X/Z horizontal, Y up, Minecraft blocks as metres.
// No crates, no globals, no unsafe, no heap allocation in hot paths.

#[cfg(not(test))]
use super::native_water_physics_quality::QualitySettings;

// Test-only stand-in so the facade's unit tests run without the quality module
// being present. Production builds always use the real QualitySettings above.
#[cfg(test)]
mod test_quality {
    #[derive(Clone, Copy, Debug)]
    pub struct QualitySettings;
    impl QualitySettings {
        #[inline]
        pub fn from_name(_name: &str) -> Self {
            Self
        }
    }
}
#[cfg(test)]
use self::test_quality::QualitySettings;

/// Maximum simultaneously live ripples (fixed storage, no allocation).
pub const MAX_RIPPLES: usize = 24;
/// Maximum simultaneously live wake nodes.
pub const MAX_WAKES: usize = 16;
/// Central-difference step used by [`WaterV4::surface_normal`].
pub const NORMAL_EPS: f32 = 0.02;
/// Largest accepted integration step; larger frames are clamped for stability.
pub const MAX_DT: f32 = 0.1;

/// One expanding circular disturbance.
#[derive(Clone, Copy, Debug)]
struct Ripple {
    x: f32,
    z: f32,
    amplitude: f32,
    age: f32,
    life: f32,
    speed: f32,
    width: f32,
    active: bool,
}

/// One trailing wake disturbance.
#[derive(Clone, Copy, Debug)]
struct Wake {
    x: f32,
    z: f32,
    dir_x: f32,
    dir_z: f32,
    amplitude: f32,
    half_width: f32,
    spread: f32,
    age: f32,
    life: f32,
    active: bool,
}

/// Directional ambient wave components: (dir_x, dir_z, frequency, amplitude,
/// phase speed, phase offset). Directions are normalised to within rounding.
const AMBIENT_WAVES: [(f32, f32, f32, f32, f32, f32); 4] = [
    (1.0000, 0.0000, 0.85, 0.0350, 1.6, 0.0),
    (0.7071, 0.7071, 1.30, 0.0220, 2.1, 1.7),
    (-0.5000, 0.8660, 2.10, 0.0120, 2.8, 3.3),
    (0.3000, -0.9539, 3.70, 0.0060, 3.5, 0.9),
];

#[inline]
fn smooth01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

#[inline]
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !len_sq.is_finite() || len_sq <= 0.0 {
        return [0.0, 1.0, 0.0];
    }
    let inv = 1.0 / len_sq.sqrt();
    [v[0] * inv, v[1] * inv, v[2] * inv]
}

/// Top-level water surface facade.
pub struct WaterV4 {
    quality: QualitySettings,
    time: f32,
    camera: [f32; 3],
    ripples: [Ripple; MAX_RIPPLES],
    wakes: [Wake; MAX_WAKES],
    next_ripple: usize,
    next_wake: usize,
}

impl WaterV4 {
    /// Build a facade. The settings are stored for root to inspect; the
    /// deterministic surface math uses conservative, fixed mobile-friendly
    /// defaults so the facade does not depend on quality field layout.
    #[inline]
    pub fn new(quality: QualitySettings) -> Self {
        let empty_ripple = Ripple {
            x: 0.0,
            z: 0.0,
            amplitude: 0.0,
            age: 0.0,
            life: 1.0,
            speed: 0.0,
            width: 1.0,
            active: false,
        };
        let empty_wake = Wake {
            x: 0.0,
            z: 0.0,
            dir_x: 0.0,
            dir_z: 1.0,
            amplitude: 0.0,
            half_width: 1.0,
            spread: 0.0,
            age: 0.0,
            life: 1.0,
            active: false,
        };
        Self {
            quality,
            time: 0.0,
            camera: [0.0, 0.0, 0.0],
            ripples: [empty_ripple; MAX_RIPPLES],
            wakes: [empty_wake; MAX_WAKES],
            next_ripple: 0,
            next_wake: 0,
        }
    }

    /// Access the settings the facade was built with.
    #[inline]
    pub fn quality(&self) -> &QualitySettings {
        &self.quality
    }

    /// Last camera position supplied to [`WaterV4::step`].
    #[inline]
    pub fn camera(&self) -> [f32; 3] {
        self.camera
    }

    /// Simulated time in seconds.
    #[inline]
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Advance the surface. Both dt and camera are sanitised; non-finite or
    /// negative dt is treated as a zero-length step.
    pub fn step(&mut self, dt: f32, cam: [f32; 3]) {
        let dt = if dt.is_finite() { dt.clamp(0.0, MAX_DT) } else { 0.0 };
        self.time += dt;

        for c in 0..3 {
            self.camera[c] = finite_or(cam[c], self.camera[c]);
        }

        for r in self.ripples.iter_mut().filter(|r| r.active) {
            r.age += dt;
            if r.age >= r.life {
                r.active = false;
            }
        }
        for w in self.wakes.iter_mut().filter(|w| w.active) {
            w.age += dt;
            w.half_width += w.spread * dt;
            if w.age >= w.life {
                w.active = false;
            }
        }
    }

    /// Inject an expanding circular ripple centred at `(x, z)`.
    /// Amplitudes are clamped to a sane range; non-finite input is ignored.
    pub fn push_ripple(&mut self, x: f32, z: f32, amplitude: f32) {
        if !x.is_finite() || !z.is_finite() || !amplitude.is_finite() {
            return;
        }
        let slot = self.next_ripple % MAX_RIPPLES;
        self.next_ripple = self.next_ripple.wrapping_add(1);
        self.ripples[slot] = Ripple {
            x,
            z,
            amplitude: amplitude.clamp(0.0, 2.0),
            age: 0.0,
            life: 2.2,
            speed: 1.4,
            width: 0.35,
            active: true,
        };
    }

    /// Inject a wake node at `(x, z)` travelling along `(dir_x, dir_z)`.
    /// `is_boat` selects the wider, longer-lived boat wake profile.
    pub fn push_wake(&mut self, x: f32, z: f32, dir_x: f32, dir_z: f32, speed: f32, is_boat: bool) {
        if !x.is_finite() || !z.is_finite() {
            return;
        }
        let dx = finite_or(dir_x, 0.0);
        let dz = finite_or(dir_z, 0.0);
        let len = (dx * dx + dz * dz).sqrt();
        let (nx, nz) = if len > 1.0e-5 {
            (dx / len, dz / len)
        } else {
            (0.0, 1.0)
        };
        let spd = finite_or(speed, 0.0).clamp(0.0, 12.0);
        let amp = (spd * 0.40).clamp(0.30, 1.60) * if is_boat { 1.0 } else { 0.7 };
        let half_width = 0.28 + spd * 0.06 + if is_boat { 0.12 } else { 0.0 };

        let slot = self.next_wake % MAX_WAKES;
        self.next_wake = self.next_wake.wrapping_add(1);
        self.wakes[slot] = Wake {
            x,
            z,
            dir_x: nx,
            dir_z: nz,
            amplitude: amp,
            half_width,
            spread: if is_boat { 0.55 } else { 0.40 },
            age: 0.0,
            life: if is_boat { 3.5 } else { 2.2 },
            active: true,
        };
    }

    /// Number of currently active ripples and wakes, for diagnostics.
    #[inline]
    pub fn active_counts(&self) -> (usize, usize) {
        (
            self.ripples.iter().filter(|r| r.active).count(),
            self.wakes.iter().filter(|w| w.active).count(),
        )
    }

    #[inline]
    fn ambient_height(&self, x: f32, z: f32) -> f32 {
        let t = self.time;
        let mut h = 0.0f32;
        for &(dx, dz, freq, amp, speed, phase) in AMBIENT_WAVES.iter() {
            let p = freq * (dx * x + dz * z) - speed * t + phase;
            h += amp * p.sin();
        }
        h
    }

    #[inline]
    fn ripple_height(&self, x: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        for r in self.ripples.iter().filter(|r| r.active) {
            let p = r.age / r.life;
            if p >= 1.0 {
                continue;
            }
            let env = smooth01(p / 0.08) * (1.0 - p) * (1.0 - p);
            if env <= 0.0 {
                continue;
            }
            let dx = x - r.x;
            let dz = z - r.z;
            let dist = (dx * dx + dz * dz).sqrt();
            let d = dist - r.speed * r.age;
            let envelope = (-0.5 * d * d / (r.width * r.width)).exp();
            let oscillation = 0.5 + 0.5 * (d * 5.0).cos();
            sum += r.amplitude * env * envelope * oscillation;
        }
        sum
    }

    #[inline]
    fn wake_height(&self, x: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        for w in self.wakes.iter().filter(|w| w.active) {
            let p = w.age / w.life;
            if p >= 1.0 {
                continue;
            }
            let env = smooth01(p / 0.05) * (1.0 - p) * (1.0 - p);
            if env <= 0.0 {
                continue;
            }
            let dx = x - w.x;
            let dz = z - w.z;
            let along = dx * w.dir_x + dz * w.dir_z;
            if along > 0.25 {
                continue;
            }
            let behind = (-along).max(0.0);
            let lateral = -dx * w.dir_z + dz * w.dir_x;
            let hw = w.half_width.max(0.05);
            let cross = (-0.5 * lateral * lateral / (hw * hw)).exp();
            let trail = (-behind * 0.55).exp() * (0.55 + 0.45 * (behind * 5.0).cos());
            sum += w.amplitude * env * cross * trail;
        }
        sum
    }

    /// Surface displacement above the flat water plane at `(x, z)`.
    /// Non-finite coordinates yield `0.0` instead of propagating NaN.
    #[inline]
    pub fn surface_height(&self, x: f32, z: f32) -> f32 {
        if !x.is_finite() || !z.is_finite() {
            return 0.0;
        }
        finite_or(
            self.ambient_height(x, z) + self.ripple_height(x, z) + self.wake_height(x, z),
            0.0,
        )
    }

    /// Unit, upward-pointing surface normal at `(x, z)` obtained from central
    /// differences of [`WaterV4::surface_height`].
    #[inline]
    pub fn surface_normal(&self, x: f32, z: f32) -> [f32; 3] {
        if !x.is_finite() || !z.is_finite() {
            return [0.0, 1.0, 0.0];
        }
        let step = NORMAL_EPS;
        let dh_dx = (self.surface_height(x + step, z) - self.surface_height(x - step, z))
            / (2.0 * step);
        let dh_dz = (self.surface_height(x, z + step) - self.surface_height(x, z - step))
            / (2.0 * step);
        if !dh_dx.is_finite() || !dh_dz.is_finite() {
            return [0.0, 1.0, 0.0];
        }
        normalize3([-dh_dx, 1.0, -dh_dz])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facade() -> WaterV4 {
        WaterV4::new(QualitySettings::from_name("Smooth"))
    }

    fn close(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn identical_facades_have_identical_surface_output() {
        let mut a = facade();
        let mut b = facade();
        for i in 0..12 {
            let dt = 1.0 / 60.0;
            a.step(dt, [i as f32, 0.0, 0.0]);
            b.step(dt, [i as f32, 0.0, 0.0]);
        }

        let samples = [(0.0, 0.0), (1.25, -3.5), (-7.0, 2.75), (13.5, 9.0)];
        for &(x, z) in samples.iter() {
            assert_eq!(a.surface_height(x, z), b.surface_height(x, z));
            assert_eq!(a.surface_normal(x, z), b.surface_normal(x, z));
        }

        // Normals must be unit length and point upward.
        for &(x, z) in samples.iter() {
            let n = a.surface_normal(x, z);
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!(close(len, 1.0, 1e-5), "normal length {len}");
            assert!(n[1] > 0.0, "normal must be Y-up: {n:?}");
        }
    }

    #[test]
    fn ripple_and_wake_change_surface_then_fade_back() {
        let mut control = facade();
        let mut disturbed = facade();

        // Establish a matching time base for both facades.
        for _ in 0..12 {
            control.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
            disturbed.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
        }

        disturbed.push_ripple(0.0, 0.0, 0.5);
        disturbed.push_wake(0.0, 0.0, 0.0, 1.0, 3.0, true);
        assert_eq!(disturbed.active_counts(), (1, 1));

        // Advance until the ripple crest has travelled out to the sample.
        for _ in 0..12 {
            control.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
            disturbed.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
        }

        let ripple_sample = (0.28, 0.0);
        let wake_sample = (0.0, -0.40);
        assert!(
            disturbed.surface_height(ripple_sample.0, ripple_sample.1)
                > control.surface_height(ripple_sample.0, ripple_sample.1) + 1e-4,
            "ripple should raise its crest above ambient"
        );
        assert!(
            disturbed.surface_height(wake_sample.0, wake_sample.1)
                > control.surface_height(wake_sample.0, wake_sample.1) + 1e-4,
            "wake should raise its crest above ambient"
        );

        // Let every disturbance expire; the surface must return to ambient.
        for _ in 0..240 {
            control.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
            disturbed.step(1.0 / 60.0, [0.0, 0.0, 0.0]);
        }
        assert_eq!(disturbed.active_counts(), (0, 0));
        let x = ripple_sample.0;
        let z = ripple_sample.1;
        assert!(close(
            disturbed.surface_height(x, z),
            control.surface_height(x, z),
            1e-6
        ));
    }

    #[test]
    fn degenerate_inputs_stay_finite_and_flat() {
        let mut f = facade();
        f.push_ripple(f32::NAN, 0.0, 1.0);
        f.push_wake(0.0, f32::INFINITY, 0.0, 0.0, f32::NAN, false);
        assert_eq!(f.active_counts(), (0, 0));

        f.step(f32::NAN, [f32::NAN, 1.0, 2.0]);
        assert_eq!(f.time(), 0.0);
        assert_eq!(f.surface_height(f32::NAN, 0.0), 0.0);
        assert_eq!(f.surface_normal(f32::NAN, 0.0), [0.0, 1.0, 0.0]);
    }
}

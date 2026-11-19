//! Phillips-like directional wave spectrum for water v4.
//!
//! `WaveSpectrum::from_wind` turns a wind speed/direction plus a `u64` seed
//! into a fixed set of eight Gerstner components. Each component is packed as
//! `[amplitude, wavelength, dir_x, dir_z, phase_speed, phase]` so callers can
//! feed it straight into a Gerstner wave evaluator:
//!
//! ```ignore
//! let spectrum = WaveSpectrum::from_wind(9.0, [1.0, 0.3], 42);
//! for &(amp, wavelength, dx, dz, speed, phase) in spectrum.components() {
//!     let k = TAU / wavelength;
//!     let f = k * (dx * x + dz * z - speed * t) + phase;
//!     height += amp * f.sin();
//! }
//! ```
//!
//! Design:
//! * Wavelengths are log-spaced around the Phillips dominant wavelength
//!   `L = U^2 / g`, so higher wind produces longer, faster waves.
//! * Directional energy follows the Phillips `cos^2(theta)` lobe around the
//!   wind direction, with deterministic jitter so the field is not symmetric.
//! * Amplitudes are normalized to a wind-dependent budget to stay bounded.
//! * All randomness comes from a splitmix64-style hash of `(seed, salt)`, so
//!   the output is deterministic on every platform. No globals, no crates, no
//!   allocation in hot paths. Units are Minecraft blocks (1 block = 1 m).

/// Number of Gerstner components produced by the spectrum.
pub const COMPONENT_COUNT: usize = 8;

/// Gravity in m/s^2 (Minecraft blocks are treated as metres).
const G: f32 = 9.81;
/// 2*PI as f32.
const TAU: f32 = std::f32::consts::TAU;
/// Wavelength of the Phillips spectral peak is about `7.7 * L`.
const PEAK_WAVELENGTH_PER_L: f32 = 7.7;
/// Maximum half-spread of wave directions away from the wind, in radians.
const MAX_SPREAD: f32 = 1.05;

/// Deterministic Gerstner wave spectrum derived from wind.
#[derive(Clone, Copy, Debug)]
pub struct WaveSpectrum {
    components: [[f32; 6]; COMPONENT_COUNT],
}

impl WaveSpectrum {
    /// Build a spectrum from wind speed (blocks/s), wind direction (XZ, the
    /// direction the wind blows *towards*) and a decorrelation seed.
    ///
    /// Non-finite or negative wind speed is treated as calm water. A zero or
    /// non-finite direction falls back to `+X`. All returned values are finite.
    #[inline]
    pub fn from_wind(speed: f32, dir: [f32; 2], seed: u64) -> Self {
        let wind = if speed.is_finite() {
            speed.clamp(0.0, 40.0)
        } else {
            0.0
        };

        let (wind_x, wind_z) = if dir[0].is_finite() && dir[1].is_finite() {
            let len_sq = dir[0] * dir[0] + dir[1] * dir[1];
            if len_sq > 1.0e-8 && len_sq.is_finite() {
                let inv = 1.0 / len_sq.sqrt();
                (dir[0] * inv, dir[1] * inv)
            } else {
                (1.0, 0.0)
            }
        } else {
            (1.0, 0.0)
        };
        let wind_angle = wind_z.atan2(wind_x);

        // Phillips_overfetch scale, clamped to keep calm-water components sane.
        let fetch = (wind * wind / G).clamp(0.5, 60.0);
        let peak_wavelength = (PEAK_WAVELENGTH_PER_L * fetch).clamp(6.0, 120.0);
        let short_wavelength = (peak_wavelength * 0.18).clamp(1.5, 24.0);

        // Wind-driven displacement budget: calm water is flat, strong wind is
        // lively but never large enough to break the overlay mesh.
        let amplitude_budget = (wind * 0.011).clamp(0.0, 0.36);

        // First pass: wavelengths, directions and unnormalised Phillips weights.
        let mut scratch = [[0.0f32; 6]; COMPONENT_COUNT];
        let mut weight_sum = 0.0f32;
        let length_ratio = short_wavelength / peak_wavelength;

        for i in 0..COMPONENT_COUNT {
            let fi = i as f32;
            let t = fi / (COMPONENT_COUNT - 1) as f32;
            let wavelength = peak_wavelength * length_ratio.powf(t);
            let k = TAU / wavelength;
            let kl = k * fetch;

            // Evenly tile the wind lobe with a small seeded jitter. Long waves
            // stay closer to the wind, short ones spread a little wider.
            let base = 2.0 * ((fi + 0.5) / COMPONENT_COUNT as f32) - 1.0;
            let jitter = (param(seed, salt(0, i)) - 0.5) * 0.35;
            let spread = (base + jitter).clamp(-1.0, 1.0) * MAX_SPREAD * (0.75 + 0.25 * t);
            let angle = wind_angle + spread;
            let alignment = spread.cos().max(0.0);

            // Phillips-directional energy: k^-4 * exp(-1/(k L)^2) * cos^2(theta).
            // A little hash jitter breaks up the otherwise regular spectrum.
            let phillips = (-1.0 / (kl * kl)).exp() / (k * k * k * k);
            let jitter_gain = 0.75 + 0.5 * param(seed, salt(1, i));
            let weight = phillips * alignment * alignment * jitter_gain;
            weight_sum += weight;

            let phase_speed = (G / k).sqrt().clamp(0.25, 18.0);
            let phase = (param(seed, salt(2, i)) * 0.99999994) * TAU;

            scratch[i] = [weight, wavelength, angle.cos(), angle.sin(), phase_speed, phase];
        }

        // Second pass: distribute the amplitude budget proportionally to energy.
        let inv_weight = if weight_sum > 1.0e-12 {
            1.0 / weight_sum
        } else {
            0.0
        };
        let mut components = [[0.0f32; 6]; COMPONENT_COUNT];
        for i in 0..COMPONENT_COUNT {
            components[i] = scratch[i];
            components[i][0] = amplitude_budget * scratch[i][0] * inv_weight;
        }

        Self { components }
    }

    /// The eight components as `[amplitude, wavelength, dir_x, dir_z, speed, phase]`.
    #[inline]
    pub fn components(&self) -> &[[f32; 6]] {
        &self.components
    }
}

/// Stable per-component hash salt.
#[inline]
fn salt(channel: u64, index: usize) -> u64 {
    0x9E37_79B9_7F4A_7C15
        .wrapping_mul(channel + 1)
        .wrapping_add(index as u64)
}

/// Splitmix64 finalizer.
#[inline]
fn mix64(mut h: u64) -> u64 {
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h
}

/// Deterministic scalar parameter in `[0, 1)` derived from `seed` and `salt`.
#[inline]
fn param(seed: u64, salt_value: u64) -> f32 {
    let h = mix64(seed.wrapping_add(salt_value));
    ((h >> 40) as f32) * (1.0 / 16777216.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let a = WaveSpectrum::from_wind(9.5, [1.0, 0.35], 0x1234_5678_9ABC_DEF0);
        let b = WaveSpectrum::from_wind(9.5, [1.0, 0.35], 0x1234_5678_9ABC_DEF0);
        let c = WaveSpectrum::from_wind(9.5, [1.0, 0.35], 0x1234_5678_9ABC_DEF1);

        assert_eq!(a.components(), b.components());
        assert_ne!(a.components(), c.components(), "seed must decorrelate phases");
        assert_eq!(a.components().len(), COMPONENT_COUNT);
    }

    #[test]
    fn components_are_physically_valid() {
        let s = WaveSpectrum::from_wind(12.0, [0.0, 1.0], 7);
        let mut total_amp = 0.0f32;
        let mut dot_sum = 0.0f32;

        for &(amp, wavelength, dx, dz, speed, phase) in s.components() {
            assert!(amp.is_finite() && amp >= 0.0);
            assert!(wavelength.is_finite() && wavelength > 0.0);
            assert!(speed.is_finite() && speed > 0.0);
            assert!(phase.is_finite() && (0.0..TAU).contains(&phase));
            assert!(approx(dx * dx + dz * dz, 1.0, 1.0e-5), "direction not unit");
            // Waves should mostly travel with the wind (+Z here).
            assert!(dz > -0.2, "component too far against the wind: {dz}");
            total_amp += amp;
            dot_sum += dz;
        }

        assert!(total_amp > 0.01 && total_amp <= 0.37, "budget: {total_amp}");
        assert!(dot_sum > 3.0, "spectrum should follow wind: {dot_sum}");
    }

    #[test]
    fn wind_speed_changes_wavelength_and_amplitude() {
        let calm = WaveSpectrum::from_wind(0.0, [1.0, 0.0], 42);
        let breeze = WaveSpectrum::from_wind(8.0, [1.0, 0.0], 42);
        let gale = WaveSpectrum::from_wind(25.0, [1.0, 0.0], 42);

        let sum_amp = |s: &WaveSpectrum| -> f32 { s.components().iter().map(|c| c[0]).sum() };
        let max_lambda = |s: &WaveSpectrum| -> f32 {
            s.components().iter().map(|c| c[1]).fold(0.0f32, f32::max)
        };

        assert_eq!(sum_amp(&calm), 0.0);
        assert!(sum_amp(&breeze) > 0.0 && sum_amp(&breeze) < sum_amp(&gale));
        assert!(max_lambda(&calm) < max_lambda(&breeze));
        assert!(max_lambda(&breeze) < max_lambda(&gale));
    }

    #[test]
    fn degenerate_inputs_stay_finite() {
        let s = WaveSpectrum::from_wind(f32::NAN, [f32::NAN, 0.0], 99);
        for c in s.components() {
            for &v in c.iter() {
                assert!(v.is_finite(), "NaN leaked into component: {c:?}");
            }
        }
        let z = WaveSpectrum::from_wind(-5.0, [0.0, 0.0], 99);
        assert!(z.components().iter().all(|c| c[0] == 0.0));
    }
}

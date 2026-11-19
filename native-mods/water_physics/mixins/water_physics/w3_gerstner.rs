// w3_gerstner.rs
// Deterministic sum-of-N Gerstner (trochoidal) wave field for ambient ocean motion.
//
// Each wave is a directional plane wave with:
//   - a horizontal displacement along its travel direction (crests sharpen, troughs flatten)
//   - a vertical displacement (the familiar sinusoidal height)
//   - analytic partial derivatives of the displaced surface, from which the
//     normal is constructed via the cross product of the two surface tangents.
//
// Wave parameters (direction, wavelength, speed, amplitude, steepness, phase)
// are seeded deterministically from a u64 via a splitmix64-style finalizer.
// No globals, no unsafe, no heap allocation. O(N) per sample with N = 6.
//
// Coordinate system: X/Z horizontal, Y up. World units are Minecraft blocks.
// Callers should add `sample.dy` to `water_y + 8.0/9.0` and add `sample.dx/dz`
// to their world X/Z position before shading or meshing.

/// 2*PI as f32.
const TAU: f32 = std::f32::consts::TAU;

/// Number of Gerstner waves summed by default.
pub const DEFAULT_WAVE_COUNT: usize = 6;

/// One Gerstner wave component. Precomputed once in [`GerstnerField::new`].
#[derive(Clone, Copy, Debug)]
pub struct GerstnerWave {
    /// Unit direction X component (direction of travel).
    pub dir_x: f32,
    /// Unit direction Z component.
    pub dir_z: f32,
    /// Angular wavenumber `TAU / wavelength`.
    pub k: f32,
    /// Angular frequency `speed * k`.
    pub omega: f32,
    /// Vertical amplitude (blocks).
    pub amplitude: f32,
    /// Steepness in `[0, 1]`. Horizontal amplitude is `steepness * amplitude`.
    pub steepness: f32,
    /// Phase offset (radians).
    pub phase: f32,
}

/// Result of sampling the Gerstner field at a world position and time.
#[derive(Clone, Copy, Debug, Default)]
pub struct WaveSample {
    /// Horizontal X displacement (blocks).
    pub dx: f32,
    /// Vertical displacement (blocks). Add to the local water surface height.
    pub dy: f32,
    /// Horizontal Z displacement (blocks).
    pub dz: f32,
    /// Unit surface normal X component (points generally upward).
    pub nx: f32,
    /// Unit surface normal Y component (positive for a non-overturned surface).
    pub ny: f32,
    /// Unit surface normal Z component.
    pub nz: f32,
}

/// Deterministic ambient Gerstner wave field.
#[derive(Clone, Debug)]
pub struct GerstnerField {
    /// Wave components (always `DEFAULT_WAVE_COUNT`, built from the seed).
    pub waves: [GerstnerWave; DEFAULT_WAVE_COUNT],
    /// Global amplitude multiplier applied to every wave.
    pub amplitude_scale: f32,
}

impl GerstnerField {
    /// Build a field from a `u64` seed.
    ///
    /// Directions are spread around the compass with seeded jitter; wavelengths,
    /// amplitudes, speeds, steepness and phases are all seeded. The result is
    /// fully deterministic: the same seed always yields the same field.
    pub fn new(seed: u64) -> Self {
        let mut waves = [GerstnerWave {
            dir_x: 1.0,
            dir_z: 0.0,
            k: 0.0,
            omega: 0.0,
            amplitude: 0.0,
            steepness: 0.0,
            phase: 0.0,
        }; DEFAULT_WAVE_COUNT];

        for (i, wave) in waves.iter_mut().enumerate() {
            let salt = (i as u64).wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15);

            // Spread base directions evenly around a circle, then jitter.
            let base_angle = TAU * (i as f32) / (DEFAULT_WAVE_COUNT as f32);
            let angle = base_angle + (param(seed, salt) - 0.5) * 0.9;

            // Wavelengths from short chop (7 m) to long swell (34 m), decreasing
            // with index so low indices carry the broad swell.
            let wavelength = 34.0 - 5.0 * (i as f32) + param(seed, salt ^ 0xA5A5_A5A5) * 4.0;
            let k = TAU / wavelength;

            // Deep-water dispersion: omega = sqrt(g*k), g = 9.81 m/s^2.
            // A small seeded speed variation keeps waves from looking mechanical.
            let speed_variation = 0.85 + 0.30 * param(seed, salt ^ 0x5A5A_5A5A);
            let omega = (9.81 * k).sqrt() * speed_variation;

            // Amplitude falls off with wavenumber: long waves are taller.
            let amplitude = (0.16 + 0.10 * param(seed, salt ^ 0x1234_5678))
                / (1.0 + 0.35 * (i as f32));

            // Keep total steepness safely below 1 so no wave overturns.
            let steepness = 0.25 + 0.45 * param(seed, salt ^ 0x8765_4321);

            let phase = param(seed, salt ^ 0xDEAD_BEEF) * TAU;

            *wave = GerstnerWave {
                dir_x: angle.cos(),
                dir_z: angle.sin(),
                k,
                omega,
                amplitude,
                steepness,
                phase,
            };
        }

        Self {
            waves,
            amplitude_scale: 1.0,
        }
    }

    /// Sample the displaced position and analytic surface normal at `(x, z, t)`.
    ///
    /// The returned `dx/dy/dz` are offsets from the undisplaced point
    /// `(x, 0, z)`; `nx/ny/nz` form a unit normal. At `t = 0` with a seed whose
    /// phases sum to zero displacement at the origin, offsets are bounded by
    /// the sum of amplitudes. Non-finite inputs produce a flat, neutral sample.
    #[inline]
    pub fn sample(&self, x: f32, z: f32, t: f32) -> WaveSample {
        if !(x.is_finite() && z.is_finite() && t.is_finite()) {
            return WaveSample {
                nx: 0.0,
                ny: 1.0,
                nz: 0.0,
                ..WaveSample::default()
            };
        }

        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        let mut dz = 0.0f32;

        // Tangent of the displaced surface with respect to world X.
        let mut tx_x = 1.0f32;
        let mut tx_y = 0.0f32;
        let mut tx_z = 0.0f32;

        // Tangent with respect to world Z.
        let mut tz_x = 0.0f32;
        let mut tz_y = 0.0f32;
        let mut tz_z = 1.0f32;

        let scale = self.amplitude_scale;

        for wave in self.waves.iter() {
            let phase = wave.k * (wave.dir_x * x + wave.dir_z * z) - wave.omega * t + wave.phase;
            let (sin_p, cos_p) = phase.sin_cos();

            let a = wave.amplitude * scale;
            let q = wave.steepness;
            let qa = q * a;
            let ka = wave.k * a;
            let kqa = wave.k * qa;

            // Displaced position.
            dx += wave.dir_x * qa * cos_p;
            dy += a * sin_p;
            dz += wave.dir_z * qa * cos_p;

            // Derivatives of displacement w.r.t. x and z.
            let dxx = -wave.dir_x * wave.dir_x * kqa * sin_p;
            let dxz = -wave.dir_x * wave.dir_z * kqa * sin_p;
            let dyx = wave.dir_x * ka * cos_p;
            let dyz = wave.dir_z * ka * cos_p;
            let dzz = -wave.dir_z * wave.dir_z * kqa * sin_p;

            // Surface tangents: dP/dx = (1+dxx, dyx, dxz), dP/dz = (dxz, dyz, 1+dzz).
            tx_x += dxx;
            tx_y += dyx;
            tx_z += dxz;
            tz_x += dxz;
            tz_y += dyz;
            tz_z += dzz;
        }

        // Normal = dP/dz x dP/dx, chosen so that ny >= 0 for non-overturned waves.
        let mut nx = tz_y * tx_z - tz_z * tx_y;
        let mut ny = tz_z * tx_x - tz_x * tx_z;
        let mut nz = tz_x * tx_y - tz_y * tx_x;

        let len_sq = nx * nx + ny * ny + nz * nz;
        if !len_sq.is_finite() || len_sq <= 0.0 {
            return WaveSample {
                dx,
                dy,
                dz,
                nx: 0.0,
                ny: 1.0,
                nz: 0.0,
            };
        }

        // Flip toward +Y in case numerical issues produced a downward normal.
        if ny < 0.0 {
            nx = -nx;
            ny = -ny;
            nz = -nz;
        }

        let inv_len = 1.0 / len_sq.sqrt();
        WaveSample {
            dx,
            dy,
            dz,
            nx: nx * inv_len,
            ny: ny * inv_len,
            nz: nz * inv_len,
        }
    }
}

/// Splitmix64 finalizer. Maps a 64-bit seed to a well-distributed 64-bit value.
#[inline]
fn mix64(mut h: u64) -> u64 {
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h
}

/// Deterministic scalar parameter in `[0, 1)` derived from `seed` and a salt.
#[inline]
fn param(seed: u64, salt: u64) -> f32 {
    let h = mix64(seed.wrapping_add(salt));
    ((h >> 40) as f32) * (1.0 / 16777216.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline]
    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn flat_at_t_zero_bounds() {
        let field = GerstnerField::new(0x1234_5678_9ABC_DEF0);

        // At t=0, all displacements are bounded by the sum of amplitudes.
        let max_vertical: f32 = field.waves.iter().map(|w| w.amplitude).sum();
        let max_horizontal: f32 = field.waves.iter().map(|w| w.steepness * w.amplitude).sum();

        for ix in -8..=8 {
            for iz in -8..=8 {
                let x = ix as f32 * 2.5;
                let z = iz as f32 * 2.5;
                let s = field.sample(x, z, 0.0);
                assert!(s.dx.is_finite() && s.dy.is_finite() && s.dz.is_finite());
                assert!(
                    s.dy.abs() <= max_vertical + 1e-5,
                    "dy {} exceeds bound {}",
                    s.dy,
                    max_vertical
                );
                assert!(
                    s.dx.abs() <= max_horizontal + 1e-5,
                    "dx {} exceeds bound {}",
                    s.dx,
                    max_horizontal
                );
                assert!(
                    s.dz.abs() <= max_horizontal + 1e-5,
                    "dz {} exceeds bound {}",
                    s.dz,
                    max_horizontal
                );
            }
        }
    }

    #[test]
    fn normals_are_unit_length_and_y_up() {
        let field = GerstnerField::new(42);
        for ix in -6..=6 {
            for iz in -6..=6 {
                for t in [0.0f32, 1.25, 7.5, 31.0] {
                    let s = field.sample(ix as f32 * 3.1, iz as f32 * 2.7, t);
                    let len = (s.nx * s.nx + s.ny * s.ny + s.nz * s.nz).sqrt();
                    assert!(approx(len, 1.0, 1e-4), "normal length was {}", len);
                    assert!(s.ny >= 0.0, "normal should be Y-up, got {:?}", s);
                }
            }
        }
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let seed = 0xDEAD_BEEFu64;
        let field = GerstnerField::new(seed);
        let s = field.sample(11.5, -4.25, 3.75);
        assert_eq!(s.dx, GerstnerField::new(seed).sample(11.5, -4.25, 3.75).dx);
        assert_eq!(s.dy, GerstnerField::new(seed).sample(11.5, -4.25, 3.75).dy);
        assert_eq!(s.dz, GerstnerField::new(seed).sample(11.5, -4.25, 3.75).dz);

        // A different seed must produce a visibly different field somewhere.
        let other = GerstnerField::new(seed.wrapping_add(1));
        let mut changed = false;
        for i in 0..16 {
            let x = i as f32 * 2.3;
            let a = field.sample(x, -4.25, 3.75).dy;
            let b = other.sample(x, -4.25, 3.75).dy;
            if (a - b).abs() > 1.0e-3 {
                changed = true;
            }
        }
        assert!(changed, "seed must decorrelate the wave field");
    }

    #[test]
    fn field_evolves_over_time_and_handles_non_finite_inputs() {
        let field = GerstnerField::new(7);
        let a = field.sample(3.0, 5.0, 0.0);
        let b = field.sample(3.0, 5.0, 2.5);
        assert!((a.dy - b.dy).abs() > 1.0e-4, "field must evolve over time");

        for (x, z, t) in [(f32::NAN, 0.0, 0.0), (0.0, f32::INFINITY, 0.0), (0.0, 0.0, f32::NAN)] {
            let s = field.sample(x, z, t);
            assert_eq!(s.dx, 0.0);
            assert_eq!(s.dy, 0.0);
            assert_eq!(s.dz, 0.0);
            assert_eq!((s.nx, s.ny, s.nz), (0.0, 1.0, 0.0));
        }
    }
}

// w3_spray: deterministic bow/stern spray emission for boats and swimmers.
//
// Pure math, allocation-free and deterministic. The caller owns all state:
// emit_rate() decides how many particles per second to spawn, spray_velocity()
// builds one particle velocity from travel direction/speed and a stable seed,
// and spray_alpha() fades a particle over its lifetime without spawn/death pops.
//
// Coordinate system: X/Z horizontal, Y up. Units: metres and seconds.

/// Below this speed (m/s) no spray is produced.
pub const SPRAY_MIN_SPEED: f32 = 0.35;
/// Speed (m/s) at which the base emission rate saturates.
pub const SPRAY_FULL_SPEED: f32 = 6.0;
/// Base particles per second at SPRAY_FULL_SPEED.
pub const SPRAY_BASE_RATE: f32 = 36.0;
/// Hard ceiling for emission rate (particles per second).
pub const SPRAY_MAX_RATE: f32 = 72.0;

#[inline]
fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

#[inline]
fn smoothstep01(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// SplitMix64 finalizer: tiny, deterministic, no allocations or globals.
#[inline]
fn hash_u64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn rand01(seed: u64, salt: u64) -> f32 {
    // 24-bit mantissa keeps the result exact in f32.
    ((hash_u64(seed ^ salt) >> 40) as f32) / 16_777_216.0
}

/// Particles per second emitted by a body moving at `speed` m/s.
/// Zero until motion is strong enough, then a smooth ramp plus a bounded
/// high-speed boost so fast boats produce a visibly denser rooster tail.
#[inline]
pub fn emit_rate(speed: f32) -> f32 {
    let s = finite_or(speed, 0.0);
    if s <= SPRAY_MIN_SPEED {
        return 0.0;
    }
    let t = ((s - SPRAY_MIN_SPEED) / (SPRAY_FULL_SPEED - SPRAY_MIN_SPEED)).clamp(0.0, 1.0);
    let base = SPRAY_BASE_RATE * smoothstep01(t);
    let boost = (s - SPRAY_FULL_SPEED).max(0.0) * 6.0;
    (base + boost).min(SPRAY_MAX_RATE)
}

/// Velocity [vx, vy, vz] (m/s) for one spray droplet.
///
/// `dir_x`/`dir_z` are the (normalised or raw) horizontal travel direction.
/// Spray is thrown backwards and slightly sideways from the hull/body, with
/// an upward component; `seed` selects a stable per-particle fan and speed.
#[inline]
pub fn spray_velocity(dir_x: f32, dir_z: f32, speed: f32, seed: u64) -> [f32; 3] {
    let s = finite_or(speed, 0.0).clamp(0.0, 12.0);
    let dx = finite_or(dir_x, 0.0);
    let dz = finite_or(dir_z, 0.0);
    let len = (dx * dx + dz * dz).sqrt();

    // Degenerate direction: pick a deterministic forward axis (+Z).
    let (fx, fz) = if len > 1.0e-5 {
        (dx / len, dz / len)
    } else {
        (0.0, 1.0)
    };

    let r_fan = rand01(seed, 0x51ED_270B_093A_7653);
    let r_back = rand01(seed, 0x243F_6A88_85A3_08D3);
    let r_h = rand01(seed, 0xB7E1_5162_14C0_4A27);
    let r_v = rand01(seed, 0x9E37_79B9_7F4A_7C15);

    // Left/right fan around the backwards direction.
    let fan = (r_fan * 2.0 - 1.0) * 0.75;
    let back = 0.40 + 0.30 * r_back;
    let hx = -fx * back - fz * fan;
    let hz = -fz * back + fx * fan;

    let horizontal = s * (0.16 + 0.12 * r_h);
    let vertical = if s > 0.0 {
        0.12 + s * (0.26 + 0.18 * r_v)
    } else {
        0.0
    };

    [hx * horizontal, vertical, hz * horizontal]
}

/// Particle alpha in [0, 1]. Smooth fade-in and fade-out keep spray from
/// popping into or out of existence. `age` and `life` are seconds.
#[inline]
pub fn spray_alpha(age: f32, life: f32) -> f32 {
    let a = finite_or(age, 0.0);
    let l = finite_or(life, 0.0);
    if l <= 0.0 || a <= 0.0 || a >= l {
        return 0.0;
    }
    let attack = smoothstep01(a / (l * 0.12));
    let release = smoothstep01((l - a) / (l * 0.5));
    (attack * release).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_rate_ramps_from_zero_and_stays_bounded() {
        assert_eq!(emit_rate(0.0), 0.0);
        assert_eq!(emit_rate(SPRAY_MIN_SPEED), 0.0);
        assert_eq!(emit_rate(-3.0), 0.0);
        assert!(emit_rate(f32::NAN) == 0.0);
        assert!(emit_rate(1.0) > 0.0);
        let mut prev = 0.0;
        for i in 0..=40 {
            let s = i as f32 * 0.5;
            let r = emit_rate(s);
            assert!(r >= prev, "rate must be monotonic at speed {}", s);
            assert!(r <= SPRAY_MAX_RATE);
            prev = r;
        }
        assert!(emit_rate(SPRAY_FULL_SPEED) > emit_rate(1.0));
        assert!(emit_rate(10.0) >= emit_rate(SPRAY_FULL_SPEED));
    }

    #[test]
    fn spray_velocity_is_deterministic_directional_and_speed_scaled() {
        let a = spray_velocity(0.0, 1.0, 5.0, 42);
        let b = spray_velocity(0.0, 1.0, 5.0, 42);
        assert_eq!(a, b);

        // Travelling toward +Z throws spray toward -Z and upward.
        assert!(a[2] < 0.0, "expected backward spray, got {:?}", a);
        assert!(a[1] > 0.0, "expected upward spray, got {:?}", a);

        // Direction length must not matter (normalisation).
        let scaled = spray_velocity(0.0, 7.5, 5.0, 42);
        for i in 0..3 {
            assert!((a[i] - scaled[i]).abs() < 1.0e-5);
        }

        // Different seeds vary the fan; higher speed increases magnitude.
        let other = spray_velocity(0.0, 1.0, 5.0, 43);
        assert_ne!(a, other);
        let slow = spray_velocity(1.0, 0.0, 1.0, 7);
        let fast = spray_velocity(1.0, 0.0, 8.0, 7);
        assert!(fast[0].abs() > slow[0].abs());
        assert!(fast[1] > slow[1]);

        // Degenerate input stays finite.
        for v in spray_velocity(0.0, 0.0, f32::NAN, 0) {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn spray_alpha_fades_at_both_ends_of_life() {
        assert_eq!(spray_alpha(0.0, 1.0), 0.0);
        assert_eq!(spray_alpha(1.0, 1.0), 0.0);
        assert_eq!(spray_alpha(2.0, 1.0), 0.0);
        assert_eq!(spray_alpha(0.5, 0.0), 0.0);
        assert!(spray_alpha(f32::NAN, 1.0) == 0.0);

        let mid = spray_alpha(0.5, 1.0);
        assert!((mid - 1.0).abs() < 1.0e-5, "midlife alpha {}", mid);
        assert!(spray_alpha(0.05, 1.0) < mid);
        assert!(spray_alpha(0.95, 1.0) < mid);

        let mut prev = 0.0;
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let a_val = spray_alpha(t, 1.0);
            assert!(a_val >= 0.0 && a_val <= 1.0);
            if t < 0.12 {
                assert!(a_val >= prev, "attack should rise at t={}", t);
            }
            prev = a_val;
        }
    }
}

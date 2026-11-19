// Continuous 3D wake-field height and normal evaluation for water v4.
//
// This module turns a single wake node (see `sim::WakeNode`) into an analytic
// height field over the X/Z plane. It is pure math: no crates, no globals, no
// allocation, and no dependency on `crate::game::Game` or on `sim.rs` itself.
//
// The local frame of a node is
//   * `s` = longitudinal offset (positive = forward, wake trails at `s < 0`),
//   * `w` = lateral offset (positive = left, using the node's left vector).
//
// The field is a smooth sum of
//   * two outer Kelvin V-arm crests at `w = +/- half_width`,
//   * a central stern-wash / keel ridge (stronger for boats),
//   * two narrower transverse wavelets bridging keel and arms (boats only),
// all windowed in `s` and `w` with C1-continuous falloff so neither the
// height nor its slope jumps at the edges of a node's influence.
//
// Public API:
// * [`WakeLike`] - accessor trait implemented by `sim::WakeNode`.
// * [`wake_height_and_slope`] - height plus world-space slopes in one pass.
// * [`wake_height`] - height only.
// * [`wake_normal`] - unit surface normal `[-dh/dx, 1, -dh/dz]`.
//
// Heights are relative to the water surface, in Minecraft blocks (meters),
// matching the crest displacement used by the v3.5 renderer (~0.04-0.06 m).

/// Read-only view of a wake node. Implement this for `sim::WakeNode` by
/// forwarding each accessor to the identically named field.
pub trait WakeLike {
    fn pos_x(&self) -> f64;
    fn pos_z(&self) -> f64;
    fn dir_x(&self) -> f32;
    fn dir_z(&self) -> f32;
    fn left_x(&self) -> f32;
    fn left_z(&self) -> f32;
    fn half_width(&self) -> f32;
    fn amplitude(&self) -> f32;
    fn age(&self) -> f32;
    fn max_age(&self) -> f32;
    fn is_boat(&self) -> bool;
}

/// Clamped Hermite smoothstep plus its derivative with respect to `t`.
#[inline]
fn smoothstep_d(t: f64) -> (f64, f64) {
    if t <= 0.0 {
        return (0.0, 0.0);
    }
    if t >= 1.0 {
        return (1.0, 0.0);
    }
    (t * t * (3.0 - 2.0 * t), 6.0 * t * (1.0 - t))
}

/// Unit Gaussian and its derivative at signed distance `d`.
#[inline]
fn gaussian_d(d: f64, inv_sigma_sq: f64) -> (f64, f64) {
    let g = (-0.5 * d * d * inv_sigma_sq).exp();
    (g, -d * inv_sigma_sq * g)
}

/// Evaluate the wake height and its world-space slopes at `(x, z)`.
///
/// Returns `(height, dh/dx, dh/dz)`. Height is a displacement above the flat
/// water surface; flat water outside the node's influence returns `(0, 0, 0)`.
#[inline]
pub fn wake_height_and_slope(node: &dyn WakeLike, x: f64, z: f64) -> (f32, f32, f32) {
    // Smooth temporal envelope: zero at spawn and death with zero slope.
    let max_age = node.max_age() as f64;
    if max_age <= 0.001 {
        return (0.0, 0.0, 0.0);
    }
    let age = node.age() as f64;
    let p = (age / max_age).clamp(0.0, 1.0);
    if p >= 1.0 {
        return (0.0, 0.0, 0.0);
    }
    let (attack, _) = smoothstep_d(p / 0.12);
    let tail = 1.0 - p;
    let env = attack * tail * tail * tail;
    if env <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let amp = (node.amplitude() as f64 * env).max(0.0);

    // Orthonormal local frame. Gram-Schmidt keeps the gradient transform valid
    // even if a caller supplies a left vector that is not perfectly perpendicular.
    let mut fx = node.dir_x() as f64;
    let mut fz = node.dir_z() as f64;
    let flen = (fx * fx + fz * fz).sqrt();
    if flen < 1e-6 {
        return (0.0, 0.0, 0.0);
    }
    fx /= flen;
    fz /= flen;

    let mut lx = node.left_x() as f64;
    let mut lz = node.left_z() as f64;
    let dot = lx * fx + lz * fz;
    lx -= dot * fx;
    lz -= dot * fz;
    let mut llen = (lx * lx + lz * lz).sqrt();
    if llen < 1e-6 {
        lx = -fz;
        lz = fx;
        llen = 1.0;
    }
    lx /= llen;
    lz /= llen;

    let rx = x - node.pos_x();
    let rz = z - node.pos_z();
    let s = rx * fx + rz * fz;
    let w = rx * lx + rz * lz;

    // Longitudinal window: fades in ahead of the node and out behind it.
    let (ahead, ahead_d) = smoothstep_d((0.22 - s) / 0.22);
    let (behind, behind_d) = smoothstep_d((0.95 + s) / 0.35);
    let gs = ahead * behind;
    if gs <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let gs_d = ahead_d * (-1.0 / 0.22) * behind + ahead * behind_d * (1.0 / 0.35);

    let is_boat = node.is_boat();
    let hw = node.half_width() as f64;
    let age_f = age;

    // Outer V-arm width matches the v3.5 ribbon renderer.
    let arm_full = if is_boat {
        (0.22 + 0.06 * hw).clamp(0.24, 0.42)
    } else {
        (0.16 + 0.05 * hw).clamp(0.18, 0.32)
    };
    let sigma = arm_full * 0.5;
    let inv_sigma_sq = 1.0 / (sigma * sigma);

    let mut sum = 0.0;
    let mut sum_d = 0.0;

    // Central stern wash / keel ridge.
    let keel_hw = if is_boat {
        (0.32 + 0.08 * age_f).clamp(0.35, 0.55)
    } else {
        0.20
    };
    let keel_sigma = keel_hw * 0.55;
    let (kg, kg_d) = gaussian_d(w, 1.0 / (keel_sigma * keel_sigma));
    let keel_amp = if is_boat {
        (0.025 * amp).min(0.035)
    } else {
        (0.012 * amp).min(0.018)
    };
    sum += keel_amp * kg;
    sum_d += keel_amp * kg_d;

    // Kelvin V-arms: symmetric crests at w = +/- half_width.
    let arm_amp = (0.045 * amp).min(0.060);
    for sign in [-1.0f64, 1.0] {
        let (g, g_d) = gaussian_d(w - sign * hw, inv_sigma_sq);
        sum += arm_amp * g;
        sum_d += arm_amp * g_d;
    }

    // Transverse wavelets bridging hull wash and outer arms (boats only).
    if is_boat {
        let gap = hw - sigma - keel_hw;
        if gap > 0.06 {
            let fade = smoothstep_d((gap - 0.06) / 0.14).0;
            if fade > 0.0 {
                let trans_sigma = 0.62 * sigma;
                // Transverse ridges sit halfway between the keel edge and the
                // inner edge of the outer arm.
                let mid = keel_hw + gap * 0.5;
                let trans_amp = (0.020 * amp).min(0.030) * fade;
                for sign in [-1.0f64, 1.0] {
                    let (g, g_d) = gaussian_d(w - sign * mid, 1.0 / (trans_sigma * trans_sigma));
                    sum += trans_amp * g;
                    sum_d += trans_amp * g_d;
                }
            }
        }
    }

    // Lateral window: smooth C1 cutoff well outside the outermost arm.
    let w_limit = hw + 3.5 * sigma + 0.15;
    let aw = w.abs();
    let (lat, lat_d_t) = smoothstep_d((w_limit - aw) / 0.30);
    if lat <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let lat_d = if aw > 0.0 {
        -lat_d_t * (w / aw) * (1.0 / 0.30)
    } else {
        0.0
    };

    let height = gs * lat * sum;
    let dh_ds = gs_d * lat * sum;
    let dh_dw = gs * (lat_d * sum + lat * sum_d);

    let dh_dx = dh_ds * fx + dh_dw * lx;
    let dh_dz = dh_ds * fz + dh_dw * lz;

    (height as f32, dh_dx as f32, dh_dz as f32)
}

/// Wake height above the flat water surface at `(x, z)`.
#[inline]
pub fn wake_height(node: &dyn WakeLike, x: f64, z: f64) -> f32 {
    wake_height_and_slope(node, x, z).0
}

/// Unit surface normal of the wake height field at `(x, z)`.
///
/// Flat water yields `[0, 1, 0]`. The returned vector points upward.
#[inline]
pub fn wake_normal(node: &dyn WakeLike, x: f64, z: f64) -> [f32; 3] {
    let (_, dh_dx, dh_dz) = wake_height_and_slope(node, x, z);
    let nx = -dh_dx;
    let ny = 1.0f32;
    let nz = -dh_dz;
    let inv_len = 1.0 / (nx * nx + ny * ny + nz * nz).sqrt();
    [nx * inv_len, ny * inv_len, nz * inv_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockWake {
        pos_x: f64,
        pos_z: f64,
        dir_x: f32,
        dir_z: f32,
        left_x: f32,
        left_z: f32,
        half_width: f32,
        amplitude: f32,
        age: f32,
        max_age: f32,
        is_boat: bool,
    }

    impl MockWake {
        fn new(boat: bool, half_width: f32, amplitude: f32, age: f32) -> Self {
            Self {
                pos_x: 0.0,
                pos_z: 0.0,
                // Heading toward +Z, left vector toward -X.
                dir_x: 0.0,
                dir_z: 1.0,
                left_x: -1.0,
                left_z: 0.0,
                half_width,
                amplitude,
                age,
                max_age: 3.5,
                is_boat: boat,
            }
        }
    }

    impl WakeLike for MockWake {
        fn pos_x(&self) -> f64 { self.pos_x }
        fn pos_z(&self) -> f64 { self.pos_z }
        fn dir_x(&self) -> f32 { self.dir_x }
        fn dir_z(&self) -> f32 { self.dir_z }
        fn left_x(&self) -> f32 { self.left_x }
        fn left_z(&self) -> f32 { self.left_z }
        fn half_width(&self) -> f32 { self.half_width }
        fn amplitude(&self) -> f32 { self.amplitude }
        fn age(&self) -> f32 { self.age }
        fn max_age(&self) -> f32 { self.max_age }
        fn is_boat(&self) -> bool { self.is_boat }
    }

    #[test]
    fn arms_are_symmetric_bow_wave_and_vanish_far_away() {
        let node = MockWake::new(true, 0.60, 1.0, 0.8);
        let left = wake_height(&node, -0.60, 0.0);
        let right = wake_height(&node, 0.60, 0.0);
        let keel = wake_height(&node, 0.0, 0.0);

        assert!((left - right).abs() < 1e-5, "V arms must be symmetric: {left} vs {right}");
        assert!(left > 0.02, "V-arm crest should be a raised ridge: {left}");
        assert!(left > keel, "V-arm crest should exceed the keel ridge: {left} vs {keel}");
        assert!(keel > 0.005, "boat keel ridge should be raised: {keel}");
        assert_eq!(wake_height(&node, 100.0, 100.0), 0.0);
        assert_eq!(wake_height(&node, 0.0, 5.0), 0.0, "no field far ahead or behind");
    }

    #[test]
    fn analytic_slope_matches_finite_difference() {
        let node = MockWake::new(true, 0.55, 1.2, 1.0);
        let (_, sx, sz) = wake_height_and_slope(&node, -0.48, 0.12);
        let e = 1e-3f64;
        let fd_x = (wake_height(&node, -0.48 + e, 0.12) - wake_height(&node, -0.48 - e, 0.12)) / (2.0 * e);
        let fd_z = (wake_height(&node, -0.48, 0.12 + e) - wake_height(&node, -0.48, 0.12 - e)) / (2.0 * e);

        assert!((sx as f64 - fd_x).abs() < 2e-3, "dh/dx mismatch: {sx} vs {fd_x}");
        assert!((sz as f64 - fd_z).abs() < 2e-3, "dh/dz mismatch: {sz} vs {fd_z}");

        let n = wake_normal(&node, -0.48, 0.12);
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-5, "normal must be unit length: {len}");
        assert!(n[1] > 0.0, "normal must point upward");
    }

    #[test]
    fn expired_and_zero_amplitude_nodes_are_flat() {
        let expired = MockWake::new(true, 0.6, 1.0, 3.5);
        assert_eq!(wake_height(&expired, -0.6, 0.0), 0.0);
        assert_eq!(wake_normal(&expired, -0.6, 0.0), [0.0, 1.0, 0.0]);

        let silent = MockWake::new(false, 0.4, 0.0, 0.5);
        assert_eq!(wake_height(&silent, -0.4, 0.0), 0.0);
        assert_eq!(wake_normal(&silent, -0.4, 0.0), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn boat_keel_is_stronger_than_player_keel() {
        let boat = MockWake::new(true, 0.8, 1.0, 0.5);
        let player = MockWake::new(false, 0.8, 1.0, 0.5);
        assert!(
            wake_height(&boat, 0.0, 0.0) > wake_height(&player, 0.0, 0.0),
            "boat stern wash should be taller than a player's step wake"
        );
    }
}

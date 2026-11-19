// w3_flow.rs - Flowing-water surface velocity and UV scroll field (water v4).
//
// The field is derived from one flowing-water block's flow level and the
// surface heights of its four horizontal neighbours. Water accelerates
// downhill, so the horizontal velocity is the negative height gradient scaled
// by a saturating function of the block flow level.
//
// Coordinate system: X/Z horizontal, Y up, 1 block = 1 m. A water surface is
// usually at `block_y + 8.0/9.0`; pass those surface heights (in Y) to this
// module. `origin_x`/`origin_z` are the minimum corner of the represented
// block, so the block centre is `(origin_x + 0.5, origin_z + 0.5)`.
//
// Neighbour order is fixed: `[north, east, south, west]` where north is -Z,
// east is +X, south is +Z and west is -X.
//
// Public API:
//   FlowField::new(flow_level, neighbor_heights)
//   FlowField::with_center(flow_level, center_height, neighbor_heights)
//   FlowField::at_block(block_x, block_z) / with_origin(...)
//   FlowField::sample(x, z) -> (vx, vz)       // blocks per second
//   FlowField::uv_offset(x, z, t) -> [u, v]   // texture repeat units
//
// Pure math: no globals, no allocation, no unsafe, no external crates.

/// Magnitude above which input heights are clamped, keeping differences finite.
const MAX_HEIGHT_MAGNITUDE: f32 = 1.0e6;
/// Denominator guard for the UV tile size.
const EPS: f32 = 1.0e-6;

#[inline]
fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[inline]
fn bounded_height(value: f32, fallback: f32) -> f32 {
    finite_or(value, fallback).clamp(-MAX_HEIGHT_MAGNITUDE, MAX_HEIGHT_MAGNITUDE)
}

#[inline]
fn mean_neighbor_height(heights: [f32; 4]) -> f32 {
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for h in heights {
        if h.is_finite() {
            sum += bounded_height(h, 0.0);
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

#[inline]
fn sanitized_heights(heights: [f32; 4], center: f32) -> [f32; 4] {
    let fallback = if center.is_finite() {
        bounded_height(center, 0.0)
    } else {
        mean_neighbor_height(heights)
    };
    let mut out = heights;
    for h in &mut out {
        *h = bounded_height(*h, fallback);
    }
    out
}

/// Smoothing curve for the discrete flow level. `0` stays still, higher levels
/// saturate towards one; this works for both 0..7 and 1..8 style metadata.
#[inline]
fn level_fraction(flow_level: u8) -> f32 {
    let level = flow_level as f32;
    if level <= 0.0 {
        0.0
    } else {
        level / (level + 2.0)
    }
}

/// Allocation-free velocity/UV field for one flowing-water block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowField {
    /// Discrete flow intensity from the water block (0 means no flow).
    pub flow_level: u8,
    /// Surface heights in blocks: `[north, east, south, west]`.
    pub neighbor_heights: [f32; 4],
    /// Surface height at the block centre. NaN selects a purely linear field.
    pub center_height: f32,
    /// Minimum corner of the represented block in world X.
    pub origin_x: f32,
    /// Minimum corner of the represented block in world Z.
    pub origin_z: f32,
    /// Slope-to-speed conversion in blocks per second per unit slope.
    pub speed_gain: f32,
    /// Upper bound on the emitted horizontal speed in blocks per second.
    pub max_speed: f32,
    /// World blocks covered by one UV repeat. Must be positive to take effect.
    pub uv_tile_blocks: f32,
}

impl FlowField {
    pub const DEFAULT_SPEED_GAIN: f32 = 1.0;
    pub const DEFAULT_MAX_SPEED: f32 = 2.0;
    pub const DEFAULT_UV_TILE_BLOCKS: f32 = 1.0;

    /// Build a linear downhill field from flow level and neighbour heights.
    #[inline]
    pub fn new(flow_level: u8, neighbor_heights: [f32; 4]) -> Self {
        Self {
            flow_level,
            neighbor_heights: sanitized_heights(neighbor_heights, f32::NAN),
            center_height: f32::NAN,
            origin_x: 0.0,
            origin_z: 0.0,
            speed_gain: Self::DEFAULT_SPEED_GAIN,
            max_speed: Self::DEFAULT_MAX_SPEED,
            uv_tile_blocks: Self::DEFAULT_UV_TILE_BLOCKS,
        }
    }

    /// Build a field whose gradient varies quadratically inside the block.
    ///
    /// Passing a non-finite `center_height` is equivalent to [`FlowField::new`].
    #[inline]
    pub fn with_center(flow_level: u8, center_height: f32, neighbor_heights: [f32; 4]) -> Self {
        let mut field = Self::new(flow_level, neighbor_heights);
        if center_height.is_finite() {
            field.center_height = bounded_height(center_height, 0.0);
            field.neighbor_heights = sanitized_heights(neighbor_heights, field.center_height);
        }
        field
    }

    /// Anchor the field to the minimum corner of block `(block_x, block_z)`.
    #[inline]
    pub fn at_block(mut self, block_x: i32, block_z: i32) -> Self {
        self.origin_x = block_x as f32;
        self.origin_z = block_z as f32;
        self
    }

    /// Anchor the field to an arbitrary minimum corner.
    #[inline]
    pub fn with_origin(mut self, origin_x: f32, origin_z: f32) -> Self {
        self.origin_x = finite_or(origin_x, 0.0);
        self.origin_z = finite_or(origin_z, 0.0);
        self
    }

    /// Horizontal surface velocity at world-space `(x, z)`, in blocks/second.
    ///
    /// Samples outside the block are clamped to its bounds, so the field stays
    /// continuous at neighbouring block edges. Non-finite inputs yield no flow.
    #[inline]
    pub fn sample(&self, x: f32, z: f32) -> (f32, f32) {
        if !x.is_finite() || !z.is_finite() {
            return (0.0, 0.0);
        }

        let strength = level_fraction(self.flow_level);
        if strength == 0.0 {
            return (0.0, 0.0);
        }

        let heights = sanitized_heights(self.neighbor_heights, self.center_height);
        let center = self.center_height;
        let ox = finite_or(self.origin_x, 0.0);
        let oz = finite_or(self.origin_z, 0.0);
        let u = (x - ox - 0.5).clamp(-0.5, 0.5);
        let v = (z - oz - 0.5).clamp(-0.5, 0.5);

        // Neighbour centres are one block from the block centre, hence /2.
        let mut grad_x = (heights[1] - heights[3]) * 0.5;
        let mut grad_z = (heights[2] - heights[0]) * 0.5;

        if center.is_finite() {
            let c = bounded_height(center, 0.0);
            grad_x += 2.0 * ((heights[1] + heights[3]) * 0.5 - c) * u;
            grad_z += 2.0 * ((heights[2] + heights[0]) * 0.5 - c) * v;
        }

        let gain = finite_or(self.speed_gain, 0.0).max(0.0);
        let scale = -strength * gain;
        let mut vx = grad_x * scale;
        let mut vz = grad_z * scale;
        let max_speed = finite_or(self.max_speed, 0.0).max(0.0);

        let speed2 = vx * vx + vz * vz;
        if speed2 > max_speed * max_speed {
            if max_speed <= 0.0 || !speed2.is_finite() {
                vx = 0.0;
                vz = 0.0;
            } else {
                let normalize = max_speed / speed2.sqrt();
                vx *= normalize;
                vz *= normalize;
            }
        }

        if !vx.is_finite() {
            vx = 0.0;
        }
        if !vz.is_finite() {
            vz = 0.0;
        }
        (vx, vz)
    }

    /// UV scroll offset after `t` seconds, in texture repeat units.
    ///
    /// The offset is `velocity * t / uv_tile_blocks`, so a tile size of one
    /// block scrolls one full UV repeat per block travelled. Non-finite time
    /// yields a zero offset.
    #[inline]
    pub fn uv_offset(&self, x: f32, z: f32, t: f32) -> [f32; 2] {
        if !t.is_finite() {
            return [0.0, 0.0];
        }
        let (vx, vz) = self.sample(x, z);
        let tile = finite_or(self.uv_tile_blocks, Self::DEFAULT_UV_TILE_BLOCKS)
            .abs()
            .max(EPS);
        let time = t.clamp(-1.0e6, 1.0e6);
        let mut u = vx * time / tile;
        let mut v = vz * time / tile;
        if !u.is_finite() {
            u = 0.0;
        }
        if !v.is_finite() {
            v = 0.0;
        }
        [u, v]
    }
}

impl Default for FlowField {
    #[inline]
    fn default() -> Self {
        Self::new(0, [0.0; 4])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flows_downhill_and_scales_with_flow_level() {
        // North=1, East=0, South=1, West=2: lowest side is +X (east).
        let heights = [1.0f32, 0.0, 1.0, 2.0];
        let fast = FlowField::new(7, heights);
        let slow = FlowField::new(1, heights);
        let still = FlowField::new(0, heights);

        let (vx, vz) = fast.sample(0.5, 0.5);
        assert!(vx > 0.0, "expected eastward flow, got {vx}");
        assert!(vz.abs() < 1.0e-6, "unexpected Z flow: {vz}");
        assert!(vx > slow.sample(0.5, 0.5).0, "flow level did not increase speed");
        assert_eq!(still.sample(0.5, 0.5), (0.0, 0.0));
    }

    #[test]
    fn center_height_curves_velocity_inside_the_block() {
        // West side has a steeper local gradient than the east side.
        let field = FlowField::with_center(7, 0.0, [1.0f32, 0.0, 1.0, 2.0]);
        let (vx_west, _) = field.sample(0.0, 0.5);
        let (vx_east, _) = field.sample(1.0, 0.5);
        assert!(vx_west > vx_east, "west {vx_west}, east {vx_east}");
        assert!(vx_east >= 0.0, "flow reversed unexpectedly: {vx_east}");

        // Without a centre height the field is linear across the block.
        let linear = FlowField::new(7, [1.0f32, 0.0, 1.0, 2.0]);
        assert!((linear.sample(0.0, 0.5).0 - linear.sample(1.0, 0.5).0).abs() < 1.0e-6);
    }

    #[test]
    fn uv_offset_is_velocity_times_time_over_tile_size() {
        let field = FlowField::new(4, [1.0f32, 0.0, 1.0, 2.0]);
        let (vx, vz) = field.sample(0.35, 0.65);
        let uv = field.uv_offset(0.35, 0.65, 2.0);
        assert!((uv[0] - vx * 2.0).abs() < 1.0e-6, "u was {}", uv[0]);
        assert!((uv[1] - vz * 2.0).abs() < 1.0e-6, "v was {}", uv[1]);
        assert_eq!(field.uv_offset(0.35, 0.65, 0.0), [0.0, 0.0]);

        let mut tiled = field;
        tiled.uv_tile_blocks = 2.0;
        let uv_tiled = tiled.uv_offset(0.35, 0.65, 2.0);
        assert!((uv_tiled[0] - vx).abs() < 1.0e-6);

        let mut degenerate = field;
        degenerate.uv_tile_blocks = 0.0;
        assert!(degenerate.uv_offset(0.35, 0.65, 1.0)[0].is_finite());
    }

    #[test]
    fn invalid_inputs_and_speed_clamp_stay_finite() {
        let field = FlowField::new(7, [f32::NAN, f32::INFINITY, 0.0, 5.0]);
        assert_eq!(field.sample(f32::NAN, 0.5), (0.0, 0.0));
        assert_eq!(field.sample(0.5, f32::INFINITY), (0.0, 0.0));

        let (vx, vz) = field.sample(0.5, 0.5);
        assert!(vx.is_finite() && vz.is_finite(), "invalid heights leaked: {vx}, {vz}");

        let mut clamped = field;
        clamped.max_speed = 0.1;
        let (cx, cz) = clamped.sample(0.5, 0.5);
        assert!((cx * cx + cz * cz).sqrt() <= 0.1 + 1.0e-6);

        clamped.max_speed = f32::NAN;
        assert_eq!(clamped.sample(0.5, 0.5), (0.0, 0.0));
        assert_eq!(field.uv_offset(0.5, 0.5, f32::NAN), [0.0, 0.0]);
    }

    #[test]
    fn block_anchor_controls_sampling_position() {
        let base = FlowField::with_center(7, 0.0, [1.0f32, 0.0, 1.0, 2.0]);
        let moved = base.at_block(4, -2);
        // The field at local (0, 0.5) in block (4, -2) is world (4, -1.5).
        assert_eq!(moved.sample(4.0, -1.5), base.sample(0.0, 0.5));
        // At the unanchored origin the moved field samples its clamped edge.
        assert_ne!(moved.sample(0.0, 0.5), base.sample(0.0, 0.5));
    }
}

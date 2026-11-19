// Adaptive surface-grid tessellation for water v4 (X/Z plane, Y up).
//
// Chooses a subdivision count from camera distance, maps normalized grid
// coordinates to world X/Z, and derives an axis-aligned visible patch.
// Pure f32 math, no allocation, no globals, no unsafe.
//
// Public API:
// * [`cells_for_distance`] - cell count from camera distance + quality.
// * [`grid_vertex`] - normalized grid vertex to world `(x, z)`.
// * [`visible_patch`] - visible X/Z bounds as `[min_x, min_z, max_x, max_z]`.

/// Minimum subdivision count (used for distant / low-quality water).
pub const MIN_CELLS: usize = 8;
/// Maximum subdivision count (used for near / high-quality water).
pub const MAX_CELLS: usize = 48;

/// Reference distance controlling the falloff curve.
const REF_DIST: f32 = 32.0;
/// Neutral quality multiplier.
const QUALITY_NEUTRAL: f32 = 1.0;

/// Choose a subdivision count for a water patch at camera distance `dist`.
///
/// Closer water gets more cells; higher `quality` also increases density.
/// The result is always clamped to [`MIN_CELLS`]..=[`MAX_CELLS`]. Non-finite
/// or negative distances fall back to the far-field minimum, and non-finite
/// quality falls back to 1.0.
#[inline]
pub fn cells_for_distance(dist: f32, quality: f32) -> usize {
    let d = if dist.is_finite() && dist >= 0.0 { dist } else { f32::INFINITY };
    let q = if quality.is_finite() && quality > 0.0 { quality } else { QUALITY_NEUTRAL };

    // t in (0, 1]: 1 when very close, ~0.5 at the reference distance.
    // Squaring concentrates detail near the camera while keeping a coarse
    // far field; quality scales only the detail added above the minimum.
    let t = REF_DIST / (d + REF_DIST);
    let extra = (MAX_CELLS - MIN_CELLS) as f32 * t * t;
    let cells_f = MIN_CELLS as f32 + extra * q;
    let cells = cells_f.round() as usize;
    cells.clamp(MIN_CELLS, MAX_CELLS)
}

/// Map a normalized grid vertex `(x, z)` in `[0, 1]` to world X/Z.
///
/// `cells` is the number of subdivisions and `size` is the full patch extent
/// in world units. The returned pair is `[world_x, world_z]`, suitable as the
/// horizontal part of a water-surface vertex. Inputs are not clamped so
/// callers may extrapolate; `size` is clamped to non-negative to avoid a
/// mirrored patch.
#[inline]
pub fn grid_vertex(x: f32, z: f32, cells: usize, size: f32) -> [f32; 2] {
    let n = cells.max(1) as f32;
    let extent = if size.is_finite() && size >= 0.0 { size } else { 0.0 };
    let wx = (x / n - 0.5) * extent;
    let wz = (z / n - 0.5) * extent;
    [wx, wz]
}

/// Axis-aligned square patch covering a circular visibility radius.
///
/// Returns `[min_x, min_z, max_x, max_z]`. Non-finite inputs are treated as
/// zero and a negative radius is treated as zero.
#[inline]
pub fn visible_patch(cam_x: f32, cam_z: f32, radius: f32) -> [f32; 4] {
    let cx = if cam_x.is_finite() { cam_x } else { 0.0 };
    let cz = if cam_z.is_finite() { cam_z } else { 0.0 };
    let r = if radius.is_finite() && radius >= 0.0 { radius } else { 0.0 };
    [cx - r, cz - r, cx + r, cz + r]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_clamped_and_monotonic() {
        assert_eq!(cells_for_distance(0.0, 1.0), MAX_CELLS);
        assert_eq!(cells_for_distance(f32::INFINITY, 1.0), MIN_CELLS);
        assert_eq!(cells_for_distance(10.0, 0.0), MIN_CELLS);

        let near = cells_for_distance(4.0, 1.0);
        let mid = cells_for_distance(24.0, 1.0);
        let far = cells_for_distance(120.0, 1.0);
        assert!(near > mid, "near should be denser: {near} <= {mid}");
        assert!(mid > far, "mid should be denser than far: {mid} <= {far}");
        assert!(mid > cells_for_distance(24.0, 0.5));
        assert!(mid < cells_for_distance(24.0, 2.0));

        for d in [f32::NAN, -5.0, 0.0, 1.0, 24.0, 1000.0, f32::INFINITY] {
            for q in [f32::NAN, -1.0, 0.0, 0.25, 1.0, 4.0] {
                let c = cells_for_distance(d, q);
                assert!((MIN_CELLS..=MAX_CELLS).contains(&c), "out of range: {c}");
            }
        }
    }

    #[test]
    fn grid_vertex_maps_corners_center_and_scales() {
        let four = grid_vertex(0.0, 0.0, 4, 16.0);
        assert_eq!(four, [-8.0, -8.0]);
        assert_eq!(grid_vertex(4.0, 4.0, 4, 16.0), [8.0, 8.0]);
        assert_eq!(grid_vertex(2.0, 2.0, 4, 16.0), [0.0, 0.0]);
        assert_eq!(grid_vertex(1.0, 3.0, 4, 16.0), [-4.0, 4.0]);

        let small = grid_vertex(1.0, 1.0, 2, 8.0);
        assert_eq!(small, [0.0, 0.0]);
        // Degenerate inputs stay finite and centered.
        assert_eq!(grid_vertex(0.0, 0.0, 0, 10.0), [-5.0, -5.0]);
        assert_eq!(grid_vertex(2.0, 2.0, 4, f32::NAN), [0.0, 0.0]);
    }

    #[test]
    fn visible_patch_centers_and_handles_bad_inputs() {
        assert_eq!(visible_patch(10.0, -4.0, 3.0), [7.0, -7.0, 13.0, -1.0]);
        assert_eq!(visible_patch(0.0, 0.0, 0.0), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(visible_patch(1.0, 2.0, -5.0), [1.0, 2.0, 1.0, 2.0]);
        assert_eq!(visible_patch(f32::NAN, 2.0, f32::INFINITY), [0.0, 2.0, 0.0, 2.0]);

        let p = visible_patch(-12.5, 30.0, 6.25);
        assert_eq!(p[0], -18.75);
        assert_eq!(p[1], 23.75);
        assert_eq!(p[2], -6.25);
        assert_eq!(p[3], 36.25);
    }
}

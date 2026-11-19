// Packed-color utilities and water palette blending for 3D water overlays.
//
// Biome water tints arrive as `0x00RRGGBB` (e.g. `0x3F76E4`) from
// `BiomeInfo::water`. This module unpacks them into linear-ish `[f32; 3]`
// floats, blends a small physically-motivated palette (deep, shallow, foam,
// crest), and repacks the result into the `ABGR` u32 layout expected by
// `horizon_client::mesher::Vertex::color`.
//
// Pure math, no allocations, no globals, no unsafe. Safe to call per vertex.

/// Unpack a `0x00RRGGBB` packed tint into normalized `[r, g, b]` floats in
/// `0.0..=1.0`. Higher bytes beyond 24 bits are ignored.
#[inline]
pub fn unpack_rgb(tint: u32) -> [f32; 3] {
    [
        ((tint >> 16) & 0xFF) as f32 * (1.0 / 255.0),
        ((tint >> 8) & 0xFF) as f32 * (1.0 / 255.0),
        (tint & 0xFF) as f32 * (1.0 / 255.0),
    ]
}

/// Clamp a scalar to `0.0..=1.0` without pulling in `f32::clamp` overhead.
#[inline]
fn sat(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

/// Component-wise linear interpolation between two RGB triples.
#[inline]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = sat(t);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Blend the water palette from a biome tint and three blend factors.
///
/// * `depth`  - `0.0` at the shoreline (shallow), `1.0` in deep water.
/// * `foam`   - `0.0..=1.0` shoreline / turbulence foam coverage.
/// * `crest`  - `0.0..=1.0` wave-crest brightness contribution.
///
/// Returns normalized `[r, g, b]` floats in `0.0..=1.0`.
/// All inputs are clamped to their valid ranges, so callers may pass raw
/// untrusted values.
#[inline]
pub fn blend_water(tint: u32, depth: f32, foam: f32, crest: f32) -> [f32; 3] {
    let base = unpack_rgb(tint);
    let depth = sat(depth);
    let foam = sat(foam);
    let crest = sat(crest);

    // Deep water: darken and slightly cool the biome tint. Absorption removes
    // red first, then green, leaving the blue channel relatively stronger.
    let deep = [
        base[0] * 0.42,
        base[1] * 0.58,
        (base[2] * 0.88 + 0.04).min(1.0),
    ];

    // Shallow water: brighten and warm toward a turquoise shoal colour so the
    // seabed reads through the surface near the shoreline.
    let shallow = [
        (base[0] * 0.72 + 0.18).min(1.0),
        (base[1] * 0.78 + 0.20).min(1.0),
        (base[2] * 0.70 + 0.16).min(1.0),
    ];

    // Body colour: shallow -> deep as depth increases.
    let body = lerp3(shallow, deep, depth);

    // Crest: lift all channels toward a bright, slightly cool highlight.
    let crest_col = [
        (base[0] * 0.55 + 0.45).min(1.0),
        (base[1] * 0.55 + 0.47).min(1.0),
        (base[2] * 0.55 + 0.50).min(1.0),
    ];
    let with_crest = lerp3(body, crest_col, crest * 0.65);

    // Foam: near-white with a faint cast of the biome tint, blended last so it
    // always dominates where coverage is high.
    let foam_col = [
        (0.90 + base[0] * 0.10).min(1.0),
        (0.93 + base[1] * 0.07).min(1.0),
        (0.96 + base[2] * 0.04).min(1.0),
    ];
    lerp3(with_crest, foam_col, foam)
}

/// Pack normalized `[r, g, b]` floats and an alpha into the `ABGR` u32 layout
/// used by `horizon_client::mesher::Vertex::color`.
///
/// Components are clamped to `0.0..=1.0` and rounded to the nearest byte.
#[inline]
pub fn to_abgr(rgb: [f32; 3], a: f32) -> u32 {
    let byte = |v: f32| -> u32 { (sat(v) * 255.0 + 0.5) as u32 };
    let r = byte(rgb[0]);
    let g = byte(rgb[1]);
    let b = byte(rgb[2]);
    let a = byte(a);
    (a << 24) | (b << 16) | (g << 8) | r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_rgb_extracts_channels() {
        assert_eq!(unpack_rgb(0x3F76E4), [
            0x3F as f32 / 255.0,
            0x76 as f32 / 255.0,
            0xE4 as f32 / 255.0,
        ]);
        assert_eq!(unpack_rgb(0xFFFFFF), [1.0, 1.0, 1.0]);
        assert_eq!(unpack_rgb(0x000000), [0.0, 0.0, 0.0]);
        // Ignore the alpha/high byte.
        assert_eq!(unpack_rgb(0xFF3F_76E4), unpack_rgb(0x003F_76E4));
    }

    #[test]
    fn to_abgr_packs_bytes_in_vertex_order() {
        assert_eq!(to_abgr([1.0, 0.0, 0.0], 1.0), 0xFF00_00FF);
        assert_eq!(to_abgr([0.0, 1.0, 0.0], 0.0), 0x0000_FF00);
        assert_eq!(to_abgr([0.0, 0.0, 1.0], 0.5), 0x80FF_0000);
        // Out-of-range values clamp.
        assert_eq!(to_abgr([-1.0, 2.0, 0.5], -1.0), 0x0080_FF00);
    }

    #[test]
    fn blend_water_deep_is_darker_than_shallow() {
        let tint = 0x3F76E4;
        let deep = blend_water(tint, 1.0, 0.0, 0.0);
        let shallow = blend_water(tint, 0.0, 0.0, 0.0);
        let lum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(lum(deep) < lum(shallow), "deep should be darker than shallow");
        // Channels stay in range.
        for c in deep.into_iter().chain(shallow) {
            assert!((0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn blend_water_foam_and_crest_brighten() {
        let tint = 0x3F76E4;
        let body = blend_water(tint, 0.5, 0.0, 0.0);
        let crest = blend_water(tint, 0.5, 0.0, 1.0);
        let foam = blend_water(tint, 0.5, 1.0, 0.0);
        let lum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(lum(crest) > lum(body));
        assert!(lum(foam) > lum(body));
        assert!(lum(foam) >= lum(crest) * 0.9, "foam should be near-white");
    }

    #[test]
    fn blend_water_clamps_inputs() {
        let a = blend_water(0x3F76E4, 0.5, 0.2, 0.1);
        let b = blend_water(0x3F76E4, -5.0, 99.0, -99.0);
        let c = blend_water(0x3F76E4, 5.0, -5.0, 99.0);
        for i in 0..3 {
            assert!(a[i].is_finite() && (0.0..=1.0).contains(&a[i]));
            assert!(b[i].is_finite() && (0.0..=1.0).contains(&b[i]));
            assert!(c[i].is_finite() && (0.0..=1.0).contains(&c[i]));
        }
    }
}

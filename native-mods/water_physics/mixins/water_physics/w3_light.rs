//! Vertex-light helpers for 3D water overlays.
//!
//! Packs shaded water colours into the `ABGR` `u32` layout expected by
//! `horizon_client::mesher::Vertex::color`. The module is pure math: no crates,
//! no allocations, no globals, no unsafe, safe to call per vertex.
//!
//! Light sources are combined as follows:
//! * block light and sky light are unpacked from the `u32` produced by
//!   `horizon_client::block_entity_render::vertex_light(packed_byte)`;
//! * sky light is boosted by Schlick Fresnel at grazing view angles;
//! * wave-normal diffuse uses the Minecraft-style `0.4 + 0.6 * ndl` curve;
//! * [`shade`] turns the combined ambient factor plus diffuse into a packed colour.
//!
//! Typical use from a renderer:
//! ```ignore
//! let ambient = ambient_light(vertex_light_u32, 0.85, view_dot);
//! let color = shade(base_rgb, normal_dot_sun, ambient, alpha);
//! ```
//! Or in one call: [`shade_light`].

/// Minimum diffuse contribution on an unlit facing, matching vanilla water.
const DIFFUSE_MIN: f32 = 0.4;
/// Small floor so fully dark water is not a pure black silhouette.
const AMBIENT_MIN: f32 = 0.10;
/// Water's normal-incidence Fresnel reflectance (IOR ~= 1.333).
const FRESNEL_F0: f32 = 0.02;

/// Clamp to `0.0..=1.0`, mapping NaN to `0.0` for deterministic output.
#[inline]
fn sat(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

/// Pack normalized `r, g, b, a` channels into a vertex `ABGR` `u32`.
///
/// Inputs are clamped to `0.0..=1.0` and rounded to the nearest byte.
#[inline]
pub fn pack_abgr(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let byte = |v: f32| -> u32 { (sat(v) * 255.0 + 0.5) as u32 };
    let r = byte(r);
    let g = byte(g);
    let b = byte(b);
    let a = byte(a);
    (a << 24) | (b << 16) | (g << 8) | r
}

/// Unpack a raw world-light byte (`sky << 4 | block`) into normalized
/// `(sky, block)` levels.
#[inline]
pub fn world_light_levels(packed: u8) -> (f32, f32) {
    let sky = (packed >> 4) as f32 * (1.0 / 15.0);
    let block = (packed & 0x0F) as f32 * (1.0 / 15.0);
    (sky, block)
}

/// Unpack the `u32` returned by `block_entity_render::vertex_light` into
/// normalized `(sky, block)` levels.
///
/// That layout stores sky in the low byte and block in the next byte, each as
/// nibble value multiplied by 16, so the maximum channel value is 240.
#[inline]
pub fn vertex_light_levels(packed: u32) -> (f32, f32) {
    let sky = (packed & 0xFF) as f32 * (1.0 / 240.0);
    let block = ((packed >> 8) & 0xFF) as f32 * (1.0 / 240.0);
    (sat(sky), sat(block))
}

/// Minecraft-style diffuse term from a wave normal dotted with the light dir.
///
/// `ndl` is `dot(normal, light_dir)` and may be negative; backfaces retain a
/// `0.4` ambient floor instead of going black.
#[inline]
pub fn diffuse(ndl: f32) -> f32 {
    DIFFUSE_MIN + (1.0 - DIFFUSE_MIN) * sat(ndl)
}

/// Schlick Fresnel reflectance for water.
///
/// `cos_theta` is the absolute cosine between the surface normal and the view
/// direction. Grazing angles (`cos_theta` near `0`) approach full reflectance.
#[inline]
pub fn fresnel(cos_theta: f32) -> f32 {
    let inv = 1.0 - sat(cos_theta);
    let inv2 = inv * inv;
    FRESNEL_F0 + (1.0 - FRESNEL_F0) * inv2 * inv2 * inv2
}

/// Combine block light, sky light and Fresnel into an ambient factor for [`shade`].
///
/// * `packed_light` is the `u32` from `block_entity_render::vertex_light`.
/// * `sky_strength` scales daylight contribution (`0..=1`, e.g. world sun factor).
/// * `cos_theta` is the view/normal cosine used by [`fresnel`].
#[inline]
pub fn ambient_light(packed_light: u32, sky_strength: f32, cos_theta: f32) -> f32 {
    let (sky, block) = vertex_light_levels(packed_light);
    let f = fresnel(cos_theta);

    // Grazing angles lift reflected skylight, while block light remains the
    // dominant local term in shelters and caves.
    let sky_term = sky * sat(sky_strength) * (1.0 + 0.6 * f);
    let lit = block.max(sky_term);
    let reflect_lift = f * 0.08 * (1.0 - sat(lit));
    AMBIENT_MIN + (1.0 - AMBIENT_MIN) * sat(lit + reflect_lift)
}

/// Shade normalized `base` RGB by wave-normal diffuse and ambient light into
/// a packed vertex `ABGR` colour.
///
/// `base` components are expected in `0.0..=1.0`; `ambient` is an environment
/// multiplier in `0.0..=1.0`, typically from [`ambient_light`]. All inputs are
/// clamped, so callers may pass raw untrusted values.
#[inline]
pub fn shade(base: [f32; 3], ndl: f32, ambient: f32, alpha: f32) -> u32 {
    let light = sat(ambient) * diffuse(ndl);
    pack_abgr(
        base[0] * light,
        base[1] * light,
        base[2] * light,
        alpha,
    )
}

/// Convenience wrapper: combine world light, diffuse and Fresnel, then pack.
#[inline]
pub fn shade_light(
    base: [f32; 3],
    packed_light: u32,
    ndl: f32,
    cos_theta: f32,
    sky_strength: f32,
    alpha: f32,
) -> u32 {
    let ambient = ambient_light(packed_light, sky_strength, cos_theta);
    shade(base, ndl, ambient, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-5
    }

    #[test]
    fn pack_abgr_uses_vertex_byte_order_and_clamps() {
        assert_eq!(pack_abgr(1.0, 0.0, 0.0, 1.0), 0xFF00_00FF);
        assert_eq!(pack_abgr(0.0, 1.0, 0.0, 0.0), 0x0000_FF00);
        assert_eq!(pack_abgr(0.0, 0.0, 1.0, 0.5), 0x80FF_0000);
        // Out-of-range and NaN inputs clamp deterministically.
        assert_eq!(pack_abgr(-1.0, 2.0, f32::NAN, -2.0), 0x0000_FF00);
    }

    #[test]
    fn unpack_light_levels_match_world_byte_and_vertex_format() {
        // world byte: high nibble sky = 15, low nibble block = 8.
        let (sky, block) = world_light_levels(0xF8);
        assert!(approx(sky, 1.0));
        assert!(approx(block, 8.0 / 15.0));
        // vertex_light(0xF8): sky = 15*16 = 240 in low byte,
        // block = 8*16 = 128 in the next byte.
        const VERTEX_F8: u32 = 240 | (128 << 8);
        let (sky, block) = vertex_light_levels(VERTEX_F8);
        assert!(approx(sky, 1.0));
        assert!(approx(block, 8.0 / 15.0));
        // Out-of-range/stale bytes are clamped.
        assert_eq!(vertex_light_levels(0xFFFF_FFFF), (1.0, 1.0));
    }

    #[test]
    fn diffuse_and_fresnel_have_expected_endpoints() {
        assert!(approx(diffuse(1.0), 1.0));
        assert!(approx(diffuse(0.0), DIFFUSE_MIN));
        assert!(approx(diffuse(-3.0), DIFFUSE_MIN));
        assert!(approx(fresnel(1.0), FRESNEL_F0));
        assert!(approx(fresnel(0.0), 1.0));
        assert!(fresnel(0.1) > fresnel(0.5));
        assert!(fresnel(0.5) > fresnel(1.0));
    }

    #[test]
    fn shade_scales_ambient_and_diffuse_and_preserves_alpha() {
        let base = [0.5, 0.25, 1.0];

        let bright = shade(base, 1.0, 1.0, 0.75);
        let ambient_only = shade(base, 0.0, 1.0, 0.75);
        let dim = shade(base, 1.0, 0.5, 0.75);

        // Full sun + full ambient reproduces the base colour exactly.
        assert_eq!(bright, pack_abgr(0.5, 0.25, 1.0, 0.75));
        // Facing away from the sun keeps the 0.4 diffuse floor, so it is darker.
        let bright_r = bright & 0xFF;
        let ambient_r = ambient_only & 0xFF;
        assert!(ambient_r < bright_r);
        assert_eq!(ambient_only >> 24, bright >> 24);
        // Halving ambient darkens the result, leaving alpha untouched.
        assert!(dim & 0xFF < bright_r);
        assert_eq!(dim >> 24, bright >> 24);
    }

    #[test]
    fn shade_light_combines_light_sources_and_grazing_reflection() {
        let base = [0.8, 0.9, 1.0];
        let dark = 0x0000_0000u32;
        let half_sky = 120u32; // sky = 7.5/15 in vertex_light layout
        let full_block = 240u32 << 8; // sky = 0, block = 15

        let dark_c = shade_light(base, dark, 1.0, 1.0, 1.0, 1.0);
        let sky_c = shade_light(base, half_sky, 1.0, 1.0, 1.0, 1.0);
        let block_c = shade_light(base, full_block, 1.0, 1.0, 1.0, 1.0);
        let grazing_c = shade_light(base, half_sky, 1.0, 0.0, 1.0, 1.0);

        assert!(sky_c > dark_c, "sky light should brighten water");
        assert!(block_c > dark_c, "block light should brighten water");
        assert!(
            grazing_c > sky_c,
            "grazing Fresnel should lift reflected skylight"
        );
        assert_eq!(dark_c >> 24, 0xFF);
    }
}

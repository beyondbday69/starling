// AMD FidelityFX Super Resolution 1.0 (FSR 1.0)
// Edge-Adaptive Spatial Upsampling (EASU) & Robust Contrast-Adaptive Sharpening (RCAS)
// Implemented in WebGPU Shading Language (WGSL) for WGPU / Vulkan mobile & desktop renderers.

struct FsrConstants {
    src_size: vec2<f32>,
    dst_size: vec2<f32>,
    sharpness: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> constants: FsrConstants;
@group(0) @binding(1) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var input_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

/// Full-screen triangle covering [-1, 1] in NDC with UV spanning [0, 1].
/// Screen top (NDC y = +1.0) maps to uv.y = 0.0, and NDC bottom (y = -1.0) maps to uv.y = 1.0,
/// correctly aligning with WGPU / Vulkan texture coordinate conventions.
@vertex
fn vs_fsr(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

/// Computes fast perceptual luminance for gradient analysis:
/// AMD FSR fast luma weights green as 1.0, red and blue as 0.5.
fn fsr_luma(color: vec3<f32>) -> f32 {
    return color.g + 0.5 * (color.r + color.b);
}

/// Fetches a source texel at integer coordinates, clamping within [0, dims - 1] and sanitizing NaNs.
fn fetch_src(coord: vec2<i32>) -> vec4<f32> {
    let dims = vec2<i32>(textureDimensions(input_texture));
    let max_coord = max(dims - vec2<i32>(1, 1), vec2<i32>(0, 0));
    let clamped = clamp(coord, vec2<i32>(0, 0), max_coord);
    let c = textureLoad(input_texture, clamped, 0);
    let safe_rgb = select(vec3<f32>(0.0), c.rgb, c.rgb == c.rgb);
    return vec4<f32>(max(safe_rgb, vec3<f32>(0.0)), clamp(c.a, 0.0, 1.0));
}

/// Fetches an intermediate texel at integer coordinates for RCAS, clamping within [0, dims - 1].
fn fetch_rcas(coord: vec2<i32>) -> vec4<f32> {
    return fetch_src(coord);
}

/// Evaluates a 2-lobed Lanczos-like reconstruction filter weight for a given tap offset.
fn easu_tap_weight(
    off: vec2<f32>,
    dir: vec2<f32>,
    len2: vec2<f32>,
    lob: f32,
    clp: f32,
) -> f32 {
    // Rotate offset by edge gradient direction
    let v = vec2<f32>(
        off.x * dir.x + off.y * dir.y,
        off.x * (-dir.y) + off.y * dir.x,
    ) * len2;

    // Distance squared clamped to filter window
    let d2 = min(v.x * v.x + v.y * v.y, clp);

    // Approximate 2-lobed Lanczos window function
    var wB = (2.0 / 5.0) * d2 - 1.0;
    var wA = lob * d2 - 1.0;
    wB = wB * wB;
    wA = wA * wA;
    wB = (25.0 / 16.0) * wB - (25.0 / 16.0 - 1.0);
    return wB * wA;
}

/// FSR 1.0 EASU (Edge-Adaptive Spatial Upsampling) Fragment Pass
/// Performs 12-tap edge-adaptive spatial reconstruction using directional gradient analysis.
@fragment
fn fs_easu(in: VertexOutput) -> @location(0) vec4<f32> {
    // Map current UV to input texture pixel coordinates
    let src_dims = vec2<f32>(textureDimensions(input_texture));
    let src_pos = in.uv * src_dims - vec2<f32>(0.5, 0.5);
    let base = vec2<i32>(floor(src_pos));
    let pp = src_pos - vec2<f32>(base);

    // 12-tap sampling pattern around base 2x2 quad:
    //       b c
    //     d e f g
    //     h i j k
    //       l m
    let b = fetch_src(base + vec2<i32>(0, -1));
    let c = fetch_src(base + vec2<i32>(1, -1));

    let d = fetch_src(base + vec2<i32>(-1, 0));
    let e = fetch_src(base + vec2<i32>(0, 0));
    let f = fetch_src(base + vec2<i32>(1, 0));
    let g = fetch_src(base + vec2<i32>(2, 0));

    let h = fetch_src(base + vec2<i32>(-1, 1));
    let i_tap = fetch_src(base + vec2<i32>(0, 1));
    let j = fetch_src(base + vec2<i32>(1, 1));
    let k = fetch_src(base + vec2<i32>(2, 1));

    let l = fetch_src(base + vec2<i32>(0, 2));
    let m = fetch_src(base + vec2<i32>(1, 2));

    // Compute luminance for directional gradient analysis
    let b_l = fsr_luma(b.rgb);
    let c_l = fsr_luma(c.rgb);
    let d_l = fsr_luma(d.rgb);
    let e_l = fsr_luma(e.rgb);
    let f_l = fsr_luma(f.rgb);
    let g_l = fsr_luma(g.rgb);
    let h_l = fsr_luma(h.rgb);
    let i_l = fsr_luma(i_tap.rgb);
    let j_l = fsr_luma(j.rgb);
    let k_l = fsr_luma(k.rgb);
    let l_l = fsr_luma(l.rgb);
    let m_l = fsr_luma(m.rgb);

    var dir = vec2<f32>(0.0, 0.0);
    var len = 0.0;

    // --- Quad 0: around e (top-left) ---
    {
        let w = (1.0 - pp.x) * (1.0 - pp.y);
        let dc = f_l - e_l;
        let cb = e_l - d_l;
        var lenX = max(abs(dc), abs(cb));
        if (lenX > 0.0) { lenX = 1.0 / lenX; }
        let dirX = f_l - d_l;
        dir.x += dirX * w;
        lenX = clamp(abs(dirX) * lenX, 0.0, 1.0);
        len += lenX * lenX * w;

        let ec = i_l - e_l;
        let ca = e_l - b_l;
        var lenY = max(abs(ec), abs(ca));
        if (lenY > 0.0) { lenY = 1.0 / lenY; }
        let dirY = i_l - b_l;
        dir.y += dirY * w;
        lenY = clamp(abs(dirY) * lenY, 0.0, 1.0);
        len += lenY * lenY * w;
    }

    // --- Quad 1: around f (top-right) ---
    {
        let w = pp.x * (1.0 - pp.y);
        let dc = g_l - f_l;
        let cb = f_l - e_l;
        var lenX = max(abs(dc), abs(cb));
        if (lenX > 0.0) { lenX = 1.0 / lenX; }
        let dirX = g_l - e_l;
        dir.x += dirX * w;
        lenX = clamp(abs(dirX) * lenX, 0.0, 1.0);
        len += lenX * lenX * w;

        let ec = j_l - f_l;
        let ca = f_l - c_l;
        var lenY = max(abs(ec), abs(ca));
        if (lenY > 0.0) { lenY = 1.0 / lenY; }
        let dirY = j_l - c_l;
        dir.y += dirY * w;
        lenY = clamp(abs(dirY) * lenY, 0.0, 1.0);
        len += lenY * lenY * w;
    }

    // --- Quad 2: around i (bottom-left) ---
    {
        let w = (1.0 - pp.x) * pp.y;
        let dc = j_l - i_l;
        let cb = i_l - h_l;
        var lenX = max(abs(dc), abs(cb));
        if (lenX > 0.0) { lenX = 1.0 / lenX; }
        let dirX = j_l - h_l;
        dir.x += dirX * w;
        lenX = clamp(abs(dirX) * lenX, 0.0, 1.0);
        len += lenX * lenX * w;

        let ec = l_l - i_l;
        let ca = i_l - e_l;
        var lenY = max(abs(ec), abs(ca));
        if (lenY > 0.0) { lenY = 1.0 / lenY; }
        let dirY = l_l - e_l;
        dir.y += dirY * w;
        lenY = clamp(abs(dirY) * lenY, 0.0, 1.0);
        len += lenY * lenY * w;
    }

    // --- Quad 3: around j (bottom-right) ---
    {
        let w = pp.x * pp.y;
        let dc = k_l - j_l;
        let cb = j_l - i_l;
        var lenX = max(abs(dc), abs(cb));
        if (lenX > 0.0) { lenX = 1.0 / lenX; }
        let dirX = k_l - i_l;
        dir.x += dirX * w;
        lenX = clamp(abs(dirX) * lenX, 0.0, 1.0);
        len += lenX * lenX * w;

        let ec = m_l - j_l;
        let ca = j_l - f_l;
        var lenY = max(abs(ec), abs(ca));
        if (lenY > 0.0) { lenY = 1.0 / lenY; }
        let dirY = m_l - f_l;
        dir.y += dirY * w;
        lenY = clamp(abs(dirY) * lenY, 0.0, 1.0);
        len += lenY * lenY * w;
    }

    // Normalize edge direction and calculate anisotropic stretch
    let dir2 = dir * dir;
    let dir_sq = dir2.x + dir2.y;
    var dir_norm = vec2<f32>(0.0, 1.0);
    if (dir_sq > 0.00001) {
        dir_norm = dir * inverseSqrt(dir_sq);
    }

    var edge_len = len * 0.5;
    edge_len = edge_len * edge_len;

    let max_dir = max(abs(dir_norm.x), abs(dir_norm.y));
    var stretch = 1.0;
    if (max_dir > 0.0) {
        stretch = (dir_norm.x * dir_norm.x + dir_norm.y * dir_norm.y) / max_dir;
    }

    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * edge_len, 1.0 - 0.5 * edge_len);
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * edge_len;
    let clp = 1.0 / lob;

    // Accumulate weighted 12-tap samples
    var aC = vec3<f32>(0.0, 0.0, 0.0);
    var aA = 0.0;
    var aW = 0.0;

    let wb = easu_tap_weight(vec2<f32>(0.0, -1.0) - pp, dir_norm, len2, lob, clp);
    aC += b.rgb * wb; aA += b.a * wb; aW += wb;

    let wc = easu_tap_weight(vec2<f32>(1.0, -1.0) - pp, dir_norm, len2, lob, clp);
    aC += c.rgb * wc; aA += c.a * wc; aW += wc;

    let wd = easu_tap_weight(vec2<f32>(-1.0, 0.0) - pp, dir_norm, len2, lob, clp);
    aC += d.rgb * wd; aA += d.a * wd; aW += wd;

    let we = easu_tap_weight(vec2<f32>(0.0, 0.0) - pp, dir_norm, len2, lob, clp);
    aC += e.rgb * we; aA += e.a * we; aW += we;

    let wf = easu_tap_weight(vec2<f32>(1.0, 0.0) - pp, dir_norm, len2, lob, clp);
    aC += f.rgb * wf; aA += f.a * wf; aW += wf;

    let wg = easu_tap_weight(vec2<f32>(2.0, 0.0) - pp, dir_norm, len2, lob, clp);
    aC += g.rgb * wg; aA += g.a * wg; aW += wg;

    let wh = easu_tap_weight(vec2<f32>(-1.0, 1.0) - pp, dir_norm, len2, lob, clp);
    aC += h.rgb * wh; aA += h.a * wh; aW += wh;

    let wi = easu_tap_weight(vec2<f32>(0.0, 1.0) - pp, dir_norm, len2, lob, clp);
    aC += i_tap.rgb * wi; aA += i_tap.a * wi; aW += wi;

    let wj = easu_tap_weight(vec2<f32>(1.0, 1.0) - pp, dir_norm, len2, lob, clp);
    aC += j.rgb * wj; aA += j.a * wj; aW += wj;

    let wk = easu_tap_weight(vec2<f32>(2.0, 1.0) - pp, dir_norm, len2, lob, clp);
    aC += k.rgb * wk; aA += k.a * wk; aW += wk;

    let wl = easu_tap_weight(vec2<f32>(0.0, 2.0) - pp, dir_norm, len2, lob, clp);
    aC += l.rgb * wl; aA += l.a * wl; aW += wl;

    let wm = easu_tap_weight(vec2<f32>(1.0, 2.0) - pp, dir_norm, len2, lob, clp);
    aC += m.rgb * wm; aA += m.a * wm; aW += wm;

    var out_rgb = aC;
    var out_a = aA;
    if (abs(aW) > 0.0001) {
        out_rgb = aC / aW;
        out_a = aA / aW;
    } else {
        out_rgb = e.rgb;
        out_a = e.a;
    }

    // Ensure non-negative colors and bounded alpha
    out_rgb = max(out_rgb, vec3<f32>(0.0, 0.0, 0.0));
    out_a = clamp(out_a, 0.0, 1.0);

    return vec4<f32>(out_rgb, out_a);
}

/// FSR 1.0 RCAS (Robust Contrast-Adaptive Sharpening) Fragment Pass
/// 5-tap neighborhood contrast calculation with min/max clamping to avoid haloing/ringing.
@fragment
fn fs_rcas(in: VertexOutput) -> @location(0) vec4<f32> {
    let ip = vec2<i32>(floor(in.position.xy));

    // Sample 5-tap cross pattern:
    //      b
    //    d e f
    //      h
    let b = fetch_rcas(ip + vec2<i32>(0, -1));
    let d = fetch_rcas(ip + vec2<i32>(-1, 0));
    let e = fetch_rcas(ip);
    let f = fetch_rcas(ip + vec2<i32>(1, 0));
    let h = fetch_rcas(ip + vec2<i32>(0, 1));

    // Neighborhood min and max across 4-neighbors
    let mn4 = min(min(b.rgb, d.rgb), min(f.rgb, h.rgb));
    let mx4 = max(max(b.rgb, d.rgb), max(f.rgb, h.rgb));

    // Contrast limiter calculation (AMD FSR 1.0 RCAS)
    let peakC = vec2<f32>(1.0, -4.0);
    let hitMin = mn4 / max(4.0 * mx4, vec3<f32>(1e-5, 1e-5, 1e-5));
    let denom = 4.0 * mn4 + vec3<f32>(peakC.y, peakC.y, peakC.y);
    let hitMax = (vec3<f32>(peakC.x, peakC.x, peakC.x) - mx4) / min(denom, vec3<f32>(-1e-5, -1e-5, -1e-5));
    let lobeRGB = max(-hitMin, hitMax);

    let max_lobe = max(lobeRGB.r, max(lobeRGB.g, lobeRGB.b));
    let rcas_limit = 0.1875; // 0.25 - 1.0/16.0

    // Sharpness control: 0.0 = no sharpening, 1.0 = maximum sharpening
    let con = clamp(constants.sharpness, 0.0, 1.0);
    let lobe = max(-rcas_limit, min(max_lobe, 0.0)) * con;

    // Apply sharpening filter with adaptive negative lobe
    let sharpened = (lobe * (b.rgb + d.rgb + f.rgb + h.rgb) + e.rgb) / (4.0 * lobe + 1.0);

    // Min/max clamping against 5-tap neighborhood to eliminate ringing artifacts and halos
    let min_color = min(mn4, e.rgb);
    let max_color = max(mx4, e.rgb);
    let final_rgb = clamp(sharpened, min_color, max_color);

    return vec4<f32>(final_rgb, clamp(e.a, 0.0, 1.0));
}

/// Integer scaling fragment shader:
/// Computes integer scale factor = floor(min(dst_w / src_w, dst_h / src_h)).
/// Centers the scaled image with clean letterbox/pillarbox borders.
/// Nearest-neighbor loads source texels using the exact same coordinate space as FSR.
@fragment
fn fs_integer(in: VertexOutput) -> @location(0) vec4<f32> {
    let dst_pixel = in.position.xy;
    let scale = floor(min(constants.dst_size.x / constants.src_size.x, constants.dst_size.y / constants.src_size.y));
    let factor = max(scale, 1.0);
    let target_size = constants.src_size * factor;
    let offset = floor((constants.dst_size - target_size) * 0.5);

    if (dst_pixel.x < offset.x || dst_pixel.x >= offset.x + target_size.x ||
        dst_pixel.y < offset.y || dst_pixel.y >= offset.y + target_size.y) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let src_pos = vec2<i32>(floor((dst_pixel - offset) / factor));
    let max_coord = vec2<i32>(constants.src_size) - vec2<i32>(1, 1);
    let clamped = clamp(src_pos, vec2<i32>(0, 0), max(max_coord, vec2<i32>(0, 0)));
    return textureLoad(input_texture, clamped, 0);
}

/// Qualcomm SGSR v1 fast 2-lobe Lanczos polynomial kernel.
/// Zero-crossings at d2 = 1.0 and d2 = 4.0 (clamped at 4.0).
fn sgsr_fast_lanczos2(d2: f32) -> f32 {
    var wA = d2 - 4.0;
    let wB = d2 * wA - wA;
    wA = wA * wA;
    return wB * wA;
}

/// Evaluates SGSR v1 edge-directed weight for a tap at offset `off` relative to `pp`.
fn sgsr_weight(
    off: vec2<f32>,
    pp: vec2<f32>,
    dir: vec2<f32>,
    edge_strength: f32,
) -> f32 {
    let delta = off - pp;
    // Rotate/stretch along detected edge direction for crisp diagonal and straight lines
    let proj = dot(delta, dir);
    let perp = delta - dir * proj;
    let stretch = mix(1.0, 0.5, edge_strength);
    let scaled = dir * (proj * stretch) + perp * mix(1.0, 1.35, edge_strength);
    let d2 = min(dot(scaled, scaled), 4.0);
    return sgsr_fast_lanczos2(d2);
}

/// Qualcomm Snapdragon Game Super Resolution v1 (SGSR v1) single-pass spatial upscaler.
/// Edge-directed 12-tap FastLanczos2 reconstruction + anti-ringing clamp in 1 pass,
/// tuned for Adreno 7xx and Mali TBDR mobile GPUs.
@fragment
fn fs_sgsr(in: VertexOutput) -> @location(0) vec4<f32> {
    let src_dims = vec2<f32>(textureDimensions(input_texture));
    let src_pos = in.uv * src_dims - vec2<f32>(0.5, 0.5);
    let base = vec2<i32>(floor(src_pos));
    let pp = src_pos - vec2<f32>(base);

    // 12-tap neighborhood around base 2x2 quad:
    //       b c
    //     d e f g
    //     h i j k
    //       l m
    let b = fetch_src(base + vec2<i32>(0, -1));
    let c = fetch_src(base + vec2<i32>(1, -1));
    let d = fetch_src(base + vec2<i32>(-1, 0));
    let e = fetch_src(base + vec2<i32>(0, 0));
    let f = fetch_src(base + vec2<i32>(1, 0));
    let g = fetch_src(base + vec2<i32>(2, 0));
    let h = fetch_src(base + vec2<i32>(-1, 1));
    let i_tap = fetch_src(base + vec2<i32>(0, 1));
    let j = fetch_src(base + vec2<i32>(1, 1));
    let k = fetch_src(base + vec2<i32>(2, 1));
    let l = fetch_src(base + vec2<i32>(0, 2));
    let m = fetch_src(base + vec2<i32>(1, 2));

    let b_l = fsr_luma(b.rgb);
    let c_l = fsr_luma(c.rgb);
    let d_l = fsr_luma(d.rgb);
    let e_l = fsr_luma(e.rgb);
    let f_l = fsr_luma(f.rgb);
    let g_l = fsr_luma(g.rgb);
    let h_l = fsr_luma(h.rgb);
    let i_l = fsr_luma(i_tap.rgb);
    let j_l = fsr_luma(j.rgb);
    let k_l = fsr_luma(k.rgb);
    let l_l = fsr_luma(l.rgb);
    let m_l = fsr_luma(m.rgb);

    // Edge gradient & direction analysis (SgsrEdgeDirection)
    let gx = (f_l - d_l) + (j_l - h_l) + (g_l - e_l) + (k_l - i_l);
    let gy = (i_l - b_l) + (j_l - c_l) + (l_l - e_l) + (m_l - f_l);
    let grad_len = length(vec2<f32>(gx, gy));
    // Edge direction is perpendicular to gradient
    let dir = select(vec2<f32>(1.0, 0.0), vec2<f32>(-gy, gx) / grad_len, grad_len > 1e-4);

    // Map sharpness [0, 1] -> SGSR EdgeSharpness [1.0, 2.5]
    let edge_sharpness = mix(1.0, 2.5, clamp(constants.sharpness, 0.0, 1.0));
    let edge_strength = clamp((grad_len - (8.0 / 255.0)) * edge_sharpness, 0.0, 1.0);

    let wb = sgsr_weight(vec2<f32>(0.0, -1.0), pp, dir, edge_strength);
    let wc = sgsr_weight(vec2<f32>(1.0, -1.0), pp, dir, edge_strength);
    let wd = sgsr_weight(vec2<f32>(-1.0, 0.0), pp, dir, edge_strength);
    let we = sgsr_weight(vec2<f32>(0.0, 0.0), pp, dir, edge_strength);
    let wf = sgsr_weight(vec2<f32>(1.0, 0.0), pp, dir, edge_strength);
    let wg = sgsr_weight(vec2<f32>(2.0, 0.0), pp, dir, edge_strength);
    let wh = sgsr_weight(vec2<f32>(-1.0, 1.0), pp, dir, edge_strength);
    let wi = sgsr_weight(vec2<f32>(0.0, 1.0), pp, dir, edge_strength);
    let wj = sgsr_weight(vec2<f32>(1.0, 1.0), pp, dir, edge_strength);
    let wk = sgsr_weight(vec2<f32>(2.0, 1.0), pp, dir, edge_strength);
    let wl = sgsr_weight(vec2<f32>(0.0, 2.0), pp, dir, edge_strength);
    let wm = sgsr_weight(vec2<f32>(1.0, 2.0), pp, dir, edge_strength);

    let w_sum = wb + wc + wd + we + wf + wg + wh + wi + wj + wk + wl + wm;
    let c_sum = b.rgb * wb
        + c.rgb * wc
        + d.rgb * wd
        + e.rgb * we
        + f.rgb * wf
        + g.rgb * wg
        + h.rgb * wh
        + i_tap.rgb * wi
        + j.rgb * wj
        + k.rgb * wk
        + l.rgb * wl
        + m.rgb * wm;

    var out_rgb = select(e.rgb, c_sum / w_sum, abs(w_sum) > 1e-5);

    // Anti-ringing clamp against center 2x2 quad
    let min_rgb = min(min(e.rgb, f.rgb), min(i_tap.rgb, j.rgb));
    let max_rgb = max(max(e.rgb, f.rgb), max(i_tap.rgb, j.rgb));
    out_rgb = clamp(out_rgb, min_rgb, max_rgb);

    // Bilinear alpha from center 2x2 quad
    let out_a = mix(mix(e.a, f.a, pp.x), mix(i_tap.a, j.a, pp.x), pp.y);
    return vec4<f32>(max(out_rgb, vec3<f32>(0.0)), clamp(out_a, 0.0, 1.0));
}

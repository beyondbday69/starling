// CubeMap's position-only cube with Complementary Reimagined dawn (time 23400) shader grading.
struct Projection { view_proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> projection: Projection;
@group(0) @binding(1) var panorama: texture_cube<f32>;
@group(0) @binding(2) var panorama_sampler: sampler;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) direction: vec3<f32>,
}

@vertex fn vs_panorama(@location(0) position: vec3<f32>) -> Out {
    var out: Out;
    out.clip = projection.view_proj * vec4<f32>(position, 1.0);
    out.direction = position;
    return out;
}

// ACES filmic tone mapping (Complementary Reimagined curve)
fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a: f32 = 2.51;
    let b: f32 = 0.03;
    let c: f32 = 2.43;
    let d: f32 = 0.59;
    let e: f32 = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment fn fs_panorama(in: Out) -> @location(0) vec4<f32> {
    let dir = normalize(in.direction);
    let col = textureSample(panorama, panorama_sampler, dir).rgb;

    // Time 23400: Sunrise just breaking the East horizon
    let sun_dir = normalize(vec3<f32>(0.98, 0.14, 0.12));
    let sun_dot = max(dot(dir, sun_dir), 0.0);

    // Complementary Reimagined dawn atmospheric color palette (Medium preset)
    let dawn_gold = vec3<f32>(1.05, 0.62, 0.28);
    let dawn_rose = vec3<f32>(0.85, 0.42, 0.48);
    let twilight_sky = vec3<f32>(0.12, 0.16, 0.32);

    // Sun radiance bloom & atmospheric haze
    let sun_glow = pow(sun_dot, 8.0) * 0.45 + pow(sun_dot, 32.0) * 0.75 + pow(sun_dot, 128.0) * 1.2;
    let horizon_haze = clamp(1.0 - abs(dir.y) * 2.8, 0.0, 1.0);

    // Sky gradient blend: twilight overhead to warm dawn along horizon
    let sky_tint = mix(twilight_sky, mix(dawn_rose, dawn_gold, pow(sun_dot, 2.0)), horizon_haze);

    // Directional illumination from sunrise
    let lighting_factor = max(dot(dir, sun_dir) * 0.4 + 0.6, 0.2);
    let lit_col = col * mix(vec3<f32>(0.82, 0.88, 1.05), dawn_gold, sun_dot * 0.65) * lighting_factor;

    // Combine scene with dawn atmosphere
    var final_col = lit_col + (sky_tint * 0.18 + dawn_gold * (sun_glow * 0.35)) * (1.0 - col.r * 0.2);

    // Reimagined vibrant saturation boost
    let luma = dot(final_col, vec3<f32>(0.2126, 0.7152, 0.0722));
    final_col = mix(vec3<f32>(luma), final_col, 1.18);

    // Tone map with ACES filmic
    let tone_mapped = aces_filmic(final_col * 1.08);

    return vec4<f32>(tone_mapped, 1.0);
}

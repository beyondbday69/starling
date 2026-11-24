// CubeMap's position-only cube with Complementary Reimagined dawn (time 23400) crisp lighting.
override HDR_WRITE_LIGHT: bool = false;
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

@fragment fn fs_panorama(in: Out) -> @location(0) vec4<f32> {
    // Direct sampling: 100% sharp, zero blur, zero haze
    let col = textureSample(panorama, panorama_sampler, in.direction).rgb;

    // Time 23400: Sunrise directional warmth and crisp contrast (Reimagined preset)
    let dir = normalize(in.direction);
    let sun_dir = normalize(vec3<f32>(0.98, 0.14, 0.12));
    let sun_factor = max(dot(dir, sun_dir), 0.0);

    // Warm dawn tint toward the sunrise without any foggy haze or blur
    let dawn_tint = mix(vec3<f32>(0.94, 0.96, 1.02), vec3<f32>(1.08, 0.96, 0.82), sun_factor * 0.65);
    let lit_col = col * dawn_tint;

    // Crisp vibrance and contrast (preserving 100% texture detail)
    let luma = dot(lit_col, vec3<f32>(0.2126, 0.7152, 0.0722));
    let crisp_col = clamp(mix(vec3<f32>(luma), lit_col, 1.12) * 1.02, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(crisp_col, select(1.0, 1.0, HDR_WRITE_LIGHT));
}

// Bloom, exposure, the vibrant color grade and tonemapping.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/grade.glsl"

const bool colortex3MipmapEnabled = true;
const float eyeBrightnessHalflife = 30.0;   // ticks the eyes need to half-adapt (30 = 1.5 s, almost done in ~5 s)
uniform ivec2 eyeBrightness;
#ifdef DEPTH_OF_FIELD
const bool colortex0MipmapEnabled = true;
#endif

uniform sampler2D colortex0;
uniform sampler2D colortex3;
uniform sampler2D depthtex0;
uniform sampler2D depthtex1;
uniform float centerDepthSmooth;
uniform float aspectRatio;

float linDepth(float d) {
	return 2.0 * near * far / (far + near - (d * 2.0 - 1.0) * (far - near));
}

#if defined GODRAYS || defined UNDERWATER_RAYS
/** Screen-space sun rays: march toward the sun and count how much open sky we pass. */
vec3 godrays(vec2 uv) {
#ifndef GODRAYS
	if (isEyeInWater != 1) return vec3(0.0);               // only underwater rays are enabled
#endif
#ifndef UNDERWATER_RAYS
	if (isEyeInWater == 1) return vec3(0.0);
#endif
	vec3 lightV = shadowLightPosition;
	if (lightV.z > 0.0) return vec3(0.0);                  // sun/moon behind the camera
	float facing = clamp(-normalize(lightV).z, 0.0, 1.0);
	if (facing <= 0.001) return vec3(0.0);

	vec4 c = gbufferProjection * vec4(lightV, 1.0);
	vec2 sunUV = (c.xy / c.w) * 0.5 + 0.5;
	vec2 d = (uv - sunUV) * vec2(aspectRatio, 1.0);
	float falloff = exp(-dot(d, d) * (isEyeInWater == 1 ? 1.5 : 3.0));
	if (facing <= 0.001 || falloff <= 0.001) return vec3(0.0);

	vec2 delta = (sunUV - uv) / 12.0;
	float dither = fract(dot(gl_FragCoord.xy, vec2(0.0671, 0.00584)) * 52.98);
	vec2 p = uv + delta * dither;
	float sky = 0.0;
	for (int i = 0; i < 12; i++) {
		vec2 q = clamp(p, vec2(0.001), vec2(0.999));
		float dVal = isEyeInWater == 1 ? textureLod(depthtex1, q, 0.0).r : textureLod(depthtex0, q, 0.0).r;
		sky += dVal >= 0.9999 ? 1.0 : 0.0;
		p += delta;
	}
	sky /= 12.0;
	if (isEyeInWater == 1) {
		// Deep aquatic blue-cyan volumetric shafts matching Complementary Reimagined
		vec3 waterShaft = vec3(0.40, 0.70, 1.20) * directLightColor();
		float eyeSky = clamp(eyeBrightnessSmooth.y / 240.0, 0.0, 1.0);
		// Streaky, slowly shimmering shafts (radial streaks around the sun, drifting like light through waves)
		float ang = atan(d.y, d.x);
		float streak = 0.62 + 0.38 * sin(ang * 11.0 + frameTimeCounter * 0.45) * sin(ang * 5.0 - frameTimeCounter * 0.3 + 1.7);
		return waterShaft * sky * falloff * facing * streak * 0.3 * UNDERWATER_RAYS_STRENGTH * (1.0 - rainStrength) * eyeSky;
	}
	return directLightColor() * sky * falloff * facing * 0.14 * GODRAYS_STRENGTH * (1.0 - rainStrength);
}
#endif

#ifdef SUN_GLOW
/** Soft warm halo around the sun, no loops: two exp() falloffs. Strongest in the sky, faint on terrain (haze). */
vec3 sunGlow(vec2 uv, float depth) {
	vec3 sv = sunPosition;
	if (sv.z > 0.0) return vec3(0.0);                      // sun behind the camera
	float sunH = mqSunDir.y;                               // sun height in world space
	float vis = smoothstep(-0.12, 0.08, sunH) * (1.0 - rainStrength * 0.8);
	if (vis <= 0.001) return vec3(0.0);
	vec4 c = gbufferProjection * vec4(sv, 1.0);
	vec2 sunUV = (c.xy / c.w) * 0.5 + 0.5;
	vec2 d = (uv - sunUV) * vec2(aspectRatio, 1.0);
	float r2 = dot(d, d);
	float horizon = 1.0 - smoothstep(0.0, 0.45, sunH);     // redder and wider near the horizon
	float halo = exp(-r2 * 9.0) * 0.55 + exp(-r2 * 1.6) * 0.25 + exp(-r2 * 0.35) * 0.08 * horizon;
	vec3 warm = mix(vec3(1.0, 0.74, 0.44), vec3(1.0, 0.58, 0.26), horizon);
	float onSky = depth >= 1.0 ? 1.0 : 0.22;
	return warm * halo * onSky * vis * (0.55 + 0.95 * horizon) * SUN_GLOW_STRENGTH * (isEyeInWater == 0 ? 1.0 : 0.0);
}
#endif

in vec2 texcoord;

/* RENDERTARGETS: 1 */   // display-ready 8-bit result (colortex1 is RGBA8); colortex0 is an HDR 11-bit float buffer and would band
layout(location = 0) out vec4 outColor;

// Two bilinear taps per mip level (the old version used four taps on five levels = 20 fetches).
vec3 bloomTap(float lod, vec2 dir) {
	vec2 px = exp2(lod) * 0.5 / vec2(viewWidth, viewHeight);
	return (textureLod(colortex3, texcoord + px * dir, lod).rgb + textureLod(colortex3, texcoord - px * dir, lod).rgb) * 0.5;
}

void main() {
	vec3 color = texture(colortex0, texcoord).rgb;

#ifdef DEPTH_OF_FIELD
	{
		float d = texture(depthtex0, texcoord).r;
		if (d > 0.56) {                                   // never blur your hand
			float focus = linDepth(centerDepthSmooth);
			float here = d >= 1.0 ? far : linDepth(d);
			float coc = clamp(abs(here - focus) / max(focus * 1.5, 4.0), 0.0, 1.0) * DOF_STRENGTH;
			float lod = coc * 3.2;
			if (lod > 0.05) {
				vec2 px = exp2(lod) * 0.5 / vec2(viewWidth, viewHeight);
				vec3 b = textureLod(colortex0, texcoord + vec2( px.x,  px.y), lod).rgb
				       + textureLod(colortex0, texcoord + vec2(-px.x,  px.y), lod).rgb
				       + textureLod(colortex0, texcoord + vec2( px.x, -px.y), lod).rgb
				       + textureLod(colortex0, texcoord + vec2(-px.x, -px.y), lod).rgb;
				color = mix(color, b * 0.25, smoothstep(0.0, 0.3, coc));
			}
		}
	}
#endif

#if (defined GODRAYS || defined UNDERWATER_RAYS) && !defined DIM_NETHER && !defined DIM_END
	color += godrays(texcoord);
#endif
#if defined SUN_GLOW && defined DIM_OVERWORLD
	color += sunGlow(texcoord, texture(depthtex0, texcoord).r);
#endif

#ifdef BLOOM
	vec3 bloom = bloomTap(2.0, vec2(1.0, 1.0)) * 0.40 + bloomTap(4.0, vec2(1.0, -1.0)) * 0.35 + bloomTap(6.0, vec2(1.0, 1.0)) * 0.25;
	color += bloom * 0.22 * BLOOM_STRENGTH;
#endif

#ifdef EYE_ADAPTATION
	// Eye adaptation: eyeBrightness is where you are right now, eyeBrightnessSmooth is what your "eyes" are used to.
	// Cave -> sunlight: now is bright, eyes are still dark-adapted -> everything flashes white, then settles over a few seconds.
	// Sunlight -> cave: now is dark, eyes are bright-adapted -> it looks darker, then slowly brightens.
	{
		float now = clamp(float(eyeBrightness.y) / 240.0, 0.0, 1.0);
		float used = clamp(float(eyeBrightnessSmooth.y) / 240.0, 0.0, 1.0);
		float lag = now - used;
		float dayW = 0.25 + 0.75 * dayFactor();
		float flash = max(lag, 0.0) * dayW * EYE_ADAPT_STRENGTH;
		float dim = max(-lag, 0.0) * EYE_ADAPT_STRENGTH;
		color *= (1.0 + 2.2 * flash) * (1.0 - 0.4 * clamp(dim, 0.0, 1.0));
		color += vec3(0.35 * flash * flash);   // blown-out white wash at the peak of the flash
	}
#endif
	color = gradeColor(color, eyeBrightnessSmooth.y / 240.0, dayFactor(), texcoord);
	// tiny dither so smooth skies don't band in 8 bit
	float n = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
	outColor = vec4(color + (n - 0.5) / 255.0, 1.0);
}

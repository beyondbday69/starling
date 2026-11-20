// Mystique - water reflections, water depth tint, underwater fog, bloom bright-pass.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

/*
const int colortex0Format = R11F_G11F_B10F;
const int colortex2Format = RG8;
const int colortex3Format = R11F_G11F_B10F;
const vec4 colortex2ClearColor = vec4(0.0, 0.0, 0.0, 0.0);
*/

uniform sampler2D colortex0;
uniform sampler2D colortex2;
uniform sampler2D depthtex0;
uniform sampler2D depthtex1;
uniform sampler2D noisetex;
#if defined WATER_RIPPLES || defined CLOUDS_REIMAGINED || defined UNDERWATER_CAUSTICS
uniform sampler2D gaux4;       // water ripple normal map (rg = normal, a = height), bound in shaders.properties
#endif
const int noiseTextureResolution = 256;

in vec2 texcoord;

/* RENDERTARGETS: 0,3 */
layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outBloom;

vec3 screenToView(vec2 uv, float depth) {
	vec4 p = gbufferProjectionInverse * vec4(vec3(uv, depth) * 2.0 - 1.0, 1.0);
	return p.xyz / p.w;
}

vec3 viewToScreen(vec3 v) {
	vec4 p = gbufferProjection * vec4(v, 1.0);
	return (p.xyz / p.w) * 0.5 + 0.5;
}

// Cheap helpers for the ray march: they skip the full matrix multiply + depth reconstruction of the old version.
// view position -> screen uv (x,y only; depth is compared in view space instead)
vec2 ssrProject(vec3 p) {
	return (vec2(gbufferProjection[0][0] * p.x + gbufferProjection[2][0] * p.z,
	             gbufferProjection[1][1] * p.y + gbufferProjection[2][1] * p.z) / -p.z) * 0.5 + 0.5;
}
// depth buffer value -> view-space z (one divide, no mat4)
float ssrViewZ(float d) {
	return -gbufferProjection[3][2] / (d * 2.0 - 1.0 + gbufferProjection[2][2]);
}

/** Screen-space reflection. Returns hit uv in xy and confidence in z (0 = miss).
 *  Per step: 1 projection (2 mads + divide), 1 depth read, 1 divide. Start is jittered by `dither`
 *  so few steps don't show bands. */
vec3 traceReflection(vec3 start, vec3 dir, float dither) {
	vec3 stepV = dir * (0.64 + length(start) * 0.064);
	vec3 pos = start + stepV * dither;
	for (int i = 0; i < SSR_STEPS; i++) {
		pos += stepV;
		if (pos.z >= -near) return vec3(0.0);
		vec2 s = ssrProject(pos);
		if (s.x < 0.0 || s.x > 1.0 || s.y < 0.0 || s.y > 1.0) return vec3(0.0);
		float d = textureLod(depthtex1, s, 0.0).r;
		if (d < 1.0) {
			float diff = ssrViewZ(d) - pos.z;          // > 0: the ray went behind something
			if (diff > 0.0 && diff < max(1.0, length(stepV) * 2.5)) {
				// refine: 2 bisection steps between the last two positions
				vec3 lo = pos - stepV, hi = pos;
				for (int j = 0; j < 2; j++) {
					vec3 m = (lo + hi) * 0.5;
					vec2 ms = ssrProject(m);
					float md = ssrViewZ(textureLod(depthtex1, ms, 0.0).r);
					if (md > m.z) hi = m; else lo = m;
				}
				vec2 hs = ssrProject(hi);
				if (hs.x < 0.0 || hs.x > 1.0 || hs.y < 0.0 || hs.y > 1.0) return vec3(0.0);
				vec2 edge = smoothstep(vec2(0.0), vec2(0.08), hs) * smoothstep(vec2(1.0), vec2(0.92), hs);
				return vec3(hs, edge.x * edge.y);
			}
		}
		stepV *= 1.7;                               // steps grow fast: 4 steps now reach ~20 blocks, not ~12
	}
	return vec3(0.0);
}

#if defined BLOCKY_CLOUDS && !defined DIM_NETHER && !defined DIM_END
/** 1 if this cloud cell is filled. Two smooth noise reads, nothing else. */
bool cloudFilled(vec2 cell) {
	float n = textureLod(noisetex, fract(cell * (0.075 / 256.0) + 0.37), 0.0).r * 0.65
	        + textureLod(noisetex, fract(cell * (0.21 / 256.0) + 0.71), 0.0).g * 0.35;
	float cover = CLOUD_COVERAGE + rainStrength * 0.25;
	return n > 1.0 - cover;
}

/** Box clouds: walk the cloud grid along the view ray (2D DDA through one thick layer). rgb + alpha. */
vec4 blockyClouds(vec3 dir, float sceneDist) {
	float h0 = float(CLOUD_HEIGHT), h1 = h0 + CLOUD_THICKNESS;
	vec3 ro = cameraPosition;
	ro.x += frameTimeCounter * 0.6 * CLOUD_SPEED;           // drift, like vanilla clouds
	if (ro.y < h0 && dir.y <= 0.0) return vec4(0.0);        // below the clouds, looking down
	if (ro.y > h1 && dir.y >= 0.0) return vec4(0.0);        // above the clouds, looking up
	if (abs(dir.y) < 1e-4) dir.y = 1e-4;
	float ta = (h0 - ro.y) / dir.y, tb = (h1 - ro.y) / dir.y;
	float tEnter = max(min(ta, tb), 0.0);
	if (sceneDist < tEnter || tEnter >= CLOUD_RENDER) return vec4(0.0);
	float tExit = min(max(ta, tb), min(CLOUD_RENDER, sceneDist));
	if (tEnter >= tExit) return vec4(0.0);

	vec3 p = ro + dir * tEnter;
	vec2 cell = floor(p.xz / CLOUD_SIZE);
	vec2 st = vec2(dir.x >= 0.0 ? 1.0 : -1.0, dir.z >= 0.0 ? 1.0 : -1.0);
	vec2 ad = max(abs(dir.xz), vec2(1e-5));
	vec2 tDelta = CLOUD_SIZE / ad;
	vec2 tMax = tEnter + ((cell + max(st, 0.0)) * CLOUD_SIZE - p.xz) * st / ad;
	float t = tEnter;
	int axis = -1;                                    // -1 = entered through top/bottom
	bool hit = false;
	for (int i = 0; i < CLOUD_STEPS; i++) {
		if (cloudFilled(cell)) { hit = true; break; }
		if (tMax.x < tMax.y) { t = tMax.x; cell.x += st.x; tMax.x += tDelta.x; axis = 0; }
		else                 { t = tMax.y; cell.y += st.y; tMax.y += tDelta.y; axis = 1; }
		if (t >= tExit) break;
	}
	if (!hit) return vec4(0.0);

	vec3 n = axis == 0 ? vec3(-st.x, 0.0, 0.0) : axis == 1 ? vec3(0.0, 0.0, -st.y) : vec3(0.0, dir.y > 0.0 ? -1.0 : 1.0, 0.0);
	if (ro.y > h0 && ro.y < h1 && t <= 0.001) n = -dir;   // camera inside a cloud

	// Minecraft-style face shading + sun side brighter + soft gradient from bottom to top
	float face = n.y > 0.5 ? 1.0 : n.y < -0.5 ? 0.58 : (abs(n.z) > 0.5 ? 0.84 : 0.72);
	vec3 lW = normalize(mat3(gbufferModelViewInverse) * shadowLightPosition);
	float hy = clamp((ro.y + dir.y * t - h0) / CLOUD_THICKNESS, 0.0, 1.0);
	vec3 col = ambientColor() * 1.7 + directLightColor() * (0.32 + 0.22 * max(dot(n, lW), 0.0));
	col *= face * mix(0.82, 1.0, hy);
	col = mix(col, vec3(luma(col)) * 0.75, rainStrength * 0.6);

	// fades into the sky with distance
	float haze = 1.0 - exp(-t * 0.0022);
	col = mix(col, skyGradient(normalize(vec3(dir.x, max(dir.y, 0.02), dir.z))), haze * 0.75);
	float alpha = CLOUD_OPACITY * (1.0 - smoothstep(CLOUD_RENDER * 0.55, CLOUD_RENDER, t));
	return vec4(col, alpha);
}
#endif


#if defined BLOCKY_CLOUDS && defined CLOUDS_REIMAGINED && !defined DIM_NETHER && !defined DIM_END
// ---- Complementary Reimagined clouds: a thin slab (+-4.2 blocks) of rounded cloud noise, marched a few steps,
// ---- lit by two samples of the same noise toward the sun. The noise is the blue channel of cloud-water.png.
float cloudTex(vec2 pos) {
	vec2 coord = pos.yx + 0.5;
	vec2 sg = sign(coord);
	coord = abs(coord) + 1.0;
	vec2 i = floor(coord);
	vec2 f = smoothstep(vec2(0.375), vec2(0.625), coord - i);     // rounds the corners of each noise cell
	// Manual bilinear read (texelFetch + wrap) so the rounded shape never depends on the texture's filter flag.
	vec2 p = (i + f - 0.5) * sg - 0.5;
	ivec2 b = ivec2(floor(p));
	vec2 w = p - vec2(b);
	float a0 = texelFetch(gaux4, (b) & 255, 0).b;
	float a1 = texelFetch(gaux4, (b + ivec2(1, 0)) & 255, 0).b;
	float a2 = texelFetch(gaux4, (b + ivec2(0, 1)) & 255, 0).b;
	float a3 = texelFetch(gaux4, (b + ivec2(1, 1)) & 255, 0).b;
	return mix(mix(a0, a1, w.x), mix(a2, a3, w.x), w.y);
}

vec4 reimaginedClouds(vec3 dir, float sceneDist, float dither) {
	const float stretch = 4.2, tallness = 8.4;
	float alt = float(CLOUD_HEIGHT);
	float thresholdF = CLOUD_RENDER * 2.0;
	// wrap the camera by one noise-texture period (256 cells of 1/0.07 blocks) so far-from-origin worlds keep their precision
	vec3 cam = vec3(mod(cameraPosition.x, 3657.1428), cameraPosition.y, mod(cameraPosition.z, 3657.1428));
	if (cam.y < alt - stretch && dir.y <= 0.0) return vec4(0.0);
	if (cam.y > alt + stretch && dir.y >= 0.0) return vec4(0.0);
	float dy = dir.y >= 0.0 ? max(dir.y, 1e-3) : min(dir.y, -1e-3);
	float d0 = (alt - stretch - cam.y) / dy, d1 = (alt + stretch - cam.y) / dy;
	float tEnter = max(min(d0, d1), 0.0), tMax = max(d0, d1);
	if (tMax < 0.0 || sceneDist < tEnter || tEnter >= thresholdF) return vec4(0.0);
	tMax = min(tMax, thresholdF * inversesqrt(max(1.0 - dir.y * dir.y, 1e-4)));
	tMax = min(tMax, sceneDist - 1.0);
	if (tMax <= tEnter) return vec4(0.0);
	float dif = tMax - tEnter;
	int count = clamp(int(dif / 10.0), 6, CLOUD_STEPS);   // Complementary quality 2: one step per 10 blocks
	float stepLen = dif / float(count);
	float xzScale = sqrt(max(1.0 - dir.y * dir.y, 0.0));
	float cover = CLOUD_COVERAGE - 0.48 + rainStrength * 0.12;
	float wind = frameTimeCounter * 0.6 * CLOUD_SPEED;

	for (int i = 0; i < CLOUD_STEPS; i++) {
		if (i >= count) break;
		float t = tEnter + stepLen * (float(i) + dither);
		vec3 tp = cam + dir * t;
		vec2 q = vec2(tp.x + alt * 64.0, tp.z - wind) * 0.07;
		float thr = clamp(abs(alt - tp.y) / stretch, 0.001, 0.999);
		float t2 = thr * thr, t4 = t2 * t2;
		if (cloudTex(q) + cover > t4 * t4 * 0.5 + 0.25) {
			float cs = 1.0 - (alt + stretch - tp.y) / tallness;
			cs = pow(max(cs, 0.0), 0.8);
			vec3 lW = normalize(mat3(gbufferModelViewInverse) * shadowLightPosition);
			float shadowTime = abs(2.0 * dayFactor() - 1.0);         // 0 while the sun is changing over, 1 by day and night
			float csM = 1.0 - cs;
			vec2 add = lW.xz * (0.08 * shadowTime);
			vec2 lp = q + (1.0 + dither) * add;
			float light = 2.0 - cloudTex(lp) * csM;
			lp += dither * add;
			light -= cloudTex(lp) * csM;
			cs = (max(dot(dir, lW), 0.0) * shadowTime * 0.25 + 0.5 * cs + 0.08) * light;

			vec3 amb = ambientColor() * (1.6 * (1.0 - 0.25 * rainStrength));
			vec3 col = amb * 0.95 * (1.0 - 0.35 * cs) + directLightColor() * 0.38 * 0.9 * (0.1 + cs);
			vec3 skyC = skyGradient(normalize(vec3(dir.x, max(dir.y, 0.02), dir.z)));
			float ratio = (thresholdF - t * xzScale) / thresholdF;
			float fogF = clamp(ratio, 0.0, 1.0);
			fogF = fogF * fogF * 0.707;
			// Complementary look: sunset clouds are lit warm (orange/peach), brightest on the sun's side
			float cSet = sunsetFactor();
			float cSide = max(dot(dir, mqSunDir.xyz), 0.0);
			col *= mix(vec3(1.0), vec3(1.35, 0.92, 0.66), clamp(cSet * (0.55 + 0.45 * cSide), 0.0, 1.0));
			col += directLightColor() * cSet * cSide * cSide * 0.06;
			col = mix(skyC, col, fogF * 0.707);
			float a = sqrt(clamp(ratio, 0.0, 0.75) * 1.33333) * min(CLOUD_OPACITY / 0.88, 1.0);
			return vec4(col, a);
		}
	}
	return vec4(0.0);
}
#endif

void main() {
	vec3 color = texture(colortex0, texcoord).rgb;
	float depth0 = texture(depthtex0, texcoord).r;
	vec3 vp0 = screenToView(texcoord, depth0);
	vec4 water = texture(colortex2, texcoord);   // r > 0.25 on water

#if defined BLOCKY_CLOUDS && !defined DIM_NETHER && !defined DIM_END
	float depth1_c = textureLod(depthtex1, texcoord, 0.0).r;
	if (isEyeInWater == 0) {
		vec3 dirW = normalize(mat3(gbufferModelViewInverse) * vp0);
		float sceneDist = depth0 >= 1.0 ? 1e6 : length(vp0);
	#ifdef CLOUDS_REIMAGINED
		float cDither = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
		vec4 cl = reimaginedClouds(dirW, sceneDist, cDither);
	#else
		vec4 cl = blockyClouds(dirW, sceneDist);
	#endif
		color = mix(color, cl.rgb, cl.a);
	} else if (isEyeInWater == 1 && depth1_c >= 0.9999) {
		// Underwater view looking up at sky through water surface:
		vec3 vpSky = screenToView(texcoord, 1.0);
		vec3 dirW = normalize(mat3(gbufferModelViewInverse) * vpSky);
		if (dirW.y > 0.0) {
		#ifdef CLOUDS_REIMAGINED
			float cDither = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
			vec4 cl = reimaginedClouds(dirW, 1e6, cDither);
		#else
			vec4 cl = blockyClouds(dirW, 1e6);
		#endif
			// Water absorbs warm colors, tinting clouds slightly blue
			cl.rgb *= vec3(0.85, 0.92, 1.05);
			color = mix(color, cl.rgb, cl.a * 0.9);
		}
	}
#endif

#if !defined DIM_NETHER && !defined DIM_END
	if (water.r > 0.25 && isEyeInWater == 0) {
		float skyL = clamp((water.r - 0.5) * 2.0, 0.0, 1.0);
		float reflMult = water.g;                            // reflection mask (1.0 - foam) from water pass
		float dist0 = length(vp0);
	#ifdef WATER_REIMAGINED
		// Clean water tint matching Complementary Reimagined without darkening or dirtying underwater terrain
		// (runtime check: floats are not allowed inside #if on strict GLSL compilers)
		if (WATER_TINT != 0.0) {
			vec3 waterTint = mix(vec3(1.0), vec3(0.95, 0.98, 1.05), WATER_TINT);
			color *= waterTint;
		}
	#endif
		vec3 v = vp0 / dist0;
		vec3 upV = normalize(upPosition);
		float t = frameTimeCounter;
		vec3 wpos = (gbufferModelViewInverse * vec4(vp0, 1.0)).xyz + cameraPosition;
		float fresG = clamp(1.0 + dot(upV, v), 0.0, 1.0);    // Fresnel of the flat surface

		// --- 1. Ripple normal
	#if defined WATER_RIPPLES
		// Complementary's recipe: one normal map read at 4 sizes, drifting in different directions.
		vec2 wp = 0.032 * (wpos.xz + wpos.y * 2.0) * 2.5;
		vec2 wind = vec2(0.0, -t * WATER_SPEED * 0.018 * 2.5);
		vec2 nMed = textureLod(gaux4, wp + wind, 0.0).rg - 0.5;
		vec2 nSmall = textureLod(gaux4, wp * 4.0 - 2.0 * wind, 0.0).rg - 0.5;
		vec2 nBig = textureLod(gaux4, wp * 0.25 - 0.5 * wind, 0.0).rg + textureLod(gaux4, wp * 0.05 - 0.05 * wind, 0.0).rg - 1.0;
		nSmall *= 1.0 / (1.0 + dist0 * 0.02);                // far away the tiny ripples would only shimmer
		nMed *= 1.0 / (1.0 + dist0 * 0.006);
		vec2 nm = nMed * 1.7 + nSmall * 0.75 + nBig * 2.0;
		nm *= 6.0 * (1.0 - 0.7 * fresG) * WATER_WAVES * (0.03 * skyL + 0.01);
		vec3 nW = normalize(vec3(nm.x, 1.0, nm.y));
	#elif defined WATER_REIMAGINED
		// Style 1 has a flat surface: the look comes from the texture, foam and reflections.
		vec3 nW = vec3(0.0, 1.0, 0.0);
	#else
		vec2 g = wpos.xz;
		#ifdef WATER_PIXELATED
		g = floor(g * 16.0) * 0.0625;
		#endif
		vec2 warp = textureLod(noisetex, fract(g * (1.0 / 48.0) + vec2(t * 0.004, t * 0.003)), 0.0).rg * 2.0 - 1.0;
		float p1 = dot(g, vec2(0.80, 0.60)) * 1.7 + t * 1.10 + warp.x * 2.5;
		float p2 = dot(g, vec2(-0.45, 0.89)) * 2.9 - t * 1.45 + warp.y * 2.5;
		float p3 = dot(g, vec2(0.30, -0.95)) * 5.3 + t * 2.10 + (warp.x + warp.y) * 1.5;
		vec2 grad = vec2(0.80, 0.60) * (0.10 * cos(p1)) + vec2(-0.45, 0.89) * (0.116 * cos(p2)) + vec2(0.30, -0.95) * (0.106 * cos(p3));
		grad *= 0.8 * WATER_WAVES / (1.0 + dist0 * 0.012);
		vec3 nW = normalize(vec3(-grad.x, 1.0, -grad.y));
	#endif
		vec3 nV = mat3(gbufferModelView) * nW;

		// --- 2. Fresnel: Complementary uses 1 - cos (a straight ramp), cubed, so reflections get strong fast.
		float fres = clamp(1.0 + dot(nV, v), 0.0, 1.0);
	#if defined WATER_REIMAGINED && !defined WATER_RIPPLES
		// Style 1 (Flat Reimagined): clean Fresnel reflection matching Complementary Reimagined
		float fres2 = fres * fres;
		float fresnelM = fres2 * fres2 * REFLECTION_STRENGTH;
	#else
		float fresnelM = (fres * fres * fres * 0.85 + 0.15) * REFLECTION_STRENGTH;
	#endif

		// Lighter on phones: when the reflection would be almost invisible (looking down into the water), skip all of it.
		float reflAmt = clamp(fresnelM * reflMult, 0.0, 1.0);
		if (reflAmt > 0.015) {
		// --- 3. Reflection: sky first, then paint terrain over it where the mirror image lands on something.
		vec3 nR = normalize(reflect(v, nV));
		vec3 rW = mat3(gbufferModelViewInverse) * nR;
		vec3 sky = skyGradient(vec3(rW.x, abs(rW.y), rW.z));
		vec3 refl = sky * (0.2 + 0.8 * skyL);

	bool needMirror = true;                               // set to false when the ray march found something
	#ifdef REFLECTIONS
		// Only march when the reflection is strong enough to see (looking down into the water it is faint).
		if (reflAmt > 0.06) {
			float ditherS = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
			vec3 hit = traceReflection(vp0 + nV * 0.05, nR, ditherS);
			if (hit.z > 0.0) { refl = mix(refl, textureLod(colortex0, hit.xy, 0.0).rgb, hit.z); needMirror = hit.z < 0.5; }
		}
	#endif
	#ifdef MIRROR_REFLECTIONS
		// With Ray-Marched Reflections on, this only runs where the ray missed, so far-away trees and hills still show.
		if (needMirror && reflAmt > 0.05)   // faint reflections skip the depth + color lookups
		{
			// Mirror image: where does the reflected direction land on screen? Look there.
			vec4 clipR = gbufferProjection * vec4(nR + 0.013 * vp0, 1.0);
			if (clipR.w > 0.0) {
				vec3 sR = clipR.xyz / clipR.w * 0.5 + 0.5;
				vec2 rEdge = vec2(0.6, 0.55);
				vec2 aR = abs(sR.xy - 0.5);
				if (aR.x < rEdge.x && aR.y < rEdge.y) {
					vec2 ef = aR / rEdge;
					ef *= ef; ef *= ef; ef *= ef;           // ^8: only the very edge fades
					float dither = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
					sR.y += (dither - 0.5) * (0.03 * (ef.x + ef.y) + 0.001);
					float zR = textureLod(depthtex1, sR.xy, 0.0).r;
					float lR = length(screenToView(sR.xy, zR));
					if (zR < 0.9999 && dist0 <= 2.0 + lR) {
						float ra = (1.0 - ef.x) * (1.0 - ef.y) * (1.0 - ef.y);
						ra *= clamp((dot(v, nR) - 0.45) * 10.0, 0.0, 1.0);    // no reflections from sideways rays
						float fg = lR / far; fg *= fg; fg *= fg; fg *= fg;
						ra *= exp(-3.0 * fg);                                  // fades out at the world edge
						refl = mix(refl, textureLod(colortex0, sR.xy, 0.0).rgb, ra);
					}
				}
			}
		}
	#endif
		color = mix(color, refl, reflAmt);
		}
	}
#endif

	// Underwater, in lava, or in powder snow.
	if (isEyeInWater == 1) {
		float eyeSky = clamp(eyeBrightnessSmooth.y / 240.0, 0.0, 1.0);
		float dayF = dayFactor();

		// Complementary Reimagined biome-aware underwater fog color:
		// Uses Minecraft's biome fogColor transformed by Reimagined's power curve
		vec3 uColor = pow(max(fogColor, vec3(0.001)), vec3(0.33, 0.21, 0.26));
		vec3 waterFogColorG = uColor * (0.22 + 0.12 * eyeSky) * (0.35 + 0.65 * dayF);
		vec3 waterFogLinear = toLinear(waterFogColorG * 1.6);

		if (depth0 >= 1.0) {
			// Sky / horizon underwater fades smoothly to the deep water fog
			color = waterFogLinear;
		} else {
			float dist = length(vp0);
			// Complementary quadratic-exponential fog curve
			float fD = dist / (28.0 / max(UNDERWATER_FOG_DENSITY, 0.1));
			float fog = 1.0 - exp(-(fD * fD + 0.35 * fD));   // linear term: fog already shows at 5-10 blocks, thick by ~30

			// Gentle underwater absorption tint (Complementary underwaterMult)
			vec3 underwaterMult = vec3(0.80, 0.87, 0.97);
			color *= toLinear(underwaterMult * 0.92);

		#ifdef UNDERWATER_CAUSTICS
			// Authentic Complementary Reimagined caustics:
			// Computed from curvature of water normal map (gaux4) projected onto underwater surfaces
			if (dayF > 0.05 && water.r < 0.1) {
				vec3 wpos = (gbufferModelViewInverse * vec4(vp0, 1.0)).xyz + cameraPosition;
				float speed = WATER_SPEED * 0.035;
				vec2 cWind = vec2(0.0, frameTimeCounter * speed);
				vec2 cPos1 = wpos.xz * 0.08 + cWind;
				vec2 cPos2 = wpos.xz * 0.06 - cWind;

				float offset = 0.002;
				float cMult = 14.0;
				float caustic = 0.0;
				caustic += dot(textureLod(gaux4, cPos1 + vec2(offset, 0.0), 0.0).rg, vec2(cMult))
				         - dot(textureLod(gaux4, cPos1 - vec2(offset, 0.0), 0.0).rg, vec2(cMult));
				caustic += dot(textureLod(gaux4, cPos2 + vec2(0.0, offset), 0.0).rg, vec2(cMult))
				         - dot(textureLod(gaux4, cPos2 - vec2(0.0, offset), 0.0).rg, vec2(cMult));

				float causticStrength = clamp(caustic * 0.75 + 0.35, 0.0, 1.0);
				// Radiant water-blue / cyan tint from Complementary Reimagined
				vec3 causticColor = mix(vec3(1.0), vec3(0.65, 0.88, 1.18), 0.75) * (causticStrength * 0.8 + 0.2);

				float depthBelowSurface = max(63.0 - wpos.y, 0.0);
				float depthFade = exp(-depthBelowSurface * 0.045);
				float skyLightFade = smoothstep(0.25, 0.85, eyeSky);
				float finalCausticFade = depthFade * skyLightFade * dayF * (1.0 - 0.85 * rainStrength);

				color *= mix(vec3(1.0), causticColor * 1.5, finalCausticFade);
			}
		#endif

			color = mix(color, waterFogLinear, clamp(fog, 0.0, 1.0));
		}
	} else if (isEyeInWater == 2) {
		color = mix(color, vec3(2.2, 0.5, 0.05), 1.0 - exp(-length(vp0) * 0.9));
	} else if (isEyeInWater == 3) {
		color = mix(color, vec3(0.75, 0.82, 0.9), 1.0 - exp(-length(vp0) * 0.6));
	}

	outColor = vec4(color, 1.0);

#ifdef BLOOM
	// Bright parts feed the bloom.
	float l = luma(color);
	float isSky = depth0 >= 1.0 ? 1.0 : 0.0;
	float thresh = mix(1.8, 5.0, isSky);   // sky only blooms around the sun
	outBloom = vec4(color * smoothstep(thresh, thresh * 2.5, l), 1.0);
#else
	outBloom = vec4(0.0);
#endif
}

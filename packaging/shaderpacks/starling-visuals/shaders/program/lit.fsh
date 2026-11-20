// Terrain, water, entities, particles and hand: lighting.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

uniform sampler2D gtexture;
uniform sampler2DShadow shadowtex0;   // hardware depth compare: what Iris 26.3 expects (a manual compare reads garbage there)
uniform float alphaTestRef = 0.1;
uniform vec4 entityColor;
uniform int heldBlockLightValue;
uniform int heldBlockLightValue2;
#ifdef WATER
uniform sampler2D depthtex1;   // depth without translucents: tells us how deep the water is
#endif
#if defined WATER || defined TERRAIN
uniform sampler2D noisetex;
#endif

in vec2 texcoord;
in vec2 lmcoord;
in vec4 glcolor;
in vec3 viewPos;
in vec3 normalV;
in vec3 shadowClip;
in vec2 signMid;
in vec3 worldPosV;
in vec4 fogMA;
flat in int blockId;

#ifdef WATER
/* RENDERTARGETS: 0,2 */
layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outWater;   // colortex2 is RG8 now: r = water flag + sky light, g = surface opacity
#else
/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;
#endif

float pow2f(float x) { return x * x; }

#if defined TOON_MODE && defined TERRAIN && !defined DIM_NETHER && !defined DIM_END
// ---- Toon look (CazToon style) -------------------------------------------------------------------
// How green a colour is (0..1): used to find grass without a block id.
float toonGreen(vec3 c, float a0, float a1, float b0, float b1) {
	return smoothstep(a0, a1, c.g - max(c.r, c.b)) * smoothstep(b0, b1, max(c.r, max(c.g, c.b)));
}
float toonNoise(vec2 p) { return textureLod(noisetex, p * (1.0 / 256.0), 0.0).r; }
/** Blocky grass patches: noise snapped to the 16x16 texel grid of each block face, quantised into 7 steps,
 *  slowly drifting. Returns a brightness multiplier, 0.83 (dark patch) .. 1.10 (light patch). */
float toonGrassPatch(vec3 wp, vec3 nW) {
	vec3 an = abs(nW);
	vec3 q = wp;
	if (an.y >= an.x && an.y >= an.z) q.xz = (floor(wp.xz * 16.0) + 0.5) / 16.0;
	else if (an.x >= an.z)            q.yz = (floor(wp.yz * 16.0) + 0.5) / 16.0;
	else                              q.xy = (floor(wp.xy * 16.0) + 0.5) / 16.0;
	float t = frameTimeCounter * 0.5;
	// two stretched noise fields at different angles, blended by a slow large-scale mask
	float u = q.x * 0.86 + q.z * 0.51;
	float v = q.z * 0.86 - q.x * 0.51;
	float warp = toonNoise(vec2(u * 0.090 + t * 0.050, v * 0.070 - t * 0.085)) - 0.5;
	vec2 pA = vec2(u * 0.155 + t * 0.018, (v + warp * 16.0 + q.y * 0.5) * 0.430 - t * 0.055);
	float nA = (toonNoise(pA) * 0.58 + toonNoise(pA * vec2(2.20, 1.32) + vec2(19.7 + t * 0.035, 42.4 - t * 0.020)) * 0.28) * (1.0 / 0.86);
	float u2 = q.x * 0.26 - q.z * 0.97;
	float v2 = q.z * 0.26 + q.x * 0.97;
	vec2 pB = vec2(u2 * 0.138 - t * 0.016, (v2 + q.y * 0.5) * 0.390 + t * 0.048);
	float nB = (toonNoise(pB + vec2(37.0 + t * 0.020, -13.0 - t * 0.035)) * 0.58 + toonNoise(pB * vec2(2.05, 1.38) + vec2(3.0 - t * 0.025, 29.0 + t * 0.030)) * 0.28) * (1.0 / 0.86);
	float sel = smoothstep(0.28, 0.72, toonNoise(vec2(q.x * 0.020 + t * 0.010 + 71.0, q.z * 0.020 - t * 0.008 + 23.0)));
	float n = clamp((mix(nA, nB, sel) - 0.5) * 1.8 + 0.5, 0.0, 0.999);   // 1.8: plain lattice noise clusters around 0.5, stretch it so all 7 steps get used
	return mix(0.83, 1.10, floor(n * 7.0) / 6.0);
}
#endif

float shadowLookup(vec2 uv, float z) {
	return texture(shadowtex0, vec3(uv, z));   // 1 = lit, 0 = shadow, already bilinear-smoothed by the GPU
}

/** 1 = fully lit, 0 = in shadow. */
float getShadow(float skyLight) {
	if (skyLight < 0.03) return 0.0;
	// Sky shadows: anything under trees, overhangs or cliffs sees less sky, so it's shaded.
	float fallback = smoothstep(0.80, 0.97, skyLight);
#if defined SHADOWS && !defined NO_SHADOW && !defined DIM_NETHER && !defined DIM_END && !defined SKY_SHADOWS
	float maxShadowDist = shadowDistance * 0.97;
	float distSq = dot(viewPos, viewPos);
	if (distSq > maxShadowDist * maxShadowDist) return fallback;
	if (abs(shadowClip.x) > 1.45 || abs(shadowClip.y) > 1.45) return fallback;
	vec3 p = distortShadow(shadowClip) * 0.5 + 0.5;
	if (p.x <= 0.001 || p.x >= 0.999 || p.y <= 0.001 || p.y >= 0.999 || p.z >= 1.0) return fallback;
	float z = p.z - 0.00005;
	float s;
	if (SHADOW_SMOOTH >= 2 || shadowMapResolution <= 512) {
		// 4 hardware-filtered taps (each already 2x2 bilinear) in a rotated grid. No noise, soft edges. Always used at 512.
		float t = 1.0 / float(shadowMapResolution);
		s = (shadowLookup(p.xy + vec2(-0.4, -1.3) * t, z) + shadowLookup(p.xy + vec2(1.3, -0.4) * t, z)
		   + shadowLookup(p.xy + vec2(0.4, 1.3) * t, z) + shadowLookup(p.xy + vec2(-1.3, 0.4) * t, z)) * 0.25;
	} else if (SHADOW_SMOOTH == 1) {
		// 2 hardware-filtered taps, rotated per pixel
		float t = 1.15 / float(shadowMapResolution);
		float a = fract(dot(gl_FragCoord.xy, vec2(0.0671, 0.00584)) * 52.98) * 6.2832;
		vec2 r1 = vec2(cos(a), sin(a)) * t;
		s = (shadowLookup(p.xy + r1, z) + shadowLookup(p.xy - r1, z)) * 0.5;
	} else {
		s = shadowLookup(p.xy, z);
	}
	float fade = smoothstep(shadowDistance * 0.8, maxShadowDist, sqrt(distSq));
	return mix(s, fallback, fade);
#else
	return fallback;
#endif
}

void main() {
	vec4 texel = texture(gtexture, texcoord);
	vec4 tex = texel * glcolor;
	if (tex.a < alphaTestRef) discard;

	vec3 albedoG = tex.rgb;   // gamma space: Complementary multiplies colors and light in gamma space, then converts
	float alpha = tex.a;
#ifdef ENTITY
	albedoG = mix(albedoG, entityColor.rgb, entityColor.a);
#endif
#if defined TERRAIN && !defined DIM_NETHER && !defined DIM_END
	if (blockId == ID_PLANT || blockId == ID_LEAVES) {
		// Complementary Reimagined foliage tweak: left/right shading across the sprite
		if (blockId == ID_PLANT) albedoG *= 0.97 - 0.1 * signMid.x;
#if GRASS_STYLE == 1
		if (blockId == ID_PLANT) {
			float gl0 = luma(albedoG);
			albedoG = clamp(mix(vec3(gl0), albedoG, 1.18), 0.0, 1.0);   // less washed-out pale, more tan/orange like CR
		}
#endif
	}
	{
		// Grass (plants and grass-block tops): large soft patches of lush / dry color, fine speckle, richer green.
		float gDom = albedoG.g - 0.5 * (albedoG.r + albedoG.b);
		bool grassy = GRASS_VARIATION > 0.0 && (blockId == ID_PLANT || blockId == ID_LEAVES || glcolor.b < 0.98) && gDom > 0.02;
		if (grassy) {
			vec2 wxz = worldPosV.xz + worldPosV.y * 0.5;
			float big = textureLod(noisetex, fract(wxz * (0.012 / 256.0 * 8.0) + 0.17), 0.0).r;
			float mid = textureLod(noisetex, fract(floor(wxz * 4.0) * (0.25 / 256.0 * 3.0) + 0.63), 0.0).g;
			float lush = (big - 0.5) * 1.6 + (mid - 0.5) * 0.5;
			vec3 lushTint = vec3(0.88, 1.04, 0.86);      // deeper, bluer-green
			vec3 dryTint  = vec3(1.04, 1.00, 0.90);      // sunlit yellow-green
			vec3 tintM = mix(lushTint, dryTint, clamp(lush * 0.5 + 0.5, 0.0, 1.0));
			albedoG *= mix(vec3(1.0), tintM, 0.35 * GRASS_VARIATION * (blockId == ID_LEAVES ? 0.5 : 1.0));
			albedoG *= 1.0 + (mid - 0.5) * 0.16 * GRASS_VARIATION;
			float gl = luma(albedoG);
			albedoG = mix(vec3(gl), albedoG, 1.0 + 0.04 * GRASS_VARIATION);   // a bit more color
			albedoG = clamp(albedoG, 0.0, 1.0);
		}
	}
#endif
#if defined TOON_MODE && defined TERRAIN && !defined DIM_NETHER && !defined DIM_END
	if (blockId == ID_LEAVES && TOON_LEAF_SHADING > 0.0) {
		// CazToon leaf shading: light yellow-green on the outside of the tree, darker blue-green the deeper you go.
		// Depth in the canopy = how much sky light the leaf has lost (vanilla: 1 light level per leaf block).
		float lvl = clamp((1.0 - lmcoord.y) * 15.0, 0.0, 4.0);
		vec3 lc = albedoG * vec3(1.06, 1.06, 0.78);
		lc = mix(lc, albedoG * vec3(0.78, 0.95, 0.66), smoothstep(0.65, 1.25, lvl));
		lc = mix(lc, albedoG * vec3(0.58, 0.78, 0.55), smoothstep(1.65, 2.25, lvl));
		lc = mix(lc, albedoG * vec3(0.42, 0.62, 0.49), smoothstep(2.65, 3.25, lvl));
		lc = mix(lc, albedoG * vec3(0.30, 0.48, 0.42), smoothstep(3.55, 4.0, lvl));
		albedoG = clamp(mix(albedoG, lc, min(TOON_LEAF_SHADING, 1.0)), 0.0, 1.0);
	} else if (blockId != ID_LEAVES && blockId != ID_PLANT && blockId != ID_WATER && TOON_GRASS_PATCHES > 0.0) {
		// Grass blocks: blocky light/dark patches, cooler in the dark ones
		float gAmt = max(toonGreen(albedoG, 0.015, 0.075, 0.035, 0.12), toonGreen(glcolor.rgb, 0.018, 0.070, 0.060, 0.160));
		if (gAmt > 0.01) {
			float pv = toonGrassPatch(worldPosV, mat3(gbufferModelViewInverse) * normalize(normalV));
			albedoG *= mix(1.0, max(1.0 + (pv - 1.0) * TOON_GRASS_PATCHES, 0.0), gAmt);
			float cool = clamp((1.0 - pv) / 0.17, 0.0, 1.0) * gAmt * min(TOON_GRASS_PATCHES, 1.0);
			albedoG *= mix(vec3(1.0), vec3(0.78, 0.90, 1.22), cool * 0.55);
			albedoG = clamp(albedoG, 0.0, 1.0);
		}
	}
#endif
	vec3 albedo = toLinear(albedoG);

	bool isWater = false;
	float foam = 0.0;   // shore foam, also used to switch reflections off on the foam
#ifdef WATER
	isWater = blockId == ID_WATER;
	if (isWater) {
	#if defined WATER_REIMAGINED
		// Complementary Reimagined (style 1): vanilla water texture squared, tinted by the biome color.
		vec3 gc = glcolor.rgb;
		gc.g = max(gc.g, 0.39);
		vec3 glM = sqrt(gc) * vec3(1.0, 0.85, 0.8);
		vec3 colPM = texel.rgb * texel.rgb;
		colPM = mix(vec3(0.25), colPM, WATER_TEXTURE_DETAIL);   // 1.0 = unchanged
		vec3 tint = mix(colPM * glM * WATER_BRIGHTNESS, vec3(0.03, 0.33, 0.50), WATER_COLOR_MIX);
		// Slow noise breathing through the color, on the 16-pixel block grid
		vec3 wP = (gbufferModelViewInverse * vec4(viewPos, 1.0)).xyz + cameraPosition;
		vec2 wGrid = floor(wP.xz * 16.0) * 0.0625;
		vec2 wWind = vec2(0.0, -frameTimeCounter * WATER_SPEED * 0.018);
		float wNoise = (textureLod(noisetex, (0.032 * (wGrid + wP.y * 2.0) + wWind) * 0.25, 0.0).g - 0.5) * 0.25;
		tint = pow(max(tint, vec3(0.0)), vec3(1.0 + wNoise));
		float detail = 1.0;
	#elif defined WATER_REALISTIC
		// Complementary Unbound (style 3): flat body color. Biome color pulled toward a dull blue-teal: biome color pulled toward a dull blue-teal, always readable.
		vec3 gc = glcolor.rgb;
		gc.g = max(gc.g, 0.39);
		vec3 body = sqrt(gc) * vec3(1.0, 0.85, 0.8) * 0.375;
		vec3 tint = toLinear(body) * 1.8;
		tint = mix(tint, vec3(0.03, 0.33, 0.50), WATER_COLOR_MIX);
		float detail = clamp(1.0 + (luma(texel.rgb) - 0.78) * 0.6 * WATER_TEXTURE_DETAIL, 0.8, 1.2);
	#else
		vec3 tint = mix(toLinear(glcolor.rgb), vec3(0.03, 0.33, 0.50), WATER_COLOR_MIX);
		float detail = clamp(1.0 + (luma(texel.rgb) - 0.78) * 2.0 * WATER_TEXTURE_DETAIL, 0.6, 1.4);
	#endif
		albedoG = clamp(tint * detail, 0.0, 1.0);
		albedo = toLinear(albedoG);

		// Opacity from depth: shallow water is clear, deep water is solid, and it turns solid
		// toward the horizon (Fresnel). Same idea as Complementary's water alpha.
		vec2 suv = gl_FragCoord.xy / vec2(viewWidth, viewHeight);
		float d1 = texture(depthtex1, suv).r;
		float thick = 100.0;
		float yDif = -100.0;   // how far the bottom under this pixel sits above (+) or below (-) the water surface
		if (d1 < 1.0) {
			vec4 pT = gbufferProjectionInverse * vec4(vec3(suv, d1) * 2.0 - 1.0, 1.0);
			pT.xyz /= pT.w;
			thick = max(length(pT.xyz) - length(viewPos), 0.0);
			yDif = (gbufferModelViewInverse * vec4(pT.xyz, 1.0)).y - (gbufferModelViewInverse * vec4(viewPos, 1.0)).y;
		}
		float waterFog = 1.0 - exp(-thick * 0.075 * WATER_DEPTH_FADE);
	#ifdef WATER_REIMAGINED
		if (isEyeInWater == 1) {
			alpha = 0.22;
		} else {
			alpha = sqrt(alpha) * (WATER_OPACITY + (1.0 - WATER_OPACITY) * waterFog);   // texture alpha, clearer in the shallows
		}
	#else
		if (isEyeInWater == 1) {
			alpha = 0.22;
		} else {
			alpha = 0.98 * (WATER_OPACITY + (1.0 - WATER_OPACITY) * waterFog);
		}
	#endif
	#if defined WATER_REIMAGINED
		if (isEyeInWater != 1) {
			// Complementary BRIGHT_CAVE_WATER: only brighten in dark caves, not outdoors
			float wf = sqrt(sqrt(max(waterFog, 0.0)));
			albedoG *= clamp(2.5 - wf - 1.5 * lmcoord.y, 1.0, 2.0);
			albedo = toLinear(albedoG);
		}
	#endif
		float nlw = length(normalV);
		// Inverted Iris water normal workaround: flips normal when viewed from underwater
		vec3 normWater = dot(normalV, viewPos) > 0.0 ? -normalV : normalV;
		float fr = nlw > 0.01 ? clamp(1.0 + dot(normWater / nlw, normalize(viewPos)), 0.0, 1.0) : 0.0;
		float fr2 = fr * fr;
		alpha = mix(alpha, 1.0, fr2 * fr2);
	#if defined WATER_REIMAGINED && !defined DIM_NETHER
		// Shore foam: only on flat top faces, where the bottom comes up to meet the surface.
		if (WATER_FOAM > 0.0 && isEyeInWater != 1 && nlw > 0.01 && d1 < 1.0 && dot(normalV / nlw, normalize(upPosition)) > 0.99) {
			float fThr = min(pow2f(dot(colPM, colPM)) * 1.6, 1.2);
			foam = pow2f(clamp((fThr + yDif) / fThr, 0.0, 1.0));
		#ifdef DIM_END
			foam *= 0.6;
		#else
			foam *= 0.4 + 0.25 * lmcoord.y;
		#endif
			foam *= clamp((fract(wP.y) - 0.7) * 10.0, 0.0, 1.0);
			vec3 foamColor = vec3(0.9, 0.95, 1.05);
			foam *= min(WATER_FOAM, 1.0);
			foamColor *= max(WATER_FOAM, 1.0);
			albedoG = mix(albedoG, foamColor, foam);
			albedo = toLinear(albedoG);
			alpha = mix(alpha, 1.0, foam);
		}
	#endif

	}
#endif
	bool foliage = blockId == ID_PLANT || blockId == ID_LEAVES;

	vec3 L = normalize(shadowLightPosition);
	float nl = length(normalV);
	vec3 n = nl > 0.01 ? normalV / nl : L;
	float NdotL = dot(n, L);
	float skyL = lmcoord.y;
	float blockL = lmcoord.x;
#ifdef DYNAMIC_LIGHTS
	// Light from whatever is in your hands: fades out over (light level) blocks, like a placed torch.
	float held = float(max(heldBlockLightValue, heldBlockLightValue2)) * DYNAMIC_LIGHT_RANGE;
	if (held > 0.5) blockL = max(blockL, clamp((held - length(viewPos)) / 15.0, 0.0, 1.0));
#endif
	vec3 color;
	float shadow = 0.0;

#if defined DIM_NETHER || defined DIM_END
	float diffuse = max(NdotL, 0.0);
	vec3 nW = mat3(gbufferModelViewInverse) * n;
	float face = nW.y > 0.5 ? 1.0 : nW.y < -0.5 ? 0.5 : (abs(nW.z) > abs(nW.x) ? 0.8 : 0.62);
	if (nl <= 0.01) face = 1.0;
  #if defined DIM_NETHER
	vec3 light = (vec3(0.32, 0.17, 0.10) * 0.65 + torchLight(blockL)) * face;
  #else
	vec3 light = (vec3(0.24, 0.19, 0.34) * 0.75 + torchLight(blockL)) * face;
  #endif
	light += vec3(0.014) + nightVision * 0.45;
	color = albedo * light;
#else
	// ---- Complementary Reimagined lighting (its DoLighting): everything in gamma space,
	// ---- sun and sky light mixed as sqrt(block + scene^2 + min), then converted to linear.
	float sunVis = clamp(mqSunDir.y + 0.0625, 0.0, 0.125) / 0.125;
	float sunVis2 = sunVis * sunVis;
	float noonFactor = sqrt(max(sin(sunAngle * 6.28318530718), 0.0));
	float stVal = abs(sunVis - 0.5) * 2.0;
	float shadowTime = (stVal * stVal) * (stVal * stVal);
	float rain = rainStrength;
	float lmY2 = skyL * skyL;
	float lmYM = skyL * skyL * (3.0 - 2.0 * skyL);
	bool foliageL = blockId == ID_LEAVES || blockId == ID_PLANT;

	vec3 upV = normalize(upPosition);
	float NdotU = dot(n, upV);
	float absE = abs(dot(n, normalize(gbufferModelView[0].xyz)));
	float absNdotE2 = absE * absE;
	float absNdotN = abs(dot(n, normalize(gbufferModelView[2].xyz)));
	if (nl <= 0.01) { NdotU = 1.0; absNdotE2 = 0.0; absNdotN = 0.0; }
#ifdef ENTITY
	// Faces turned away from the sun get no direct light at all (was a flat 0.25), so the shadow lookup can't flicker on/off there
	// while the model animates (running in third person).
	float NdotLM = 0.25 * smoothstep(-0.45, 0.05, NdotL) + 0.75 * max(NdotL + 0.4, 0.0) * 0.714;
#ifdef TOON_MODE
	NdotLM = mix(NdotLM, 0.7, TOON_FLAT_ENTITIES * 0.6);   // flatter lighting on mobs, items and players
#endif
#else
	// Low sun: only foliage on the sun's side (backlit) catches the light; the rest stays dim. Noon stays even.
	float VdotLf = dot(normalize(viewPos), L);
#if GRASS_STYLE == 1
	float foliFacing = 1.0;   // Complementary: grass is lit evenly (always like the ground), no dimming on the far side
#else
	float foliFacing = mix(1.0, mix(0.6, 1.0, smoothstep(-0.2, 0.9, VdotLf)), 1.0 - noonFactor);
#endif
	float NdotLM = foliageL ? foliFacing : max(NdotL + 0.4, 0.0) * 0.714;   // Complementary SIDE_SHADOWING
#endif

	shadow = NdotLM > 0.0001 ? getShadow(skyL) : 0.0;
#ifdef ENTITY
	// Faces turned away from the sun: don't use the shadow map at all (it flickers there while the model animates).
	shadow = mix(1.0, shadow, smoothstep(-0.15, 0.05, NdotL));
#endif
#ifdef TOON_MODE
	shadow = mix(1.0, shadow, TOON_SHADOW_STRENGTH);   // CazToon: shadows stay lighter than normal
#endif
	float skyMask = smoothstep(0.05, 0.4, skyL);
	float shadowMult = shadow * skyMask * max(NdotLM * shadowTime, 0.0);
#if GRASS_STYLE == 1
	if (foliageL) shadowMult *= 0.82;   // Complementary: foliage never gets full-strength sun, so shadows on grass stay soft
#endif

	// Block light
	vec3 blockLighting = vec3(0.0);
	if (blockL > 0.005) {
		float bl2 = blockL * blockL;
		float bl4 = bl2 * bl2;
		float lxS = (bl4 * bl4) * (2.8 - 0.3 + 1.0);
		float lxC = blockL * (2.8 + 0.3 - 1.0);
		float lightmapXM = pow(lxS + lxC, 2.25);
		blockLighting = lightmapXM * vec3(0.1775, 0.104, 0.077) * TORCH_BRIGHTNESS;
		float lxFactor = (sunVis2 * 0.4 + (0.6 - 0.6 * (1.0 - noonFactor) * (1.0 - noonFactor))) * (6.0 - 5.0 * rain);
		lxFactor *= lmY2 + lmY2 * 2.0 * shadowMult * shadowMult;
		blockLighting *= pow(lightmapXM / 60.0 + 0.001, 0.09 * lxFactor);
	}

	// Sun and sky light
	float ambientMult = mix(lmYM, lmYM * lmYM * lmYM, rain);
	vec3 lightColorM = mqLightG * SUN_BRIGHTNESS;
	vec3 ambientColorM = mqAmbG;
#ifdef TOON_MODE
	// CazToon: cool blue shadows, a touch of warmth in direct sun (only where the sky can be seen)
	ambientColorM *= mix(vec3(1.0), vec3(0.85, 0.97, 1.20), TOON_SHADOW_TINT * skyMask);
	lightColorM *= mix(vec3(1.0), vec3(1.06, 1.00, 0.90), TOON_SHADOW_TINT * skyMask);
#endif
	float rainLF = 0.1 * rain;
	float lightFogTweaks = 1.0 + max(96.0 - length(viewPos), 0.0) * (0.002 * (1.0 - sunVis2) + 0.0104 * rainLF) - rainLF;
	ambientMult *= lightFogTweaks;
	lightColorM *= lightFogTweaks;

	float directionShade = (0.75 + NdotU * 0.25) * (1.0 - 0.1 * absNdotE2) * (1.0 + 0.075 * absNdotN);
	if (isWater && isEyeInWater == 1) directionShade = 1.0;
#if defined TOON_MODE && defined ENTITY
	directionShade = mix(directionShade, 1.0, TOON_FLAT_ENTITIES);
#endif
	lightColorM *= 1.0 + absNdotE2 * 0.75;
	float nf2 = noonFactor * noonFactor;
	float nf4 = nf2 * nf2;
	float nf8 = nf4 * nf4;
	float noonFactor20 = nf8 * nf8 * nf4;
	lightColorM *= 1.0 + (foliageL ? 0.0 : 1.0) * noonFactor20 * (absNdotN * absNdotN * 0.8 - absNdotE2 * 0.2);
	ambientColorM = mix(ambientColorM, lightColorM, 0.05 * absNdotN * lmY2);   // fake bounced light

	vec3 sceneLighting = lightColorM * shadowMult + ambientColorM * ambientMult;
	vec3 minLighting = (0.005625 + 0.5 * 0.043) * vec3(0.45, 0.475, 0.6) * (1.0 - lmYM) + nightVision * vec3(0.5, 0.5, 0.75);

	vec3 finalDiffuse = sqrt(max(directionShade * directionShade * (blockLighting + sceneLighting * sceneLighting + minLighting), vec3(0.0)));
	vec3 litColorG = albedoG * finalDiffuse;
#if defined FOLIAGE_SSS && !defined ENTITY
	if (foliageL && isEyeInWater == 0) {
		// Complementary subsurface highlight: bright and warm when the sun is behind the leaf, toward the camera
#if GRASS_STYLE == 1
		// Complementary Reimagined: narrow, natural glow only when looking almost straight at the sun
		float sssF = pow(max(VdotLf, 0.0), 10.0) * 0.8;
		litColorG += albedoG * lightColorM * shadowMult * sssF * (1.0 - 0.85 * rain) * 0.5;
#else
		float v2 = pow2f(max(VdotLf, 0.0)), v4 = v2 * v2;
		float sssF = min(v4 * v2 * (1.0 + 0.5 * (1.0 - noonFactor)), 0.8);   // wider and stronger toward sunset
		vec3 hlC = normalize(pow(max(lightColorM, vec3(1e-4)), vec3(0.37))) * (0.3 + 1.5 * sunVis2) * (1.0 - 0.85 * rain);
		litColorG += albedoG * hlC * shadowMult * sssF * (blockId == ID_PLANT ? 0.4 : 0.3);
#endif
	}
#endif

#ifdef WATER
	if (isWater && isEyeInWater != 1) {
	#ifdef SUN_REFLECTION
		float NdotL_w = max(dot(n, L), 0.0);
		if (NdotL_w > 0.001 && skyL > 0.05) {
			vec3 nView = normalize(viewPos);
			vec3 H = normalize(L - nView);
			float NdotH = max(dot(n, H), 0.0);
			float LdotH = clamp(dot(L, H), 0.0, 1.0);
			float NdotH2 = NdotH * NdotH;

			// Roughness 0.137 matches Complementary Reimagined water (smoothness 0.5)
			const float roughness = 0.137;
			float denom = NdotH2 * (roughness - 1.0) + 1.0;
			float D = roughness / (3.14159265 * denom * denom);

			// Schlick Fresnel
			float F = exp2((-5.55473 * LdotH - 6.98316) * LdotH) * 0.95 + 0.05;

			float spec = max(NdotL_w * D * F / max(LdotH * LdotH, 0.01), 0.0);
			spec = spec / (0.125 * spec + 1.0);

			vec3 hlCol = normalize(pow(max(lightColorM, vec3(1e-4)), vec3(0.37)));

			// Complementary Reimagined streak modulation:
			// ONLY the moving animated ripples/dashes of the vanilla water texture glisten!
			#if defined WATER_REIMAGINED
				float pat = dot(texel.rgb, texel.rgb) * 0.4;
				float streak = min(pow2f(pow2f(pat)), 0.5);
			#else
				float streak = 0.12;
			#endif

			float nlw_hl = length(normalV);
			float fr_hl = nlw_hl > 0.01 ? clamp(1.0 + dot(normalV / nlw_hl, nView), 0.0, 1.0) : 0.0;
			float fr2_hl = fr_hl * fr_hl;

			float hlMult = streak * (16.0 - 15.0 * fr2_hl) * (sunVis > 0.5 ? 0.85 : 0.425) * (1.0 - 0.85 * rain);
			vec3 waterGlint = (isEyeInWater != 1 ? shadowMult : pow(max(shadowMult, 0.0), 0.25) * 0.35) * spec * hlCol * hlMult * (1.0 - foam);
			litColorG += waterGlint;
		}
	#endif
	}
#endif

	color = toLinear(litColorG);
#endif

	// Glowing blocks keep their brightness and feed the bloom.
	if (blockId == ID_EMISSIVE) color += albedo * (0.6 + luma(albedo) * 4.0);
	if (blockId == ID_LAVA) color = albedo * 4.0;



#ifdef SHADOW_DEBUG
	{
		vec3 p = distortShadow(shadowClip) * 0.5 + 0.5;
		bool inside = shadowClip.x < 1.5 && p.x > 0.0 && p.x < 1.0 && p.y > 0.0 && p.y < 1.0 && p.z < 1.0;
		if (!inside) color = mix(color, vec3(0.0, 0.2, 1.0), 0.5);
		else color = mix(color, vec3(1.0, 0.0, 0.0), (1.0 - shadow) * 0.6);
	}
#endif
	color = color * fogMA.a + fogMA.rgb;   // distance fog + mist, worked out per vertex
	color *= (1.0 - blindness) * (1.0 - darknessFactor * 0.7);
	outColor = vec4(color, alpha);

#ifdef WATER
	// For the composite pass: is this water, and how much sky it sees.
	// alpha must stay 1.0, otherwise blending would mix this data with what's behind the water
	// Moving water (flowing, sloped or falling) has a tilted surface: no reflection there. Only flat, still water reflects.
	vec3 gN = normalize(cross(dFdx(viewPos), dFdy(viewPos)));
	float flatWater = smoothstep(0.9975, 0.9995, abs(dot(gN, normalize(upPosition))));
	outWater = isWater ? vec4(0.5 + 0.5 * skyL, (1.0 - foam) * flatWater, 0.0, 1.0) : vec4(0.0, 0.0, 0.0, 1.0);
#endif
}

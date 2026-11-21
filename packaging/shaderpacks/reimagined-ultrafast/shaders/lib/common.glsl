// ============================================================
//  Mystique - shared uniforms and lighting
//  All colors here are linear HDR; tonemapping happens at the end.
// ============================================================

uniform vec3 sunPosition;
uniform vec3 moonPosition;
uniform vec3 upPosition;
uniform vec3 shadowLightPosition;
uniform vec3 cameraPosition;
uniform vec3 relativeEyePosition;
uniform vec3 fogColor;
uniform vec3 skyColor;
uniform mat4 gbufferModelView;
uniform mat4 gbufferModelViewInverse;
uniform mat4 gbufferProjection;
uniform mat4 gbufferProjectionInverse;
uniform mat4 shadowModelView;
uniform mat4 shadowProjection;
uniform float rainStrength;
uniform float frameTimeCounter;
uniform float viewWidth;
uniform float viewHeight;
uniform float near;
uniform float far;
uniform float nightVision;
uniform float blindness;
uniform float darknessFactor;
uniform int isEyeInWater;
uniform int worldTime;
uniform ivec2 eyeBrightnessSmooth;
uniform float sunAngle;

#define ID_PLANT 10001
#define ID_WATER 10002
#define ID_EMISSIVE 10003
#define ID_LAVA 10004
#define ID_LEAVES 10005

vec3 toLinear(vec3 c) {
	// sRGB -> linear with a cheap polynomial (no pow: pow is 2 slow ops on mobile GPUs)
	c = max(c, vec3(0.0));
	return c * (c * (c * 0.305306011 + 0.682171111) + 0.012522878);
}
float luma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

// ------------------------------------------------------------ per-frame values
// Sun/sky/ambient colors only depend on uniforms, so they are worked out ONCE per vertex
// (mqSetup) and handed to the fragment stage as flat values. The old code redid all of it
// for every single pixel (several normalize + smoothstep chains per pixel).
#ifdef MQ_VERT
vec4 mqZenDay;   // rgb = zenith color,   a = day factor
vec4 mqHorSet;   // rgb = horizon color,  a = sunset factor
vec4 mqSunDir;   // xyz = sun direction in world space, w = sky day factor
vec3 mqAmb;      // ambient light color
vec3 mqDir;      // direct sun / moon color
flat out vec4 mqZenDayO;
flat out vec4 mqHorSetO;
flat out vec4 mqSunDirO;
flat out vec3 mqAmbO;
flat out vec3 mqDirO;
flat out vec3 mqLightGO;   // Complementary light color (gamma space, used by terrain lighting)
flat out vec3 mqAmbGO;     // Complementary ambient color (gamma space)
#else
flat in vec4 mqZenDayO;
flat in vec4 mqHorSetO;
flat in vec4 mqSunDirO;
flat in vec3 mqAmbO;
flat in vec3 mqDirO;
flat in vec3 mqLightGO;
flat in vec3 mqAmbGO;
#define mqZenDay mqZenDayO
#define mqHorSet mqHorSetO
#define mqSunDir mqSunDirO
#define mqAmb mqAmbO
#define mqDir mqDirO
#endif
#ifdef MQ_VERT
vec3 mqLightG;
vec3 mqAmbG;
#else
#define mqLightG mqLightGO
#define mqAmbG mqAmbGO
#endif

/** 0 = night, 1 = day. */
float dayFactor() { return mqZenDay.w; }
/** Strongest when the sun is near the horizon. */
float sunsetFactor() { return mqHorSet.w; }
/** Golden hour amount: strongest at sunrise, lighter at sunset, 0 by day / night / rain. */
float goldenAmount() {
	float side = smoothstep(-0.2, 0.6, cos(sunAngle * 6.28318530718));
	return clamp(GOLDEN_HOUR * sunsetFactor() * mix(0.25, 1.0, side) * (1.0 - rainStrength * 0.7), 0.0, 1.5);
}
/** Color of direct sun or moon light. */
vec3 directLightColor() { return mqDir; }
/** Light coming from the whole sky (fills in shadows). */
vec3 ambientColor() { return mqAmb; }

/** Warm torch / lantern light from the block light level (0..1). */
vec3 torchLight(float bl) {
	float k = bl * bl * bl * (0.45 + 0.55 * bl) * 2.6 + bl * 0.08;   // ~ pow(bl, 3.2) without the pow
	return vec3(1.0, 0.56, 0.24) * k * TORCH_BRIGHTNESS;
}

/** Complementary Reimagined sky (its GetSky), in gamma space. dir = world-space direction. */
float cSqrt3(float x) { return sqrt(sqrt(sqrt(x))); }
vec3 skyGamma(vec3 dir, bool glare) {
	vec3 sunW = mqSunDir.xyz;
	float SdotU = sunW.y;
	float sunFactor = SdotU < 0.0 ? clamp(SdotU + 0.375, 0.0, 0.75) / 0.75 : clamp(SdotU + 0.03125, 0.0, 0.0625) / 0.0625;
	float sunVis = clamp(SdotU + 0.0625, 0.0, 0.125) / 0.125;
	float noonFactor = sqrt(max(sin(sunAngle * 6.28318530718), 0.0));
	float nightFactor = max(sin(sunAngle * -6.28318530718), 0.0);
	float invNoon = 1.0 - noonFactor, invNoon2 = invNoon * invNoon;
	float rain = rainStrength, rain2 = rain * rain, invRain = 1.0 - rain;
	float VdotU = dir.y, VdotS = dot(dir, sunW);

	vec3 skyColorSqrt = sqrt(skyColor);
	float invRain2 = invRain * invRain;
	vec3 skyColorM = mix(max(skyColorSqrt, vec3(0.63, 0.67, 0.73)), skyColorSqrt, invRain2);
	vec3 skyColorM2 = mix(max(skyColor, sunFactor * vec3(0.265, 0.295, 0.35)), skyColor, invRain2);
	vec3 noonUp = pow(skyColorM, vec3(2.9)) * (vec3(0.85, 0.92, 0.81) + rain * vec3(0.1, 0.0, 0.1));
	vec3 noonMid = skyColorM * (vec3(1.4, 1.3, 1.3) + rain * (vec3(-0.1, -0.4, -0.6) + vec3(0.0, 0.06, 0.12) * noonFactor)) + noonUp * 0.25;
	vec3 noonDown = skyColorM * (vec3(0.9) + rain * (vec3(-0.15, -0.3, -0.42) + vec3(0.0, 0.02, 0.08) * noonFactor)) + noonUp * 0.25;
	vec3 sunsetUp = skyColorM2 * (vec3(0.72, 0.522, 0.47) + vec3(0.1, 0.2, 0.35) * rain2);
	vec3 sunsetMid = skyColorM2 * (vec3(1.8, 1.3, 1.2) + vec3(0.15, 0.25, -0.05) * rain2);
	vec3 sunsetDownP = vec3(1.45, 0.86, 0.5) - vec3(0.8, 0.3, 0.0) * rain;
	vec3 sunsetDown = sunsetDownP * 0.5 + 0.25 * sunsetMid;
	vec3 dayUp = mix(noonUp, sunsetUp, invNoon2);
	vec3 dayMid = mix(noonMid, sunsetMid, invNoon2);
	vec3 dayDown = mix(noonDown, sunsetDown, invNoon2);
	vec3 nightCol = 0.9 * vec3(0.07, 0.14, 0.24) * (1.0 - 0.5 * rain) + skyColor;
	vec3 nightUp = pow(nightCol, vec3(0.90)) * 0.45;
	vec3 nightMid = sqrt(nightUp) * 0.65;
	vec3 nightDown = nightMid * vec3(0.82, 0.82, 0.88);

	float nfS2 = sqrt(sqrt(nightFactor));
	float nfM = sqrt(sqrt(nfS2)) * 0.4;
	float VdotSM1 = pow(max(VdotS, 0.0), 2.0);
	float VdotSM2 = VdotSM1 * VdotSM1;
	float VdotSM3 = pow(max(-VdotS, 0.0), 4.0);
	float VdotUmax0 = max(VdotU, 0.0);

	vec3 upColor = mix(nightUp * (1.5 - 0.5 * nfS2 + nfM * VdotSM3 * 1.5), dayUp, sunFactor);
	vec3 middleColor = mix(nightMid * (3.0 - 2.0 * nfS2), dayMid * (1.0 + VdotSM2 * 0.3), sunFactor);
	vec3 downColor = mix(nightDown, dayDown, (sunFactor + sunVis) * 0.5);

	float u1 = (1.0 - VdotUmax0) * (1.0 - VdotUmax0);
	u1 = pow(u1, (1.0 - VdotSM2 * 0.4) * (1.15 - 0.15 * rain2));
	u1 = mix(u1, 1.0, rain2 * 0.15);
	vec3 sky = mix(upColor, middleColor, u1);

	float u2 = (1.0 - abs(VdotU)) * (1.0 - abs(VdotU));
	u2 = u2 * u2 * (3.0 - 2.0 * u2);
	u2 *= (0.7 - nfM + VdotSM1 * (0.3 + nfM)) * invNoon * sunFactor;
	sky = mix(sky, sunsetDownP * (1.0 + VdotSM1 * 0.3), u2 * invRain);

	float u3 = min(max(-VdotU + 0.08, 0.0) / 0.35, 1.0);
	u3 = u3 * u3 * (3.0 - 2.0 * u3);
	vec3 scat = vec3(u3 * u3, sqrt(u3), cSqrt3(u3));
	scat = mix(vec3(u3), scat, 0.75 - 0.5 * rain);
	sky = mix(sky, downColor, scat);

	if (glare) {
		float VdotSML = sunVis > 0.5 ? VdotS : -VdotS;
		if (VdotSML > 0.0) {
			float shadowTime = pow(abs(sunVis - 0.5) * 2.0, 4.0);
			float gs = 3.0 * (2.0 - clamp(VdotS * 1000.0, 0.0, 1.0)) * (1.0 - 0.75 * rain2);
			float m4 = pow(abs(VdotS), gs);
			float g = 0.075 / (1.0 - 0.925 * m4) - 0.075;
			g *= 0.7 * (1.0 - 0.8 * rain);
			vec3 gc = mix(vec3(0.38, 0.4, 0.5) * 0.3, vec3(1.5, 0.7, 0.3) + vec3(0.0, 0.5, 0.5) * noonFactor, sunVis);
			sky += gc * g * shadowTime;
		}
	}
	return max(sky, vec3(0.0));
}

/** Sky color for a world-space direction, linear HDR. Used for fog, reflections and clouds. */
vec3 skyGradient(vec3 dir) {
	return toLinear(skyGamma(dir, false));
}

#ifdef MQ_VERT
void mqSetup() {
	vec3 sunV = normalize(sunPosition);
	float e = dot(sunV, normalize(upPosition));          // sun height: 1 overhead, 0 horizon
	float day = smoothstep(-0.12, 0.22, e);
	float set = (1.0 - smoothstep(0.0, 0.38, abs(e))) * smoothstep(-0.3, 0.0, e);

	// ambient
	vec3 dayAmb = vec3(0.40, 0.60, 1.0) * 0.52;
	vec3 setAmb = vec3(0.62, 0.46, 0.52) * 0.45;
	vec3 nightAmb = vec3(0.05, 0.075, 0.16) * 0.32 * NIGHT_BRIGHTNESS;
	vec3 amb = mix(nightAmb, dayAmb, day);
	amb = mix(amb, setAmb, set * 0.6);
	amb = mix(amb, vec3(luma(amb)) * 0.8, rainStrength * 0.7);

	// direct light
	vec3 col;
	if (e > 0.0) {
		vec3 golden = vec3(1.0, 0.52, 0.22) * 2.3;
		vec3 noon = vec3(1.0, 0.95, 0.86) * 3.2;
		col = mix(golden, noon, smoothstep(0.05, 0.45, e)) * SUN_BRIGHTNESS;
	} else {
		col = vec3(0.30, 0.42, 0.80) * 0.16 * NIGHT_BRIGHTNESS;
	}
	col *= smoothstep(0.0, 0.07, abs(e));                // smooth hand-over between sun and moon
	col *= 1.0 - 0.85 * rainStrength;

	// sky colors
	vec3 sunW = normalize(mat3(gbufferModelViewInverse) * sunPosition);
	float skyDay = smoothstep(-0.15, 0.25, sunW.y);
	vec3 dayZen = vec3(0.10, 0.30, 0.80) * 0.62;
	vec3 dayHor = vec3(0.36, 0.70, 1.0) * 0.72;
	vec3 setZen = vec3(0.14, 0.20, 0.48) * 0.45;
	vec3 setHor = vec3(1.0, 0.30, 0.10) * 0.85;
	vec3 nightZen = vec3(0.004, 0.008, 0.024);
	vec3 nightHor = vec3(0.02, 0.030, 0.075);
	vec3 zen = mix(mix(nightZen, dayZen, skyDay), setZen, set * 0.6);
	vec3 hor = mix(mix(nightHor, dayHor, skyDay), setHor, set * 0.85);

	// Complementary Reimagined light and ambient colors (gamma space; the terrain shader mixes them its way)
	{
		float sv = clamp(e + 0.0625, 0.0, 0.125) / 0.125;
		float sv2 = sv * sv;
		float noonF = sqrt(max(sin(sunAngle * 6.28318530718), 0.0));
		float invN = 1.0 - noonF;
		float vsb = 0.5;
		vec3 noonL = vec3(0.65, 0.55, 0.375) * 2.05;
		vec3 setL = pow(vec3(0.64, 0.45, 0.3), vec3(1.5 + invN)) * 5.0;
		vec3 nightL = 0.9 * vec3(0.15, 0.14, 0.20) * (0.4 + vsb * 0.4);
		vec3 noonA = pow(skyColor, vec3(0.75)) * 0.85;
		vec3 setA = noonA * vec3(1.21, 0.92, 0.76) * 0.95;
		vec3 nightA = 0.9 * vec3(0.09, 0.12, 0.17) * (1.55 + vsb * 0.77);
		vec3 clearL = mix(nightL, mix(setL, noonL, noonF), sv2);
		vec3 clearA = mix(nightA, mix(setA, noonA, noonF), sv2);
		vec3 dayRainL = vec3(0.21, 0.16, 0.13) * 0.85 + noonF * vec3(0.0, 0.02, 0.06);
		vec3 nightRainL = vec3(0.03, 0.035, 0.05) * (0.5 + 0.5 * vsb);
		vec3 dayRainA = vec3(0.2, 0.2, 0.25) * (1.8 + 0.5 * vsb);
		vec3 nightRainA = vec3(0.16, 0.20, 0.3) * (0.75 + 0.6 * vsb);
		vec3 rainL = mix(nightRainL, dayRainL * 0.6, sv2) * 2.5;
		vec3 rainA = mix(nightRainA, dayRainA * 1.4, sv2);
		mqLightG = mix(clearL, rainL, rainStrength);
		mqAmbG = mix(clearA, rainA, rainStrength);
		// Golden hour: warm tint on sunlit surfaces and shadows while the sun is near the horizon.
		// Strongest at sunrise (cos > 0 side of the day), lighter at sunset which was already warm.
		float sunriseSide = smoothstep(-0.2, 0.6, cos(sunAngle * 6.28318530718));
		float gold = GOLDEN_HOUR * set * mix(0.25, 1.0, sunriseSide) * (1.0 - rainStrength * 0.7);
		gold = clamp(gold, 0.0, 1.5);
		mqLightG *= mix(vec3(1.0), vec3(1.35, 0.9, 0.5), gold) * (1.0 + 0.2 * gold);
		mqAmbG *= mix(vec3(1.0), vec3(1.25, 0.93, 0.66), gold);
		mqLightGO = mqLightG;
		mqAmbGO = mqAmbG;
	}
	mqZenDay = vec4(zen, day);
	mqHorSet = vec4(hor, set);
	mqSunDir = vec4(sunW, skyDay);
	mqAmb = amb;
	mqDir = col;
	mqZenDayO = mqZenDay;
	mqHorSetO = mqHorSet;
	mqSunDirO = mqSunDir;
	mqAmbO = mqAmb;
	mqDirO = mqDir;
}

/** Distance fog + night mist for one vertex, as  color * fogMul + fogAdd  (interpolated to the pixels). */
void fogTerms(vec3 vp, out float fogMul, out vec3 fogAdd) {
	fogMul = 1.0;
	fogAdd = vec3(0.0);
	if (isEyeInWater != 0) return;                       // underwater fog is done later, in composite
	float dist = length(vp);
	vec3 dirW = mat3(gbufferModelViewInverse) * (vp / max(dist, 1e-4));
#if defined DIM_NETHER || defined DIM_END
	vec3 fogCol = toLinear(fogColor);
	float amount = smoothstep(far * 0.35, far, dist);
#else
	vec3 fogCol = skyGradient(dirW);
	fogCol *= mix(vec3(1.0), vec3(1.3, 0.95, 0.65), goldenAmount() * 0.8);   // warm golden haze instead of pink-lavender
	float border = smoothstep(far * 0.62, far, dist);
	float setF = sunsetFactor();
	float sunSide = max(dot(dirW, mqSunDir.xyz), 0.0);          // haze piles up toward the sun
	float haze = (1.0 - exp(-dist * (0.0008 + 0.0012 * setF * (0.5 + 0.5 * sunSide)) * ATMOSPHERIC_FOG)) * (0.10 + 0.6 * rainStrength + 0.22 * setF);
	float amount = max(border, haze);
#endif
	amount = clamp(amount, 0.0, 1.0);
	fogMul = 1.0 - amount;
	fogAdd = fogCol * amount;

#if !defined DIM_NETHER && !defined DIM_END
	// Night / dawn mist: thick low down, thin higher up.
	if (MIDNIGHT_MIST > 0.0) {
		float t = float(worldTime % 24000);
		float dawn = max(smoothstep(22000.0, 23500.0, t), 1.0 - smoothstep(0.0, 1500.0, t));
		float when = max((1.0 - dayFactor()) * 0.6, dawn * 0.12);   // sunrise mist stays very light so the golden light shows
		if (when > 0.01) {
			float camY = cameraPosition.y;
			float pointY = camY + dirW.y * dist;
			float midY = (camY + pointY) * 0.5;
			float low = exp(-max(midY - float(MIST_THICKNESS), 0.0) / 6.0);
			float mist = (1.0 - exp(-dist * 0.02 * MIDNIGHT_MIST * low)) * when * (1.0 - rainStrength * 0.5);
			vec3 nightMist = ambientColor() * 1.6 + vec3(0.004, 0.006, 0.01);
			vec3 dawnMist = skyGradient(normalize(vec3(dirW.x, 0.05, dirW.z))) * 0.32;
			dawnMist = mix(dawnMist, vec3(1.0, 0.6, 0.3) * (0.25 + 0.4 * sunSide), clamp(setF * 1.5, 0.0, 1.0));   // golden sunrise haze
			vec3 mistCol = mix(nightMist, dawnMist, clamp(dawn * dayFactor() * 2.0, 0.0, 1.0));
			float m = clamp(mist, 0.0, 0.5);
			fogAdd = fogAdd * (1.0 - m) + mistCol * m;
			fogMul *= 1.0 - m;
		}
	}
#endif
}
#endif

// ------------------------------------------------------------ shadows

/** How strongly the shadow map is squeezed toward the player. A small map (512) gets more squeeze, so shadows near you stay sharp. */
float shadowD() { return shadowMapResolution <= 512 ? SHADOW_DISTORTION_512 : 0.85; }

vec3 distortShadow(vec3 clip) {
	// Extra resolution near the player, less far away (gentler than before = less blotchy far shadows)
	float sd = shadowD();
	float factor = length(clip.xy) * sd + (1.0 - sd);
	clip.xy /= factor;
	clip.z *= 0.5;
	return clip;
}

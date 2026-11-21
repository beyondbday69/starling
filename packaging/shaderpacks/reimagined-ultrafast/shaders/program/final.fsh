// Copies the finished 8-bit picture to the screen (composite1 already dithered it).
#include "/lib/settings.glsl"
#include "/lib/color_grading.glsl"

uniform sampler2D colortex1;
uniform sampler2D depthtex0;
uniform sampler2D depthtex1;
uniform sampler2D depthtex2;
uniform mat4 gbufferProjectionInverse;
uniform mat4 gbufferModelViewInverse;
uniform mat4 gbufferPreviousModelView;
uniform mat4 gbufferPreviousProjection;
uniform vec3 cameraPosition;
uniform vec3 previousCameraPosition;
uniform vec3 relativeEyePosition;
uniform float viewWidth;
uniform float viewHeight;
uniform float near;
uniform float far;
uniform int isEyeInWater;
uniform float frameTimeCounter;
uniform float rainStrength;

in vec2 texcoord;

layout(location = 0) out vec4 outColor;

// Linear (view-space) distance from a depth buffer value, in blocks.
float toonLinZ(float d) {
	return (2.0 * near * far) / (far + near - (d * 2.0 - 1.0) * (far - near));
}

// True if this pixel belongs to the player character or hand in any camera mode (FPP, TPB, TPF, or camera mods)
bool isPlayerPixel(vec2 coord, float d0) {
	if (d0 >= 0.9999) return false; // sky is never player

	float d1 = textureLod(depthtex1, coord, 0.0).r;
	float d2 = textureLod(depthtex2, coord, 0.0).r;

	// 1. Hand / held items in first-person (FPP)
	// gbuffers_hand writes to depthtex0, but never to depthtex1.
	if (d0 < 0.56 || d0 < d1 - 0.00002) return true;

	// Reconstruct view and world position for 3D checks
	vec4 clip = vec4(coord * 2.0 - 1.0, d0 * 2.0 - 1.0, 1.0);
	vec4 vP = gbufferProjectionInverse * clip;
	vec3 vPos = vP.xyz / vP.w;
	float zDist = length(vPos);

	// 2. Third-person player check via relativeEyePosition (TPF & TPB)
	// relativeEyePosition is the vector from camera to player eye position.
	float relEyeLen = length(relativeEyePosition);
	if (relEyeLen > 0.15) {
		vec3 worldOffset = (gbufferModelViewInverse * vec4(vPos, 1.0)).xyz;
		vec3 pDiff = worldOffset - relativeEyePosition;
		// Player cylinder relative to eye position:
		// Radius 1.45m covers player body, arms, held items/weapons, armor, elytra.
		// Vertical: -2.3m to feet/boots, +0.65m to top of head/helmet.
		if (length(pDiff.xz) < 1.45 && pDiff.y > -2.3 && pDiff.y < 0.65) {
			return true;
		}
	} else {
		// First-person mode: anything held close in front of camera
		bool isEntity = (d0 < d2 - 0.00002) || (d1 < d2 - 0.00002);
		if (isEntity && zDist < 1.35) return true;
	}

	// 3. Fallback for third-person camera mods where relativeEyePosition might not be updated:
	// In third-person, the player is an entity in front of the camera (distance < 5.5 blocks) in the screen center area
	bool isEntity = (d0 < d2 - 0.00002) || (d1 < d2 - 0.00002);
	if (isEntity && zDist < 5.5) {
		vec2 centerDiff = abs(coord - 0.5);
		if (centerDiff.x < 0.35 && centerDiff.y < 0.45) return true;
	}

	return false;
}

#if defined TOON_MODE && TOON_OUTLINES > 0
// 1 = draw the outline, 0 = skip it. Green foliage (grass, leaves) never gets outlined
float toonNotFoliage(vec3 c) {
	c = clamp(c, 0.0, 1.0);
	float mx = max(c.r, max(c.g, c.b));
	float mn = min(c.r, min(c.g, c.b));
	float d = mx - mn;
	float sat = d / max(mx, 0.0001);
	float h = 0.0;
	if (d > 0.0001) {
		if (mx == c.r) h = mod((c.g - c.b) / d, 6.0);
		else if (mx == c.g) h = (c.b - c.r) / d + 2.0;
		else h = (c.r - c.g) / d + 4.0;
		h /= 6.0;
	}
	float gDom = c.g - max(c.r, c.b);
	float gRatio = c.g / max(0.0001, (c.r + c.b) * 0.5);
	float hd = abs(h - 0.3333333);
	hd = min(hd, 1.0 - hd);
	float m = (1.0 - smoothstep(0.12, 0.24, hd)) * smoothstep(0.15, 0.45, sat) * smoothstep(0.05, 0.25, mx)
	        * smoothstep(0.02, 0.12, gDom) * smoothstep(1.05, 1.45, gRatio);
	return 1.0 - clamp(m, 0.0, 1.0);
}
#endif

void main() {
	vec2 uv = texcoord;
	vec3 col = vec3(0.0);
	bool haveCol = false;
#ifdef UNDERWATER_DISTORTION
	if (isEyeInWater == 1) {
		uv += 0.0007 * sin((texcoord.x + texcoord.y) * 25.0 + frameTimeCounter * 3.0);
		uv = clamp(uv, vec2(0.001), vec2(0.999));
	}
#endif
#ifdef MOTION_BLUR
	// Camera motion blur: rebuild where this pixel was last frame, blur along that line. Smooths 30 fps.
	float depth = textureLod(depthtex0, texcoord, 0.0).r;
	// Never blur the player, hand, held items in ANY camera mode (FPP, TPB, TPF, or camera mods)
	bool isPlayerCenter = isPlayerPixel(texcoord, depth);
	if (!isPlayerCenter) {
		vec4 clip = vec4(texcoord * 2.0 - 1.0, depth * 2.0 - 1.0, 1.0);
		vec4 vP = gbufferProjectionInverse * clip;
		vec3 worldPos = (gbufferModelViewInverse * vec4(vP.xyz / vP.w, 1.0)).xyz + cameraPosition;
		vec4 pV = gbufferPreviousModelView * vec4(worldPos - previousCameraPosition, 1.0);
		vec4 pC = gbufferPreviousProjection * pV;
		if (pC.w > 0.0) {
			vec2 vel = (texcoord - (pC.xy / pC.w * 0.5 + 0.5)) * MOTION_BLUR_STRENGTH;
			float vl = length(vel);
			if (vl > 0.0004) {
				vel *= min(vl, 0.045) / vl;                   // cap the blur length
				float dither = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
				vec3 centre = textureLod(colortex1, uv, 0.0).rgb;
				vec3 sum = vec3(0.0);
				for (int i = 0; i < MOTION_BLUR_SAMPLES; i++) {
					float t = (float(i) + dither) / float(MOTION_BLUR_SAMPLES) - 0.5;
					vec2 tuv = clamp(uv + vel * t, vec2(0.001), vec2(0.999));
					float dTap = textureLod(depthtex0, tuv, 0.0).r;
					vec3 tc = textureLod(colortex1, tuv, 0.0).rgb;
					// Never smear the player or hand into the surrounding world blur
					if (isPlayerPixel(tuv, dTap)) tc = centre;
					sum += tc;
				}
				col = sum / float(MOTION_BLUR_SAMPLES);
				haveCol = true;
			}
		}
	}
#endif
	if (!haveCol) col = texture(colortex1, uv).rgb;

#ifdef TOON_MODE
	// ---- Toon look, CazToon style: light sharpen, richer colour, coloured depth-edge outlines ----
	vec2 px = 1.0 / vec2(viewWidth, viewHeight);
	if (TOON_SHARPEN > 0.0) {
		vec3 nb = texture(colortex1, uv + vec2(px.x, 0.0)).rgb + texture(colortex1, uv - vec2(px.x, 0.0)).rgb
		        + texture(colortex1, uv + vec2(0.0, px.y)).rgb + texture(colortex1, uv - vec2(0.0, px.y)).rgb;
		col += (col - nb * 0.25) * (TOON_SHARPEN * 2.0);
	}
	col = clamp(mix(vec3(dot(col, vec3(0.2126, 0.7152, 0.0722))), col, TOON_SATURATION), 0.0, 1.0);
#if TOON_BANDS > 0
	{
		float lum = dot(col, vec3(0.2126, 0.7152, 0.0722));
		float bandPos = lum * float(TOON_BANDS);
		float edgeSoft = smoothstep(0.38, 0.62, fract(bandPos));
		float qLum = (floor(bandPos) + edgeSoft) / float(TOON_BANDS);
		col *= qLum / max(lum, 0.001);
	}
#endif
#if TOON_OUTLINES > 0
	{
		ivec2 ip = ivec2(gl_FragCoord.xy);
		ivec2 lim = textureSize(depthtex0, 0) - 1;
		float d0 = texelFetch(depthtex0, ip, 0).r;
		// Sky and the held item never get outlines
		bool isHandPx = d0 < 0.56 || texelFetch(depthtex1, ip, 0).r < texelFetch(depthtex2, ip, 0).r - 0.00002;
		if (d0 < 0.9999 && (TOON_OUTLINE_PLAYER > 0 || !isHandPx)) {
			int r = TOON_OUTLINE_SIZE;
			float edge = 0.0;
			float zC = toonLinZ(d0);

			if (isHandPx) {
				// Player first-person hand & held weapons/items outline
				float dA = texelFetch(depthtex0, clamp(ip + ivec2(-r, -r), ivec2(0), lim), 0).r;
				float dB = texelFetch(depthtex0, clamp(ip + ivec2( r,  r), ivec2(0), lim), 0).r;
				float dD = texelFetch(depthtex0, clamp(ip + ivec2(-r,  r), ivec2(0), lim), 0).r;
				float dE = texelFetch(depthtex0, clamp(ip + ivec2( r, -r), ivec2(0), lim), 0).r;

				// Hand boundary against the world
				float maxDiff = max(max(dA - d0, dB - d0), max(dD - d0, dE - d0));
				if (max(max(dA, dB), max(dD, dE)) > 0.56) {
					edge = 1.0;
				} else if (maxDiff > 0.012) {
					// Internal crease between fingers / hand and held tool
					edge = smoothstep(0.010, 0.028, maxDiff);
				}
			} else {
				float rawA = texelFetch(depthtex0, clamp(ip + ivec2(-r, -r), ivec2(0), lim), 0).r;
				float rawB = texelFetch(depthtex0, clamp(ip + ivec2( r,  r), ivec2(0), lim), 0).r;
				float rawD = texelFetch(depthtex0, clamp(ip + ivec2(-r,  r), ivec2(0), lim), 0).r;
				float rawE = texelFetch(depthtex0, clamp(ip + ivec2( r, -r), ivec2(0), lim), 0).r;

				float zA = toonLinZ(rawA);
				float zB = toonLinZ(rawB);
				float zD = toonLinZ(rawD);
				float zE = toonLinZ(rawE);
				float zr = max(zC, 1.0);

				bool isSkyBorder = max(max(rawA, rawB), max(rawD, rawE)) >= 0.9999;

		#if TOON_OUTLINES == 1
				// Standard: thin line on both sides of a depth jump
				edge = smoothstep(0.12, 0.30, abs((zA + zB + zD + zE) - 4.0 * zC) / zr);
		#else
				// Dungeons (CazToon): line on the NEAR object where a diagonal neighbour is much farther
				float qA = zA - zC, qB = zB - zC, qD = zD - zC, qE = zE - zC;
				float farthest = max(max(qA, qB), max(qD, qE));

				// Properly handle thin foreground features (player limbs, head, weapons, fences):
				float spreadA = (qA < 0.0 || qB < 0.0) ? abs(qA - qB) : max(qA, qB);
				float spreadD = (qD < 0.0 || qE < 0.0) ? abs(qD - qE) : max(qD, qE);
				float spread = max(spreadA, spreadD);

				edge = smoothstep(0.10, 0.22, farthest / zr) * smoothstep(0.05, 0.12, spread / zr);
		#endif

				// Distance fade: completely eliminate thick outlines on distant trees, hills, and chunk borders
				// Sky borders fade out fast by 20 blocks; world edges fade out completely past 40 blocks.
				float distFade = isSkyBorder ? (1.0 - smoothstep(12.0, 24.0, zC)) : (1.0 - smoothstep(22.0, 42.0, zC));
				edge *= distFade;
			}

			if (edge > 0.01) {
				float e = edge * (isHandPx ? 1.0 : (1.0 - smoothstep(22.0, 42.0, zC))) * (1.0 - rainStrength);
				e *= toonNotFoliage(col);
				col *= 1.0 + e * TOON_OUTLINE_BRIGHTNESS;
				float ol = dot(col, vec3(0.299, 0.587, 0.114));
				vec3 vivid = max(mix(vec3(ol), col, max(TOON_OUTLINE_SATURATION, 0.0)), vec3(0.0));
				col = mix(col, vivid, clamp(e, 0.0, 1.0));
			}
		}
	}
#endif
	col = clamp(col, 0.0, 1.0);
#endif
#ifdef COLOR_GRADING_ENABLED
	col = applyCazToonColorGrading(col, uv);
#endif
	outColor = vec4(col, 1.0);
}

// ============================================================================
// =====                  CAZTOON COLOR GRADING SYSTEM                    =====
// ============================================================================
// Ported from CazToon: Cinema-grade tonal grading, style presets (Tropical,
// Cyberpunk, Fantasy, etc.), temperature/tint, intelligent vibrance, and split toning.

#if defined COLOR_GRADING_ENABLED

vec3 cgRgb2Hsv(vec3 c) {
	vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
	vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
	vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
	float d = q.x - min(q.w, q.y);
	float e = 1.0e-10;
	return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 cgHsv2Rgb(vec3 c) {
	vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
	vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
	return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

float cgLuma(vec3 c) {
	return dot(c, vec3(0.299, 0.587, 0.114));
}

float cgMax(vec3 c) {
	return max(max(c.r, c.g), c.b);
}

float cgBrightnessVal(vec3 c) {
	float lum = cgLuma(c);
	float mx = cgMax(c);
	return max(lum, mx * 0.70);
}

vec3 cgTemperature(vec3 col, float temp) {
	float lum = cgLuma(col);
	vec3 warm = vec3(1.0, 0.85, 0.65);
	vec3 cool = vec3(0.65, 0.85, 1.0);
	vec3 tCol = mix(cool, warm, temp * 0.5 + 0.5);
	return clamp(mix(col, col * tCol, abs(temp) * 0.4 * smoothstep(0.0, 0.1, lum)), 0.0, 1.0);
}

vec3 cgTint(vec3 col, float tint) {
	float lum = cgLuma(col);
	vec3 magenta = vec3(1.0, 0.85, 1.0);
	vec3 green = vec3(0.85, 1.0, 0.85);
	vec3 tCol = mix(green, magenta, tint * 0.5 + 0.5);
	return clamp(mix(col, col * tCol, abs(tint) * 0.4 * smoothstep(0.0, 0.1, lum)), 0.0, 1.0);
}

vec3 cgExposure(vec3 col, float ev) {
	vec3 g = pow(max(col, vec3(0.0)), vec3(2.2));
	g *= pow(2.0, ev);
	return clamp(pow(clamp(g, 0.0, 4.0), vec3(1.0 / 2.2)), 0.0, 1.0);
}

vec3 cgBrightnessAdj(vec3 col, float b) {
	return clamp(col * (1.0 + b * 0.5), 0.0, 1.0);
}

vec3 cgContrastAdj(vec3 col, float c) {
	return clamp((col - 0.5) * (1.0 + c) + 0.5, 0.0, 1.0);
}

vec3 cgBlacks(vec3 col, float blacks) {
	float br = cgBrightnessVal(col);
	float mask = 1.0 - smoothstep(0.0, 0.35, br);
	return max(col + blacks * 0.10 * mask, vec3(0.0));
}

vec3 cgWhites(vec3 col, float whites) {
	float br = cgBrightnessVal(col);
	float mask = smoothstep(0.65, 1.0, br);
	return clamp(col + whites * 0.10 * mask, 0.0, 1.0);
}

vec3 cgShadowsHighlights(vec3 col, float shadows, float highlights) {
	float br = cgBrightnessVal(col);
	float shadowMask = 1.0 - smoothstep(0.0, 0.5, br);
	float highlightMask = smoothstep(0.5, 1.0, br);
	return clamp(col + vec3(shadows * 0.20 * shadowMask + highlights * 0.20 * highlightMask), 0.0, 1.0);
}

vec3 cgSplitToning(vec3 col, vec3 shadowTint, vec3 highlightTint, float intensity, float balance) {
	float br = cgBrightnessVal(col);
	float splitMid = 0.5 + balance * 0.3;
	float shadowMask = 1.0 - smoothstep(0.0, splitMid, br);
	float highlightMask = smoothstep(splitMid, 1.0, br);
	vec3 tint = mix(shadowTint * shadowMask, highlightTint * highlightMask, 0.5);
	return clamp(mix(col, col + (tint - 0.5) * col, intensity), 0.0, 1.0);
}

vec3 cgLiftGammaGain(vec3 col, vec3 lift, vec3 gamma, vec3 gain) {
	vec3 c = col * gain;
	vec3 l = (lift - 1.0) * (1.0 - c);
	c += l * 0.5;
	c = pow(max(c, vec3(0.001)), 1.0 / max(gamma, vec3(0.001)));
	return clamp(c, 0.0, 1.0);
}

vec3 cgVibrance(vec3 col, float vib) {
	float mx = max(max(col.r, col.g), col.b);
	float mn = min(min(col.r, col.g), col.b);
	float sat = mx > 0.001 ? (mx - mn) / mx : 0.0;
	float curve = pow(1.0 - sat, 1.5);
	float lum = cgLuma(col);
	vec3 res = mix(vec3(lum), col, max(1.0 + vib * curve, 0.0));
	return clamp(res, 0.0, 1.0);
}

vec3 cgHueShift(vec3 col, float hue) {
	vec3 hsv = cgRgb2Hsv(col);
	hsv.x = fract(hsv.x + hue / 360.0);
	return cgHsv2Rgb(hsv);
}

vec3 cgStylePreset(vec3 col, int preset, float intensity) {
	if (preset <= 0) return col;
	vec3 outCol = col;
	if (preset == 1) { // Cinematic (warm highlights, cool shadows, slight desaturation)
		vec3 sTint = vec3(0.08, 0.1, 0.18);
		vec3 hTint = vec3(0.22, 0.18, 0.12);
		outCol = cgSplitToning(col, sTint, hTint, 0.35, 0.1);
		outCol = cgVibrance(outCol, -0.15);
		outCol = cgLiftGammaGain(outCol, vec3(0.98, 0.98, 1.02), vec3(1.0), vec3(1.05, 1.02, 0.98));
	} else if (preset == 2) { // Vintage (warm sepia tones, reduced contrast)
		outCol = cgTemperature(col, 0.4);
		outCol = cgVibrance(outCol, -0.3);
		outCol = cgLiftGammaGain(outCol, vec3(1.05, 1.02, 0.95), vec3(0.95), vec3(0.95, 0.93, 0.88));
		float lum = cgLuma(outCol);
		vec3 sepia = vec3(lum * 1.1, lum * 0.95, lum * 0.75);
		outCol = mix(outCol, sepia, 0.2);
	} else if (preset == 3) { // Cyberpunk (teal & orange, high contrast)
		vec3 sTint = vec3(0.0, 0.25, 0.35);
		vec3 hTint = vec3(0.35, 0.2, 0.05);
		outCol = cgSplitToning(col, sTint, hTint, 0.5, 0.0);
		outCol = cgVibrance(outCol, 0.4);
		outCol = (outCol - 0.5) * 1.15 + 0.5;
		outCol = cgLiftGammaGain(outCol, vec3(0.95, 1.0, 1.08), vec3(1.0), vec3(1.08, 1.0, 0.92));
	} else if (preset == 4) { // Horror (desaturated, greenish tint, deep shadows)
		outCol = cgVibrance(col, -0.5);
		outCol = cgTint(outCol, -0.3);
		outCol = cgLiftGammaGain(outCol, vec3(0.9, 0.95, 0.9), vec3(1.1), vec3(0.9, 0.92, 0.88));
		outCol = max(outCol - 0.05, vec3(0.0));
	} else if (preset == 5) { // Fantasy (vivid, purple shadows, golden highlights)
		vec3 sTint = vec3(0.18, 0.08, 0.28);
		vec3 hTint = vec3(0.28, 0.22, 0.1);
		outCol = cgSplitToning(col, sTint, hTint, 0.4, 0.15);
		outCol = cgVibrance(outCol, 0.35);
		outCol = cgLiftGammaGain(outCol, vec3(1.0, 0.95, 1.1), vec3(0.95), vec3(1.1, 1.05, 0.95));
	} else if (preset == 6) { // Nordic (cold, desaturated, deep shadows)
		outCol = cgTemperature(col, -0.35);
		outCol = cgVibrance(outCol, -0.25);
		vec3 sTint = vec3(0.1, 0.12, 0.22);
		vec3 hTint = vec3(0.18, 0.18, 0.2);
		outCol = cgSplitToning(outCol, sTint, hTint, 0.3, -0.2);
		outCol = cgLiftGammaGain(outCol, vec3(0.95, 0.97, 1.05), vec3(1.05), vec3(0.98, 1.0, 1.05));
	} else if (preset == 7) { // Tropical (CazToon Signature Look: warm sunlight, vibrant greens & blues)
		outCol = cgTemperature(col, 0.2);
		outCol = cgVibrance(outCol, 0.4);
		outCol.g = pow(max(outCol.g, 0.0), 0.9);
		outCol.b = pow(max(outCol.b, 0.0), 0.95);
		outCol = cgLiftGammaGain(outCol, vec3(1.0, 1.03, 1.02), vec3(0.95), vec3(1.02, 1.05, 1.08));
	} else if (preset == 8) { // Noir (heavy desaturation, high contrast)
		outCol = cgVibrance(col, -0.7);
		outCol = cgTemperature(outCol, -0.15);
		outCol = (outCol - 0.5) * 1.25 + 0.5;
		outCol = cgLiftGammaGain(outCol, vec3(0.9, 0.9, 0.95), vec3(1.1), vec3(1.1, 1.08, 1.05));
	}
	return mix(col, clamp(outCol, 0.0, 1.0), intensity);
}

vec3 applyCazToonColorGrading(vec3 col, vec2 uv) {
	vec3 c = clamp(col, 0.0, 1.0);

#if CG_STYLE_PRESET > 0
	c = cgStylePreset(c, CG_STYLE_PRESET, CG_STYLE_INTENSITY);
#endif

#if CG_TEMPERATURE != 0.0
	c = cgTemperature(c, CG_TEMPERATURE);
#endif

#if CG_TINT != 0.0
	c = cgTint(c, CG_TINT);
#endif

#if CG_EXPOSURE != 0.0
	c = cgExposure(c, CG_EXPOSURE);
#endif

#if CG_BRIGHTNESS != 0.0
	c = cgBrightnessAdj(c, CG_BRIGHTNESS);
#endif

#if CG_CONTRAST != 0.0
	c = cgContrastAdj(c, CG_CONTRAST);
#endif

#if CG_BLACKS != 0.0
	c = cgBlacks(c, CG_BLACKS);
#endif

#if CG_WHITES != 0.0
	c = cgWhites(c, CG_WHITES);
#endif

#if CG_SHADOWS != 0.0 || CG_HIGHLIGHTS != 0.0
	c = cgShadowsHighlights(c, CG_SHADOWS, CG_HIGHLIGHTS);
#endif

#if CG_HUE_SHIFT != 0.0
	c = cgHueShift(c, CG_HUE_SHIFT);
#endif

#if CG_SATURATION != 0.0
	{
		float lum = cgLuma(c);
		c = mix(vec3(lum), c, max(1.0 + CG_SATURATION, 0.0));
	}
#endif

#if CG_VIBRANCE != 0.0
	c = cgVibrance(c, CG_VIBRANCE);
#endif

#ifdef CG_SPLIT_TONING_ENABLED
	{
		vec3 sTint = vec3(CG_SHADOW_TINT_R, CG_SHADOW_TINT_G, CG_SHADOW_TINT_B);
		vec3 hTint = vec3(CG_HIGHLIGHT_TINT_R, CG_HIGHLIGHT_TINT_G, CG_HIGHLIGHT_TINT_B);
		c = cgSplitToning(c, sTint, hTint, CG_SPLIT_TONING_INTENSITY, CG_SPLIT_TONING_BALANCE);
	}
#endif

	return c;
}

#endif

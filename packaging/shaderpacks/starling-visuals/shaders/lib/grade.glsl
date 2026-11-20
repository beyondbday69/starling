// Exposure, saturation, vibrance, tonemap, contrast, vignette. Input linear HDR, output display 0..1.
vec3 aces(vec3 x) {
	return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

#if GRADE_STYLE == 1
// Complementary Reimagined's grade: modified Lottes tonemap (with dark-color lift, path to white and dark
// desaturation), then its saturation step. Lottes 2016, "Advanced Techniques and Optimization of HDR Color Pipelines".
vec3 compTonemap(vec3 color) {
	float lum0 = luma(color);
	float a = TM_CONTRAST;
	float hdrMaxA = pow(8.0, a), midInA = pow(0.25, a);          // constants, folded by the compiler
	float HM2 = (hdrMaxA - midInA) * 0.25;
	float b = (hdrMaxA * 0.25 - midInA) / HM2;
	float c = (hdrMaxA * midInA - hdrMaxA * 0.25 * midInA) / HM2;
	vec3 p = pow(max(color, vec3(1e-6)), vec3(a));
	vec3 outC = p / (p * b + c);
	vec3 sq = sqrt(outC);
	outC = sq * (1.138 - 0.138 * sq);                             // linear -> display (cheap gamma)

	// Dark colors keep their natural look (less tonemapping)
	vec3 sc = sqrt(color);
	vec3 smoothC = sc * (1.138 - 0.138 * sc);
	float darkLift = smoothstep(0.1, 0.0, lum0);
	outC = mix(outC, smoothC, darkLift * 0.75 * max(0.55 - abs(1.05 - TM_CONTRAST), 0.0) / 0.55);

	// Path to white
	float modL = pow(max(lum0 / 16.0, 1e-6), 2.0 - TM_WHITE_PATH) * 16.0;
	outC = mix(outC, vec3(1.0), smoothstep(0.0, 16.0, modL));

	// Desaturate dark colors
	outC = mix(outC, vec3(luma(outC)), smoothstep(0.1, 0.0, lum0) * TM_DARK_DESATURATION);
	return clamp(outC, 0.0, 1.0);
}
#endif

/** Cool blue grade: whole picture leans blue-teal, shadows and haze most of all. Display-space color in and out. */
vec3 blueTint(vec3 c) {
	float l = luma(c);
	c *= mix(vec3(1.0), vec3(0.90, 0.99, 1.10), BLUE_TINT);
	c += vec3(-0.012, 0.004, 0.03) * (1.0 - l) * BLUE_TINT;
	return c;
}

vec3 gradeColor(vec3 color, float eyeSky, float day, vec2 uv) {
	// A bit brighter at night and in caves so it's never pitch black.
#if GRADE_STYLE == 1
	float exposure = EXPOSURE;   // Complementary has no extra night or cave boost
#else
	float exposure = EXPOSURE * (1.0 + (1.0 - day) * 0.35 * eyeSky) * mix(1.6, 1.0, eyeSky);
#endif
	color *= exposure;

#if GRADE_STYLE == 1
	color = compTonemap(max(color, vec3(0.0)));
	{
		float sf = SATURATION + 0.07;                              // Complementary: T_SATURATION + 0.07, vibrance 1.0 (no-op)
		float gray = (color.r + color.g + color.b) * 0.3333333;
		if (sf < 1.0) gray = luma(color);
		color = color * sf - gray * (sf - 1.0);
	}
	color = blueTint(color);
	vec2 v0 = uv - 0.5;
	color *= 1.0 - VIGNETTE * dot(v0, v0) * 2.2;
	return clamp(color, 0.0, 1.0);
#endif

	// Saturation and vibrance: dull colors get the biggest boost.
	float l = luma(color);
	color = mix(vec3(l), color, SATURATION);
	float mx = max(color.r, max(color.g, color.b));
	float mn = min(color.r, min(color.g, color.b));
	float sat = mx > 1e-4 ? (mx - mn) / mx : 0.0;
	color = mix(vec3(luma(color)), color, 1.0 + VIBRANCE * (1.0 - sat));
	color = max(color, vec3(0.0));

	color = aces(color * 0.9);
	vec3 sq = sqrt(color);
	color = sq * (1.138 - 0.138 * sq);          // ~ pow(color, 1/2.2) with no pow
	color = (color - 0.5) * CONTRAST + 0.5;
	color = blueTint(color);

	vec2 v = uv - 0.5;
	color *= 1.0 - VIGNETTE * dot(v, v) * 2.2;
	return clamp(color, 0.0, 1.0);
}

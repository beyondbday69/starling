// Sun, moon and the End sky.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

uniform sampler2D gtexture;
uniform int renderStage;

in vec2 texcoord;
in vec4 glcolor;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
	vec4 c = texture(gtexture, texcoord) * glcolor;
	vec3 col = toLinear(c.rgb);
#if defined DIM_OVERWORLD
	vec4 pv = gbufferProjectionInverse * vec4(gl_FragCoord.xy / vec2(viewWidth, viewHeight) * 2.0 - 1.0, 1.0, 1.0);
	float VdotU = dot(normalize(mat3(gbufferModelViewInverse) * (pv.xyz / pv.w)), vec3(0.0, 1.0, 0.0));
	float horizon = smoothstep(0.0, 1.0, pow(1.0 + min(VdotU, 0.0), 2.0));      // sun and moon sink under the horizon
#else
	float horizon = 1.0;
#endif
	if (renderStage == MC_RENDER_STAGE_SUN) {
		// Complementary Reimagined sun: sharp hot core with a warm falloff instead of the flat vanilla square
		float sunVis = clamp(mqSunDir.y + 0.0625, 0.0, 0.125) / 0.125;
		float noonF = sqrt(max(sin(sunAngle * 6.28318530718), 0.0));
		float d = dot(c.rgb, c.rgb) * 0.45;
		float d2 = d * d, d3 = d2 * d;
		float core = d3 * d3;                                    // d^6 without pow
		float soft = mix(core, d2 * d, rainStrength);            // rain: wider, dimmer disc
		vec3 sunCol = vec3(1.1, 0.78, 0.36) * 3.0;
		sunCol = mix(sunCol, vec3(0.35) * 3.0, rainStrength * 0.75);
		col = vec3(soft) * sunCol * (0.25 + 0.75 * sunVis * sunVis + 0.5 * noonF);
		col += vec3(1.0, 0.92, 0.75) * core * 2.0 * (1.0 - rainStrength);   // white-hot center feeds the bloom
		col *= horizon;
		col *= 1.0 - rainStrength * 0.6;
	} else if (renderStage == MC_RENDER_STAGE_MOON) {
		float m = min(length(c.rgb), 1.0);
		col *= m * m * (3.0 - 2.0 * m) * 1.3 * 2.5 * horizon;
		col *= 1.0 - rainStrength * 0.9;
	}
	outColor = vec4(col, c.a);
}

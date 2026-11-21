// Sky gradient (vanilla stars kept, vanilla sunset fan replaced by our own glow).
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

uniform int renderStage;

in vec4 glcolor;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
	if (renderStage == MC_RENDER_STAGE_STARS) {
		outColor = vec4(toLinear(glcolor.rgb) * 2.0, glcolor.a);
		return;
	}
	if (renderStage == MC_RENDER_STAGE_SUNSET || renderStage == MC_RENDER_STAGE_VOID) discard;
	vec4 p = gbufferProjectionInverse * vec4(gl_FragCoord.xy / vec2(viewWidth, viewHeight) * 2.0 - 1.0, 1.0, 1.0);
	vec3 dirW = normalize(mat3(gbufferModelViewInverse) * (p.xyz / p.w));
	vec3 sg = skyGamma(dirW, true);
	sg *= smoothstep(0.0, 1.0, pow(1.0 + min(dirW.y, 0.0), 2.0));   // Complementary: sky ground fades darker below the horizon
	sg = toLinear(sg);

	outColor = vec4(sg, 1.0);
}

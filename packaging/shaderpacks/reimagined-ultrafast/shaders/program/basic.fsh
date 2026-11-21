// Untextured geometry: block outlines, leads, lightning, clouds.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

in vec4 glcolor;
in vec4 fogMA;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
	vec3 c = toLinear(glcolor.rgb);
#if defined LIGHTNING
	c *= 6.0;
#elif defined CLOUDS
	#ifdef BLOCKY_CLOUDS
	discard;
	#endif
	c = c * (ambientColor() * 1.2 + directLightColor() * 0.4) + c * 0.05;
	c = c * fogMA.a + fogMA.rgb;
#endif
	outColor = vec4(c, glcolor.a);
}

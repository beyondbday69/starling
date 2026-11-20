#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

out vec4 glcolor;
out vec4 fogMA;

void main() {
	glcolor = gl_Color;
	vec3 vp = (gl_ModelViewMatrix * gl_Vertex).xyz;
#ifdef CLOUDS
	mqSetup();
	float fm; vec3 fa;
	fogTerms(vp, fm, fa);
	fogMA = vec4(fa, fm);
#else
	fogMA = vec4(0.0, 0.0, 0.0, 1.0);
#endif
	gl_Position = ftransform();
}

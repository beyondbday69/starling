#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

in vec4 mc_Entity;

out vec2 texcoord;
out vec4 glcolor;
flat out int skip;
flat out int solid;

void main() {
	texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
	glcolor = gl_Color;
	int id = int(mc_Entity.x + 0.5);
	skip = id == ID_WATER ? 1 : 0;   // water doesn't block sunlight
#ifdef LEAF_SOLID_SHADOWS
	solid = id == ID_LEAVES ? 1 : 0;
#else
	solid = 0;
#endif
	gl_Position = ftransform();
	gl_Position.xyz = distortShadow(gl_Position.xyz);
}

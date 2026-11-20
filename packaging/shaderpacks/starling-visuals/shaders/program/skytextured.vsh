#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

out vec2 texcoord;
out vec4 glcolor;

void main() {
	mqSetup();
	texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
	glcolor = gl_Color;
	gl_Position = ftransform();
}

#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

out vec2 texcoord;

void main() {
	mqSetup();
	gl_Position = ftransform();
	texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
}

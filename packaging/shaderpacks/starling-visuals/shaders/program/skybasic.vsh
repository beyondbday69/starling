#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

out vec4 glcolor;

void main() {
	mqSetup();
	glcolor = gl_Color;
	gl_Position = ftransform();
}

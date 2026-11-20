#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

out vec2 texcoord;
out vec4 glcolor;
out vec3 viewPos;

void main() {
	texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
	glcolor = gl_Color;
	viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
	gl_Position = ftransform();
}

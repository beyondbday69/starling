uniform sampler2D gtexture;

in vec2 texcoord;
in vec4 glcolor;
flat in int skip;
flat in int solid;

layout(location = 0) out vec4 outColor;

void main() {
	if (skip == 1) discard;
	vec4 c = texture(gtexture, texcoord) * glcolor;
	if (solid == 0 && c.a < 0.1) discard;   // leaves are solid blocks for shadows     // leaves and grass cast leafy shadows
	outColor = c;
}

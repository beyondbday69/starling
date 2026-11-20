// Unlit textured things: beacon beams, enchantment glint, glowing eyes.
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

uniform sampler2D gtexture;
uniform float alphaTestRef = 0.1;

in vec2 texcoord;
in vec4 glcolor;
in vec3 viewPos;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
	vec4 c = texture(gtexture, texcoord) * glcolor;
	if (c.a < alphaTestRef) discard;
	outColor = vec4(toLinear(c.rgb) * 1.6, c.a);
}

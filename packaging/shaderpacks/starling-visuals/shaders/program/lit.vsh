// Terrain, water, entities, particles and hand: vertex stage.
#define MQ_VERT
#include "/lib/settings.glsl"
#include "/lib/common.glsl"

in vec4 mc_Entity;
in vec2 mc_midTexCoord;

out vec2 texcoord;
out vec2 lmcoord;
out vec4 glcolor;
out vec3 viewPos;
out vec3 normalV;
out vec3 shadowClip;
out vec2 signMid;               // -1..1 across a plant sprite (left/right shading, sun-through glow)
out vec3 worldPosV;
out vec4 fogMA;                 // rgb = fog to add, a = how much of the pixel color stays
flat out int blockId;

#ifdef WAVING_FOLIAGE
vec3 rawWave(vec3 pos, float wind) {
	float magnitude = sin(wind * 0.0027 + pos.x + pos.y) * 0.04 + 0.04;
	float d0 = sin(wind * 0.0127);
	float d1 = sin(wind * 0.0089);
	float d2 = sin(wind * 0.0114);
	return magnitude * vec3(sin(wind * 0.0224 + d1 + d2 + pos.x - pos.z + pos.y),
	                        sin(wind * 0.0015 + d2 + d0 + pos.x),
	                        sin(wind * 0.0063 + d0 + d1 - pos.x + pos.z + pos.y));
}
vec3 getWave(vec3 pos) {
	float wind = frameTimeCounter * 170.0 * WAVING_SPEED;
	vec3 w = rawWave(pos, wind);
	if (rainStrength > 0.01) w = mix(w, rawWave(pos, wind * 1.6), rainStrength);
	return w * WAVING_I * (1.0 + 0.6 * rainStrength);
}
#endif

void main() {
	mqSetup();
	texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
	vec2 lm = (gl_TextureMatrix[1] * gl_MultiTexCoord1).xy;
	lmcoord = clamp((lm * 16.0 - 0.5) / 15.0, 0.0, 1.0);
	glcolor = gl_Color;
	blockId = int(mc_Entity.x + 0.5);

	vec4 vp = gl_ModelViewMatrix * gl_Vertex;
	vec2 dMid = texcoord - mc_midTexCoord;
	signMid = vec2(dMid.x > 0.0 ? 1.0 : -1.0, dMid.y > 0.0 ? 1.0 : -1.0);
	worldPosV = (gbufferModelViewInverse * vp).xyz + cameraPosition;
#if (defined WAVING_FOLIAGE || defined PLAYER_PLANT_INTERACTION) && !defined DIM_NETHER && !defined DIM_END && defined TERRAIN
	if (blockId == ID_PLANT || blockId == ID_LEAVES) {
		vec3 pp = (gbufferModelViewInverse * vp).xyz;
		vec3 wp = pp + cameraPosition;
		float indoors = clamp(lmcoord.y - 0.87, 0.0, 0.1);     // no wind in caves and under roofs
		if (blockId == ID_PLANT) {
			// grounded foliage: roots stay planted, tops sway (Complementary DoWave_Foliage)
			bool isTop = gl_MultiTexCoord0.t < mc_midTexCoord.t || fract(wp.y + 0.21) > 0.26;
			if (isTop) {
			#ifdef WAVING_FOLIAGE
				vec3 w = getWave(vec3(wp.x, wp.y * 0.5, wp.z));
				pp += vec3(w.x * 3.0, 0.0, w.z * 8.0 + w.y * 4.0) * indoors;
			#endif
			#ifdef PLAYER_PLANT_INTERACTION
				// CazToon Player-Plant interaction: grass and flowers bend away when walked through
				float dP = length(pp.xz);
				float vertDist = abs(pp.y - (-1.0));
				float maxRadius = PLANT_INTERACTION_RADIUS;
				if (dP < maxRadius && vertDist < 1.8) {
					float tP = 1.0 - smoothstep(0.1, maxRadius, dP);
					float vertMask = 1.0 - smoothstep(0.8, 1.8, vertDist);
					vec2 pushDir = dP > 0.001 ? (pp.xz / dP) : vec2(0.0, 1.0);
					float bendStrength = tP * tP * vertMask * PLANT_INTERACTION_INTENSITY;
					pp.xz += pushDir * (0.55 * bendStrength);
					pp.y -= 0.25 * bendStrength;
				}
			#endif
			}
		} else {
		#ifdef WAVING_FOLIAGE
			vec3 w = getWave(wp * vec3(0.75, 0.375, 0.75)) * vec3(4.0, 3.0, 8.0);
			pp += w * indoors;
		#endif
		}
		vp = gbufferModelView * vec4(pp, 1.0);
		worldPosV = pp + cameraPosition;
	}
#endif
	viewPos = vp.xyz;
	{ float fm; vec3 fa; fogTerms(vp.xyz, fm, fa); fogMA = vec4(fa, fm); }   // fog per vertex, not per pixel
	normalV = gl_NormalMatrix * gl_Normal;      // not normalized: particles have no normal
	// Grass, flowers and crops are lit like the ground they stand on, so they match the grass block.
	if (blockId == ID_PLANT) normalV = normalize(upPosition);

	shadowClip = vec3(2.0);
#if defined SHADOWS && !defined NO_SHADOW && !defined DIM_NETHER && !defined DIM_END && !defined SKY_SHADOWS
	vec3 playerPos = (gbufferModelViewInverse * vp).xyz;
	// Normal offset sized to one shadow-map texel at this spot (texels get bigger away from the player,
	// because of the distortion). Stops "shadow acne" stripes and blotches without detaching shadows.
	vec4 sp0 = shadowProjection * (shadowModelView * vec4(playerPos, 1.0));
	float r = length(sp0.xy);
	float sdv = shadowD();
	float texelWorld = (2.0 * shadowDistance / float(shadowMapResolution)) * (sdv * r + 1.0 - sdv) * (sdv * r + 1.0 - sdv) / (1.0 - sdv);
	float nl = length(normalV);
	if (blockId == ID_PLANT) {
		// Plants: look up the shadow a bit toward the sun, past the plant's own top,
		// so a grass tuft isn't shaded by itself but still sits in tree and cliff shadows.
#if GRASS_STYLE == 1
		// Complementary: tiny offset, so the crossed grass quads shade each other. Looking away from the sun,
		// the half of the tuft behind the other quad goes dark, like in Complementary Reimagined.
		playerPos += normalize(mat3(gbufferModelViewInverse) * shadowLightPosition) * (0.03 + texelWorld * 0.6);
#else
		playerPos += normalize(mat3(gbufferModelViewInverse) * shadowLightPosition) * (0.9 + texelWorld);
#endif
	} else if (nl > 0.01) {
		vec3 nW = mat3(gbufferModelViewInverse) * (normalV / nl);
		vec3 lW = normalize(mat3(gbufferModelViewInverse) * shadowLightPosition);
		float NdotL = clamp(dot(nW, lW), 0.0, 1.0);
		float bias = (0.03 + texelWorld * 1.1) * (1.6 - 0.6 * NdotL);   // original offset: no striped shadow acne on the ground
		playerPos += nW * bias;
	} else {
		playerPos += normalize(mat3(gbufferModelViewInverse) * shadowLightPosition) * (0.05 + texelWorld);
	}
	vec4 sp = shadowProjection * (shadowModelView * vec4(playerPos, 1.0));
	shadowClip = sp.xyz;
#endif

	gl_Position = gl_ProjectionMatrix * vp;
}

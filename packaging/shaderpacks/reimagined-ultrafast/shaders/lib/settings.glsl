// ============================================================
//  Mystique - settings (shown in Shader Pack Settings)
// ============================================================

// ---------- Shadows ----------
const int shadowMapResolution = 512;   // [512 1024 2048 4096] 512 is optimal for mobile Snapdragon
const float shadowDistance = 32.0;      // [1.0 2.0 4.0 8.0 16.0 24.0 32.0 40.0 48.0 56.0 64.0 80.0 96.0 128.0]
const float sunPathRotation = -25.0;    // [-40.0 -30.0 -25.0 -20.0 -10.0 0.0 10.0 20.0 30.0]
const float shadowDistanceRenderMul = 1.0;
const bool shadowHardwareFiltering = true;
const float shadowIntervalSize = 0.0;   // no grid snapping: shadows slide smoothly instead of jumping
#define SHADOW_DISTORTION_512 0.9  // [0.85 0.88 0.9 0.92] Only used when shadow resolution is 512: higher = sharper shadows near you, blockier far away
#define SHADOW_SMOOTH 1           // [0 1 2] Shadow edge smoothing: 0 = off, 1 = 2 taps, 2 = 4 taps (smoothest, a bit slower)
#define LEAF_SOLID_SHADOWS        // Leaves cast solid block shadows (no see-through holes)
#define SHADOWS                          // Real-time shadows from the sun and moon
//#define SOFT_SHADOWS                   // Smoother shadow edges (2 filtered samples instead of 1)
//#define SKY_SHADOWS                    // Shade everything from how much sky it sees. Works on every device, almost free
//#define SHADOW_DEBUG                   // Red = in shadow, blue = outside the shadow map. For troubleshooting.

// ---------- Colors ----------
#define SATURATION 1.0      // [0.8 0.9 1.0 1.05 1.1 1.2 1.3 1.4 1.5]
#define VIBRANCE 0.06       // [0.0 0.06 0.12 0.2 0.3 0.5]
#define EXPOSURE 1.0        // [0.6 0.7 0.8 0.9 1.0 1.1 1.2 1.35 1.5]
#define CONTRAST 1.06       // [0.9 0.95 1.0 1.03 1.06 1.1 1.15 1.2]
#define SUN_BRIGHTNESS 1.0  // [0.6 0.8 1.0 1.2 1.4]
#define TORCH_BRIGHTNESS 1.0 // [0.5 0.75 1.0 1.25 1.5 2.0]
#define NIGHT_BRIGHTNESS 1.0 // [0.5 0.75 1.0 1.5 2.0 3.0]
#define VIGNETTE 0.0        // [0.0 0.1 0.18 0.25 0.35]
#define BLUE_TINT 0.0       // [0.0 0.3 0.6 1.0 1.5] Cool blue tint over the whole picture: bluer shadows and haze. 0 = off
#define GRADE_STYLE 1       // [0 1] 1 = Complementary Reimagined color grade (Lottes tonemap), 0 = Mystique's own ACES grade
#define TM_CONTRAST 1.05    // [0.85 0.9 0.95 1.0 1.05 1.1 1.15 1.2 1.3] Complementary tonemap contrast (Complementary grade only)
#define TM_WHITE_PATH 1.0   // [0.4 0.6 0.8 1.0 1.2 1.4 1.6] How fast very bright colors burn out to white (Complementary grade only)
#define TM_DARK_DESATURATION 0.25 // [0.0 0.1 0.25 0.4 0.6] Drains color from dark areas (Complementary grade only)

// ---------- CazToon Color Grading ----------
#define COLOR_GRADING_ENABLED         // CazToon cinematic color grading system
#define CG_STYLE_PRESET 7             // [0 1 2 3 4 5 6 7 8] 0=None, 1=Cinematic, 2=Vintage, 3=Cyberpunk, 4=Horror, 5=Fantasy, 6=Nordic, 7=Tropical (CazToon Default), 8=Noir
#define CG_STYLE_INTENSITY 0.30       // [0.0 0.1 0.2 0.3 0.4 0.5 0.6 0.7 0.8 0.9 1.0]
#define CG_TEMPERATURE 0.55           // [-1.0 -0.8 -0.6 -0.4 -0.2 0.0 0.2 0.4 0.55 0.7 0.85 1.0] Warm sunlight (positive) vs cold blue (negative)
#define CG_TINT 0.17                  // [-1.0 -0.8 -0.6 -0.4 -0.2 0.0 0.17 0.3 0.5 0.7 1.0] Magenta (positive) vs green (negative)
#define CG_VIBRANCE 0.15              // [-1.0 -0.75 -0.5 -0.25 0.0 0.15 0.25 0.5 0.75 1.0] Intelligent color vibrance boost
#define CG_SPLIT_TONING_ENABLED       // Tint shadows and highlights separately (CazToon style)
#define CG_SPLIT_TONING_INTENSITY 0.15 // [0.0 0.05 0.10 0.15 0.20 0.30 0.40 0.50 0.75 1.0]
#define CG_SPLIT_TONING_BALANCE 0.0   // [-1.0 -0.75 -0.5 -0.25 0.0 0.25 0.5 0.75 1.0]
#define CG_SHADOW_TINT_R 0.1          // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_SHADOW_TINT_G 0.1          // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_SHADOW_TINT_B 0.2          // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_HIGHLIGHT_TINT_R 0.2       // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_HIGHLIGHT_TINT_G 0.15      // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_HIGHLIGHT_TINT_B 0.1       // [0.0 0.05 0.1 0.15 0.2 0.25 0.3 0.4 0.5]
#define CG_EXPOSURE 0.16              // [-1.0 -0.5 -0.25 0.0 0.16 0.25 0.5 0.75 1.0]
#define CG_BRIGHTNESS -0.16           // [-0.5 -0.25 -0.16 -0.1 0.0 0.1 0.25 0.5]
#define CG_CONTRAST 0.14              // [-0.5 -0.25 -0.1 0.0 0.1 0.14 0.25 0.5]
#define CG_BLACKS -0.14               // [-0.5 -0.25 -0.14 -0.1 0.0 0.1 0.25 0.5]
#define CG_WHITES 0.08                // [-0.5 -0.25 -0.1 0.0 0.08 0.1 0.25 0.5]
#define CG_SHADOWS 0.16               // [-0.5 -0.25 -0.1 0.0 0.1 0.16 0.25 0.5]
#define CG_HIGHLIGHTS 0.27            // [-0.5 -0.25 -0.1 0.0 0.1 0.25 0.27 0.5]
#define CG_HUE_SHIFT 0.0              // [-180.0 -120.0 -60.0 0.0 60.0 120.0 180.0]
#define CG_SATURATION 0.0             // [-0.5 -0.25 0.0 0.25 0.5]

// ---------- Foliage and sun (Complementary Reimagined style) ----------
#define WAVING_FOLIAGE            // Grass, flowers, crops and leaves sway in the wind
#define WAVING_I 1.0       // [0.5 0.75 1.0 1.25 1.5 2.0] Wind strength
#define WAVING_SPEED 1.0   // [0.5 0.75 1.0 1.5 2.0] Wind speed
#define PLAYER_PLANT_INTERACTION  // Grass and flowers bend away as you walk through them (CazToon style)
#define PLANT_INTERACTION_RADIUS 2.2 // [1.2 1.5 1.8 2.0 2.2 2.5 3.0] Distance in blocks where plants start bending
#define PLANT_INTERACTION_INTENSITY 1.0 // [0.5 0.75 1.0 1.25 1.5 2.0] How strongly plants bend away
#define FOLIAGE_SSS               // Sunlight glowing through grass and leaves when you look toward the sun
#define GRASS_STYLE 1             // [0 1] 1 = Complementary Reimagined grass (even light, soft shadows, narrow sun glow, natural color), 0 = old Mystique grass
#define GRASS_VARIATION 0.0       // [0.0 0.5 1.0 1.5 2.0] Patchy, richer grass color (0 = flat vanilla)
#define SUN_GLOW                  // Soft warm glow around the sun
#define SUN_GLOW_STRENGTH 1.0     // [0.5 0.75 1.0 1.25 1.5 2.0]

// ---------- Water ----------
#define MIRROR_REFLECTIONS             // Cheap mirror-image reflection of terrain and trees (1 depth + 1 color read per water pixel)
//#define REFLECTIONS                    // Full ray-marched reflections (heavy on phones). Off = mirror image + sky
#define SSR_STEPS 4            // [3 4 5 6 8] Steps for ray-marched reflections (only used when Ray-Marched Reflections is on)
#define REFLECTION_STRENGTH 0.75 // [0.5 0.75 1.0]
#define SUN_REFLECTION             // Bright sun / moon glint on water
#define UNDERWATER_DISTORTION      // Subtle refraction wave wobble when submerged (Complementary Reimagined)
#define UNDERWATER_CAUSTICS        // Shimmering sunlight caustic ripples on underwater floors
#define UNDERWATER_FOG_DENSITY 1.0 // [0.5 0.75 1.0 1.5 2.0]
#define WATER_OPACITY 0.25      // [0.15 0.2 0.25 0.32 0.4 0.5 0.65 0.8] How solid very shallow water is. Lower = clearer shallows
#define WATER_DEPTH_FADE 1.5    // [0.5 0.75 1.0 1.5 2.0] How quickly water turns deep blue-teal with depth
#define WATER_COLOR_MIX 0.0     // [0.0 0.2 0.35 0.5 0.7 1.0] 0 = Complementary-style biome color, 1 = the pack's own teal-blue
#define WATER_TEXTURE_DETAIL 1.0 // [0.0 0.5 1.0 1.5] How much of the vanilla water texture shows through
#define WATER_WAVES 1.0         // [0.0 0.5 1.0 1.5 2.0] Ripple strength on the surface. 0 = flat
#define WATER_STYLE 1           // [1 2 3 4] 1 = Reimagined (flat), 2 = Reimagined + Waves (rippled, strong reflections), 3 = Unbound, 4 = Classic blocky
#define WATER_BRIGHTNESS 1.0    // [0.3 0.45 0.6 0.8 1.0] How light the Reimagined water surface is. Lower = deeper blue, less milky
#define WATER_TINT 1.0          // [0.0 0.5 1.0 1.5] Blue tint on everything seen through the water (Reimagined style)
#define WATER_FOAM 1.0          // [0.0 0.5 1.0 1.4] White foam where water meets the shore (Reimagined style)
#if WATER_STYLE <= 2
	#define WATER_REIMAGINED      // Reimagined water color, opacity and foam
#elif WATER_STYLE == 3
	#define WATER_REALISTIC       // Unbound water color
#endif
#if WATER_STYLE == 2 || WATER_STYLE == 3
	#define WATER_RIPPLES         // Ripple normal map on the surface (reflections get the streaky Complementary look)
#endif
#define WATER_SPEED 1.1         // [0.0 0.5 0.8 1.1 1.5 2.0] How fast ripples drift
//#define WATER_PIXELATED       // Only for the older look: ripples snap to the block pixel grid

// ---------- Bloom and fog ----------
#define BLOOM                            // Glow around bright light
#define BLOOM_STRENGTH 1.0  // [0.0 0.25 0.5 0.75 1.0 1.25 1.5 2.0]
#define ATMOSPHERIC_FOG 1.0 // [0.0 0.5 1.0 1.5 2.0]
#define GOLDEN_HOUR 1.0       // [0.0 0.5 1.0 1.5 2.0] Warm golden tint on the world at sunrise (and a little at sunset). 0 = off
#define MIDNIGHT_MIST 1.0      // [0.0 0.5 1.0 1.5 2.0] Low mist at night and sunrise. 0 = off
#define MIST_THICKNESS 64      // [48 56 60 63 64 68 72 80 96] Height the mist settles at (sea level is 63)


// ------------------------------------------------------------ clouds
#define BLOCKY_CLOUDS
#define CLOUD_STYLE 1          // [0 1] 1 = Complementary Reimagined clouds (soft rounded slab), 0 = Mystique box clouds
#if CLOUD_STYLE == 1
	#define CLOUDS_REIMAGINED
#endif
#define CLOUD_HEIGHT 192       // [128 160 192 224 256 320]
#define CLOUD_THICKNESS 6.0    // [4.0 6.0 8.0 10.0]
#define CLOUD_SIZE 12.0        // [8.0 12.0 16.0 24.0]
#define CLOUD_COVERAGE 0.48    // [0.30 0.38 0.44 0.48 0.52 0.58]
#define CLOUD_OPACITY 0.88     // [0.6 0.75 0.88 1.0]
#define CLOUD_SPEED 1.0        // [0.0 0.5 1.0 2.0 4.0]
#define CLOUD_STEPS 8         // [8 10 12 16 24 32 48 64]
#define CLOUD_RENDER 350.0      // [250.0 350.0 400.0 500.0 600.0 750.0 900.0 1000.0 1200.0]

// ------------------------------------------------------------ extras (off by default, on in High)
//#define DYNAMIC_LIGHTS        // Torches and glowing items light up the area around you while held
#define DYNAMIC_LIGHT_RANGE 1.0 // [0.5 0.75 1.0 1.25 1.5]
//#define GODRAYS               // Sun rays through trees and clouds
//#define UNDERWATER_RAYS           // Sun light shafts when you are underwater (works without Godrays)
#define UNDERWATER_RAYS_STRENGTH 1.0 // [0.5 0.75 1.0 1.5 2.0]
#define GODRAYS_STRENGTH 1.0    // [0.5 0.75 1.0 1.5 2.0]
#define EYE_ADAPTATION             // Flash of white when you go from a dark cave into bright sun, then your eyes adjust
#define EYE_ADAPT_STRENGTH 1.0     // [0.5 0.75 1.0 1.5 2.0]
//#define TOON_MODE                 // Cartoon look (CazToon style): coloured edge outlines, soft tinted shadows, grass patches, leaf shading
#define TOON_OUTLINES 2            // [0 1 2] Edge outlines: 0 = off, 1 = Standard (thin, both sides of an edge), 2 = Dungeons (CazToon look, on the near object)
#define TOON_OUTLINE_PLAYER 1      // [0 1] Outlines on player character, first-person hands and held items: 0 = off, 1 = on
#define TOON_OUTLINE_BRIGHTNESS 0.20 // [-0.80 -0.60 -0.40 -0.20 0.0 0.10 0.20 0.30 0.50 0.80] Outline lightness. Positive = light coloured outline (CazToon), negative = dark ink outline
#define TOON_OUTLINE_SATURATION 1.35 // [0.0 0.5 1.0 1.35 1.75 2.5 3.0] How vivid the outline colour is. 1 = same as the scene
#define TOON_OUTLINE_SIZE 2        // [1 2 3 4] Outline thickness in pixels
#define TOON_SHADOW_STRENGTH 0.75  // [0.5 0.6 0.7 0.75 0.8 0.9 1.0] How dark shadows get. 1 = normal pack shadows, lower = softer, lighter shadows
#define TOON_SHADOW_TINT 0.5       // [0.0 0.25 0.5 0.75 1.0] Cool blue shadows + warm sunlit areas. 0 = off
#define TOON_GRASS_PATCHES 1.0     // [0.0 0.5 1.0 1.5] Blocky light/dark patches on grass that drift slowly (CazToon). 0 = off
#define TOON_LEAF_SHADING 1.0      // [0.0 0.5 1.0] Leaves: light yellow-green on top, darker blue-green deeper in the tree. 0 = off
#define TOON_FLAT_ENTITIES 0.7     // [0.0 0.35 0.7 1.0] Flatter lighting on mobs, items and players. 0 = normal
#define TOON_SHARPEN 0.15          // [0.0 0.1 0.15 0.25 0.4 0.6] Picture sharpening (CazToon uses a light touch). 0 = off
#define TOON_BANDS 0               // [0 3 4 5 6] Optional hard brightness steps over the whole picture. 0 = off (CazToon has none)
#define TOON_SATURATION 1.15       // [1.0 1.1 1.15 1.25 1.4] Color richness in toon mode
//#define MOTION_BLUR                // Camera motion blur: makes 30 fps feel smoother
#define MOTION_BLUR_STRENGTH 1.0   // [0.5 0.75 1.0 1.5 2.0]
#define MOTION_BLUR_SAMPLES 6      // [4 6 8 12] More samples = smoother blur, a bit slower
//#define DEPTH_OF_FIELD        // Blurs things away from what you're looking at
#define DOF_STRENGTH 1.0        // [0.5 0.75 1.0 1.5 2.0]
const float centerDepthHalflife = 1.2;   // how fast focus follows your view

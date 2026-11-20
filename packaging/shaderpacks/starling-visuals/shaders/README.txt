MYSTIQUE(TM)
A bright, lightweight shader pack for Iris
Minecraft Java Edition 1.21.11 and 26.3

Copyright (c) 2026 Mystique. All rights reserved.
"Mystique" is a trademark (TM) of the Mystique author.


FEATURES
- Real-time soft shadows
- Blocky 3D clouds
- Water reflections, underwater fog and bloom
- Rain fog, night mist, Nether and End fog
- Extras: dynamic lights, sun rays and depth of field
  (off by default, on in High, or switch them on in Settings > Extras)
- Presets: Potato, Lite, Medium, High


INSTALL
1. Install Iris (Sodium recommended).
2. Drop Mystique-UltraFast.zip into your shaderpacks folder. Don't unzip it.
3. Options > Video Settings > Shader Packs > Mystique.
4. Pick a preset. Lite is made for phones and low-end PCs.


TERMS OF USE
You can:
- Play with it and make videos or screenshots.
- Change settings and edit it for your own private use.

You can't (without permission):
- Re-upload or redistribute it, edited or not.
- Sell it or put it behind ad-links.
- Claim it as your own.
- Use the Mystique name for another project.

Mystique is not an official Minecraft product and is not approved by
or associated with Mojang or Microsoft.

Provided "as is", without warranty of any kind.


PRIVATE EDIT: Snapdragon 7s Gen 2 build (Adreno 710)
Changes from Mystique-UltraFast:
- Sun, sky and ambient colors are worked out once per vertex instead of per pixel.
- Distance fog and night mist are worked out per vertex (interpolated), not per pixel.
- Bloom uses 6 texture reads instead of 20.
- colortex0 is R11F_G11F_B10F, the water buffer is RG8, and the finished picture goes
  through an 8-bit buffer: much less memory traffic on a tile-based mobile GPU.
- pow() removed from the hot paths (sRGB conversion, tonemap gamma, torch light, sun glint).
- Defaults now match the Lite profile: shadow distance 40, soft shadows off, 16 cloud steps.
  Player and block-entity shadows are off.
- Cloud Style option (Settings > Clouds): Reimagined (default) = Complementary Reimagined clouds: a thin slab of rounded
  cloud noise (from cloud-water.png, blue channel), shaded by two noise samples toward the sun, softened into the sky
  with distance. Uses Coverage, Height, Speed, Steps and Render Distance; Size and Thickness only affect the box clouds.
- Color Grade option (Settings > Colors): Complementary (default) uses Complementary Reimagined's modified Lottes
  tonemap (dark-color lift, path to white, dark desaturation) plus its saturation step. Mystique = the original ACES grade.
  Exposure, Saturation and Vignette still apply; Contrast and Vibrance sliders only affect the Mystique grade.
- Water Style option (Shader Pack Settings > Water), same three styles as Complementary:
  2 Reimagined + Waves (default): style 1 plus the ripple normal map and strong mirror reflections.
  1 Reimagined: vanilla water texture squared x biome color, flat surface, block-grid color noise,
    shore foam, texture-based opacity, slightly softer reflections.
  3 Unbound: the smooth rippled water described below. 4 Classic: the older blocky look.
- Style 3 (Unbound) is the water rebuilt after Complementary Reimagined r5.9.3 (Realistic Water):
  ripple normal map read at four sizes, depth-based opacity, Complementary's Fresnel curve,
  and a cheap mirror-image reflection of terrain and trees (no ray marching).
  Pick Classic for the older blocky look. Ray-marched reflections are optional.
  lib/textures/cloud-water.png is Complementary's ripple texture (by EminGT), used for private use.
  Tune it in Shader Pack Settings > Water.
This edited copy is for your own private use only (see Terms of Use above).

v1.6 (grass + sun pass):
- Waving foliage (Complementary DoWave_Foliage / Leaves): grass, flowers, crops and leaves sway, roots stay planted. Off in caves. Settings: Waving Foliage / Wind Strength / Wind Speed.
- Foliage sun-through glow (Complementary subsurface highlight): grass and leaves light up warm when the sun is behind them.
- Grass variation: patchy lush/dry color, fine speckle, richer green on plants and grass-block tops. Grass Variation slider (0 = vanilla).
- Complementary Reimagined sun disc (hot core, warm falloff, sinks under the horizon) + cheap screen-space sun glow. Sun Glow / Sun Glow Strength options.
- Soft shadows use 2 filtered taps (cheaper than the old 4); off by default for FPS.
- v1.6e: grass glow softened, warm sunset clouds, thinner sunset haze (ALU only, no extra texture reads).
- v1.6d: foliage glow is directional at low sun: only grass on the sun side lights up, rest stays dim.
- v1.6c: sunset pass: whiter sun core, wider glow, warm horizon haze.
- v1.6b: grass tint toned down (less yellow), soft shadows default off.

v1.6f (CR grass): Grass Style option (Settings > Foliage). 1 = Complementary Reimagined grass: even lighting, softer shadows, narrow natural sun glow, vanilla grass color (Grass Variation now defaults to 0). 0 = old look.
v1.6g: Grass Style 1 now darkens the base of grass tufts, adds a bit of color, and uses a smaller shadow offset so shadows touch the grass base (less floating look).
v1.6h: Grass Style 1: grass tufts shade themselves like Complementary (side away from the sun goes dark when you look away from it).
v1.6i: Shadow Smoothing option (4-tap smooth shadows, works at 512) and Solid Leaf Shadows.
v1.6j: removed dark base on grass tufts.
v1.6k: Underwater Rays option (on by default).
v1.6l: lighter water reflections (skip when faint, fewer SSR steps).
v1.6m: underwater rays off by default; restored original shadow offset on ground (fixes striped pattern on grass blocks).
v1.6n: sunrise: much lighter dawn mist, golden tinted (matches sunset gold).

v1.6o: cheaper Ray-Marched Reflections: view-space depth compare (no mat4 + reconstruction per step), jittered start (less banding at few steps), skipped when the reflection is faint. New option Reflection Steps (Settings > Water). High profile uses 6.
v1.6p: Ray-Marched Reflections reach about twice as far, and fall back to the mirror reflection where the ray misses, so distant trees and hills are reflected too.
v1.6q: thicker underwater fog that starts close (visibility ~28 blocks instead of ~44). Tune with Underwater Fog in Settings > Water.
v1.6r: Golden Hour Tint (Settings): warm golden light on surfaces at sunrise, lighter at sunset.
v1.6s: stronger Golden Hour Tint: warmer light + ambient, more sun, golden distance haze.
v1.6t: no reflections on moving (flowing/sloped) water; new defaults: ray-marched reflections on, 8 steps, strength 0.75, shadow distance 48, smoothing 1, soft shadows on, clouds 16 steps / 500 range.
v1.6u: Motion Blur (Settings > Extras): camera motion blur, on by default.
v1.6v: motion blur never blurs the hand / held item and doesn't smear it into the world.
v1.6w: fixed flickering shadow on the player's back (entities facing away from the sun get no direct light).
v1.6x: Eye Adaptation (Extras): white flash when leaving a cave into sunlight that fades over a few seconds.
v1.6y: entities facing away from the sun no longer use the shadow map (fixes back flicker when walking toward the sun in third person).
v1.6z: better shadows at 512: always 4-tap smoothing and stronger focus near the player (Shadow Focus at 512).
v1.8: menu reorganized: Quick Settings first, then Shadows, Lighting, Colors & Tone, Water (Look / Reflections / Underwater), Clouds, Grass & Leaves, Camera Effects, Atmosphere.
v1.9: Toon Mode (Toon Mode screen, off by default): shading bands, depth outlines, richer color. Original implementation.
v1.9a: removed float comparisons from #if (strict GPU compilers reject them) - fixes pack not loading on some phones.

v1.10: Toon Mode reworked to look like CazToon (all options in Toon Mode screen):
- Outlines: CazToon "Dungeons" depth-edge outlines (line on the nearer object, light and saturated instead of black), plus a Standard style. Thickness, brightness and saturation are options. Green foliage, sky and the held item are not outlined.
- Shadows: softer (Shadow Strength) with cool blue shadows and warm sunlit areas (Cool Shadows).
- Grass Patches: blocky 16x16 light/dark grass patches that drift slowly, cooler in the dark ones.
- Leaf Shading: light yellow-green outside, darker blue-green deeper inside the tree (from sky light, no voxel pass).
- Flat Mob Lighting, light Sharpen. The old whole-picture brightness steps are now optional (Hard Brightness Steps, off by default).
- Toon look ported from CazToon 1.2.7; private use only, like the rest of this pack.

# Fix: 3D Skins Native Mod Rendering Flat Outer Layers on Android

## 1. Problem Statement
In `native-mods/skins_3d`, the outer skin layers (hat, jacket, sleeves, pants) were rendering flat instead of 3D voxels on Android devices.

## 2. Root Cause Analysis
1. **No Loose PNGs on Android:** `bundled()` in `glue.rs` attempted to open individual PNGs from `options.resources`. On Android, textures are bundled inside binary atlases rather than as individual loose PNG files. All default skins (`entity/player/wide/steve`, etc.) failed to load.
2. **Permanent Blacklist Poisoning:** When `bundled()` failed, it inserted the skin location into `MISSING`. Once added, `missing.contains()` caused the mod to permanently skip the skin on every subsequent frame.
3. **Unresolved Own / Offline Skin:** When players used a custom skin from Skin Customization (`horizon/own_skin`), `glue.rs` only queried `skin_downloads` and disk, never reading `game.own_skin.image` where the 64x64 RGBA pixels actually reside in memory.
4. **CEM Custom Body Override:** In `crates/horizon-client/src/mobs/render.rs`, `render_visible` checked `custom_body` first and only fell back to `m.skin_3d` if `custom_body` was `None`. Any resource pack or CEM custom model completely overrode `skin_3d`.
5. **Static Cache Stagnation:** In `cache.rs`, `offer()` returned immediately if `cache.skins.contains_key(location)` was true, even if the skin texture pixels had changed.

## 3. Changes Implemented
1. **Atlas Pixel Extraction (`glue.rs: from_atlas`):**
   - Implemented `from_atlas(game, location)` which locates the sprite in `game.gui.atlas.sprites`.
   - Reads `game.gui.atlas.pixels()` at `(u0, v0)` with texel width and height, copying 64x64 RGBA texels row-by-row.
2. **In-Memory Own Skin & Offline Skin Resolution (`glue.rs: ensure_skin`):**
   - Directly checks `game.own_skin.image` for `horizon/own_skin` and `horizon/offline_skin`.
   - Checks `horizon_client::skins::offline_skin()`.
   - Excluded dynamic locations (`skins/*`, `horizon/own_skin`, `horizon/offline_skin`) from the permanent `MISSING` set.
3. **Player CEM Precedence Fix (`crates/horizon-client/src/mobs/render.rs`):**
   - Prioritized `m.skin_3d` so that when voxel geometry exists for a player, it is rendered and not masked by `custom_body`.
4. **Dynamic Cache Refresh (`cache.rs: offer`):**
   - If `location` already exists in `cache.skins`, `offer()` compares RGBA bytes. If different, it updates the pixels and clears cached models so they are re-voxelized.
5. **Logcat Visibility & Tracing (`glue.rs`):**
   - Added informative `log::info!` lines recording every resolution step (`from_atlas`, `own_skin`, `skin_downloads`, `offline_skin`, `bundled`) and initialization status.
6. **Revision Compatibility:**
   - Updated `horizon_revision` in `native-mods/skins_3d/mixins/skins_3d/horizon-mixin.toml`.

## 4. Verification
- All mixin hooks (`model`, `arm`, `frame`, `list`, `settings`, `click`) verified against Starling/Horizon signatures.
- Remote push and GitHub Actions Android CI workflow execution.

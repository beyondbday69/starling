# Starling Project Status & Complete Context

**Repository:** `beyondbday69/starling`  
**Current Active Branch:** `feat/water-physics-v2`  
**Target Platform:** Android (ARM64 `arm64-v8a`)  
**Latest Commit:** `5a2db4a` (`fix(water_physics): ensure f64 types for splash particle velocity vectors`)  
**Latest Successful CI Run:** `#44543723733` (Artifact: `14540746729`, size: 35.2 MB)  
**Last Updated:** November 2026  

---

## 1. Project Architecture & System Overview

Starling is an optimized, high-performance Minecraft client built in Rust (based on the Horizon engine) tailored specifically for Android. It features custom graphics pipelines, direct all-in-one native mod compilation into the core Android shared library (`libhorizon_android.so`), seamless touch gestures, virtual trackpad aiming, and cinematic in-world crafting interactions.

```mermaid
graph TD
    A["Starling Client (Android / Rust)"] --> B["Horizon Graphics Pipeline"]
    A --> C["Modern Horizon UI & Touch Engine"]
    A --> D["Direct Native Mod Integrations"]

    B --> B1["AMD FSR 1.0 (EASU + RCAS WGSL)"]
    B --> B2["True Integer Scaling (fs_integer)"]
    B --> B3["Shader Packs & Post-Processing"]

    C --> C1["Virtual Trackpad Relative Cursor (+)"]
    C --> C2["1-Finger Gestures (Tap LMB, Dbl-Tap RMB, Drag, Long-Press)"]
    C --> C3["Sleek HUD Overlay ([ X ] Exit, [ ^ ] Inspect)"]

    D --> D1["Cinematic Interactions (Gliding Camera, Resting Hands)"]
    D --> D2["Hold My Items 5.2.0 (Spring Inertia & Arms)"]
    D --> D3["3D Skins & HD Player Models (horizon-3d-skins)"]
    D --> D4["Gyroscope Aiming Controls (gyro)"]
    D --> D5["Living Motion, Zoom, ElytraTrails"]
```

---

## 2. Full Implementation History & Phases

### Phase 1: AMD FSR 1.0 & CAS Spatial Upscaling
* **WGSL Shader Implementation:**
  * Implemented 12-tap Lanczos-approximated **EASU** in `crates/horizon-client-render/src/gfx/fsr.wgsl`.
  * Implemented contrast-adaptive **RCAS**.
  * Resolved Vulkan/Android NDC inversion by binding screen-space `@builtin(position).xy` directly to texel fetch coordinates via `textureLoad`.
* **Pipeline Integration:**
  * Integrated `FsrPipeline` into `crates/horizon-client-render/src/gfx/fsr.rs` and `pack_post.rs`.
  * Added resolution scaling presets (**Performance 50%**, **Balanced 67%**, **Quality 75%**, **Off 100%**) and sharpness controls into Horizon's menus.

### Phase 2: Integer Scaling & Modern Shaders UI
* **Modernized Shader Pack Menus:** Redesigned `Page::ShaderPacks`, `Page::ShaderOptions`, and `Page::NativePackOptions` in `crates/horizon-client-render/src/options_ui.rs`.
* **True Integer Scaling (`ShaderUpscaling::Integer`):** Added integer multiplier mode `min(dst_w / src_w, dst_h / src_h)` for crisp pixel-art rendering. Routed integer scaling through the upright FSR coordinate space to prevent 180° screen flipping.

### Phase 3: Runtime `.so` Mod Loader Integration
* Dynamic `.so` loader on Android utilizing anonymous memory file descriptors (`memfd_create` + `android_dlopen_ext`) to bypass SELinux restrictions without client rebuilds.

### Phase 4: Camera ABI v5, Crosshair & Shoulder Surfing Mod
* Added `CameraOffset` struct and `camera_offset` callback in `horizon-mod-abi` (v5).
* Wired into third-person camera calculation with block collision detection (`clip_world`).
* Enabled aiming crosshair (`+`) and attack indicators in third person.

### Phase 5: Dynamic Eyes, Smooth Blinking & Look Tracking
* Added `PlayerEyes` struct in `horizon-mod-abi` (v6) with real-time GPU atlas patching (`update_mod_player_eyes`).
* Continuous sub-pixel blinking ($\sin^2(\pi \cdot \tau)$), smooth eye tracking with exponential smoothing filter, downward eyelid droop, and continuous blushing.

### Phase 6: Resizable Eyes & Reactive Expressions (`libeyemod.so`)
* Eye scales: Normal (2x2), Tall/Big (2x3), Wide (3x2), Huge Anime (3x3), and Auto Dynamic.
* Reactive Shock Reaction: Triggers on damage taken or fast fall; eyes pop open, eyebrows arch, pupils jitter with micro-tremor, anime sweat drop on temple, open 'o' mouth.

### Phase 7: Hold My Items Core Integration (Direct APK Compilation)
* Direct port of the HMI 5.2.0 engine into `crates/horizon-client/src/game/holdmyitems/`:
  * 12 easing curves, $4\times4$ matrix transforms, spring-damper inertia physics, alternating sword combo slashes, per-item swing speed tables, and weapon inspection (`C` key 360° spin).
  * Direct hook injections into `totem.rs`, `hand.rs`, and `native_hooks.rs`.
  * All bundled assets embedded via `native_data!`, completely eliminating dynamic dlopen issues on Android.

### Phase 8: Core Mods Suite (Zoom, ElytraTrails, Living Motion)
* **Zoom (`zoom.rs`):** Built-in smooth camera zoom with sensitivity scaling.
* **ElytraTrails (`elytratrails/`):** Dynamic fading wingtip contrails with LAN color synchronization.
* **Living Motion (`livingmotion/`):** Dynamic procedural animation rig (swings, leaning, jumping, banking, swimming, climbing).

### Phase 9: Horizon 3D Skins & Gyro Native Mods Integration (`03e97f9`, `ff69d2f`)
* **3D Skins (`umbx-dot/horizon-3d-skins`):** Integrated layered 3D skin geometry, extruded clothing/armor layers, and HD skin rendering. Resolved `studio` field visibility and mutable borrow conflicts.
* **Gyro Aiming (`CGRLabs/gyro`):** Integrated device gyroscope motion sensing for fine-tuned tilt aiming and camera orientation on Android.

### Phase 10: Virtual Trackpad Cursor & 1-Finger Crafting (`25802d5`, `218f825`, `b9cd04b`)
* **Virtual Trackpad Cursor:**
  * Replaced jumpy direct-touch input with a smooth relative trackpad cursor. Swiping anywhere on the screen moves the in-world `+` crosshair cursor smoothly across workstation slots.
* **1-Finger Gesture Engine (`touch_helper.rs`):**
  * **Single Tap:** Left-Click (`inventory_press(false)`) — picks up or places a stack of items.
  * **Double Tap:** Right-Click (`inventory_press(true)`) — places 1 single item or splits a stack in half.
  * **Double-Tap & Drag:** Right-Click Drag — distributes 1 item across each slot dragged over (row crafting).
  * **Long Press:** Quick-Move (Shift + Left-Click) — transfers stack immediately to/from inventory.
* **HUD Overlay:**
  * Sleek `[ X ]` exit button in the top right to close workstation interactions cleanly.
  * Sleek `[ ^ ]` inspect/toggle button.
  * Real-time 3D item drag preview floating with the cursor.

### Phase 11: Workstation Crash & Deadlock Fix (`34a0af2`, Run `#44355584017`)
* **The Problem:** Game froze or crashed instantly upon opening/interacting with a crafting table or workstation.
* **Root Causes Identified & Fixed:**
  1. **Recursive Mutex Deadlock:** `ci::vcursor` locked the global `STATE` mutex and invoked `default_aim_point(w, h)`, which internally called `table()`, which attempted to re-lock `STATE` on the same thread. Rust's `std::sync::Mutex` is not re-entrant, causing an immediate deadlock.
     * *Fix:* Created `aim_point_from_state(&State, w, h)` to project aim points directly from the already-borrowed `State` without re-locking.
  2. **Render Pass Mutation:** `draw_inventory` GPU render pass was invoking `inventory_press`, mutating game state while wgpu rendering was in flight.
     * *Fix:* Removed all inventory mutation from rendering code; moved pending tap execution cleanly into the engine frame hook (`Game::frame`).
  3. **Touch Safety Guard:** Added state validation in `touch_helper.rs`:
     `if !ci::controls() || game.screen != Screen::Inventory || ci::walking(Instant::now()) { return ControlFlow::Continue(()); }`
* **Verification:** GitHub Actions Run `#44355584017` completed with status **SUCCESS**.

---

## 3. Implemented Feature: Option 1 - "Mouse Tweaks" Sweep Drag to Collect Matching Items

### The User Request:
> *"Now can I grab all items at one's like sab scattered hai and muje sab ek hand me lena hai same item ? Just answer that"*  
> *"Toh right click kaise rakhenge just reply"*

### Implementation Details:
* **Feature:** 'Mouse Tweaks' style sweep drag collection (`Input::PickupAll`).
* **Behavior:** When the player is holding an item stack (`game.carried.is_some()`) and sweeps the virtual trackpad cursor across the inventory without active RMB crafting drag:
  * Detects hovered slot via `inventory_target()`.
  * If the slot contains matching items (`target.item == held.item`) and the cursor stack is not full (`held.count < 64`):
    * Triggers `game.menu_click(Some(slot), 0, Input::PickupAll)`.
    * Plays click sound `game.audio.play_button_click()`.
    * Marks container changed `game.container_changed(key)`.
  * Tracks `last_swept_slot: Option<usize>` in `TouchState` to prevent repeated rapid clicks on the same cell during cursor movement.
  * Preserves existing Double-Tap & Drag for RMB crafting and `[ X ]` / `[ ^ ]` HUD buttons.

---

## 3.1. 3D Skins Native Mod Flat Outer Layers Fix
* **Problem:** Outer skin layers (hat, jacket, sleeves, pants) rendered flat on Android devices.
* **Fixes Applied:**
  * Added `from_atlas()` in `glue.rs` to extract default player skin pixels directly from `game.gui.atlas.pixels()`.
  * Resolved `horizon/own_skin` and `horizon/offline_skin` directly from `game.own_skin.image` and `horizon_client::skins::offline_skin()`.
  * Prevented dynamic skin locations from being blacklisted in `MISSING`.
  * Fixed `render_visible()` in `crates/horizon-client/src/mobs/render.rs` so `m.skin_3d` is not overridden by CEM custom body models.
  * Added dynamic model cache invalidation in `cache.rs` on RGBA change.
  * Added logcat tracing for resolution steps and mod initialization.

---

## 3.2. Water Physics Native Mod v1 (`water_physics`)
* **Goal:** Realistic fluid wakes and surface waves on water.
* **Features:**
  * 2D heightfield leapfrog PDE wave simulation with dispersion term and boundary damping.
  * Boat Kelvin wakes (19.47° envelope), bow spray foam trails, stern depression, and oar ripples.
  * Player walking/wading ring ripples, swimming V wake, and fall splashes scaling with impact velocity.
  * Dynamic active tile management with energy-based sleeping and configurable resolution for mobile.
  * Translucent mesh overlay with specular highlights and fast-fading foam.
  * Toggleable and configurable in Options > Mods (default on).

---

## 3.3. Water Physics Native Mod v2 (Real 3D Ripples & High-Performance Mobile Optimization)
* **Diagnosis of v1:** Flat quads at +0.003 lacked 3D depth; 12m tiles at 24x24 were coarse (2 cells/block); per-frame full rebuild, heap allocations, and CPU specular loops caused mobile lag.
* **v2 Features & Architectural Improvements:**
  * **Real 3D Displaced Mesh:** Vertices displaced vertically ($Y = y_{water} + 0.002 + W \cdot (\text{base\_lift} + h \cdot \text{ripple\_scale})$) with 0.04 - 0.12m visible amplitude.
  * **Vanilla Flat Water Pitfall Solved:** Baseline elevated by +0.05m so troughs ($h < 0$) remain above vanilla water and are never clipped by the depth buffer. Fades to 0 at edges and shoreline.
  * **Smooth Small High-Res Tiles:** 4.5m tiles at 32x32 cells (~7.1 cells/block) with Gaussian & Mexican-hat kernels for clean, smooth concentric ripple rings.
  * **Zero-Allocation 30Hz Fixed Engine:** Preallocated `h_next` and mesh scratch buffers; simulation and mesh generation run strictly at 30Hz; frame draw does only a fast camera translation.
  * **Mobile Optimizations:** Single-dot diffuse lighting, instant early-out when player is away from water, mob scanning capped to 12m and 2 mobs, fast idle sleeping.
  * **Logcat Profiling:** Logs `sim_ms`, `mesh_ms`, `active_tiles`, and `verts` every second.

---

## 4. Key Artifacts, Releases & Action Runs

| Run ID | Commit | Description | Artifact ID / Download | Status |
|---|---|---|---|---|
| **#44550331982** | `1d6f549` | **Water Physics v3.1 (White Strip Artifact Fix & Continuous Zero-Pop Harmonic Envelope) + Full Native Mods:** Removed center foam strip, node continuity distance/direction guards, independent player/boat wake channels, smooth cubic decay envelope (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | Building... | **Building** |
| **#44543723733** | `5a2db4a` | **Water Physics v3.0 (Zero-Grid Fluid Engine, 24-gon Circular 3D Rings, Kelvin V-Wake Ribbons, Particle Droplets) + Full Native Mods:** Smooth circular 3D waves, true 19.47° V-wake ribbons, ParticleType::Splash droplet bursts, live framerate integration (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | [Artifact 14540746729](https://github.com/beyondbday69/starling/actions/runs/44543723733/artifacts/14540746729) | **PASSED (Green)** |
| **#44535215513** | `f60300b` | **Water Physics v2.2 (Dynamic V-Wake Hydrodynamics, True 3D Crests, Continuous Motion Tracking) + Full Native Mods:** 4-point physical V-wake, alternating footstep ripples, boat Kelvin wake, 8m tiles, 3s idle timeout, no plateauing (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | [Artifact 14536726864](https://github.com/beyondbday69/starling/actions/runs/44535215513/artifacts/14536726864) | **PASSED (Green)** |
| **#44525337249** | `0a6af8c` | **Water Physics v2.1 (Glass-like Transparent Overlay) + Full Native Mods:** Transparent overlay without body tint, soft crest/trough shading, foam max alpha 0.45 for Reimagined shader compatibility + 3D Skins + Gyro + Cinematic Interactions (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | [Artifact 14531455455](https://github.com/beyondbday69/starling/actions/runs/44525337249/artifacts/14531455455) | **PASSED (Green)** |
| **#44521237888** | `7b40393` | **Water Physics v2 with Full Native Mods Compiled In:** Real 3D Ripples + Baseline Lift + Small High-Res Tiles + 30Hz Zero-Alloc Engine + Cinematic Interactions + 3D Skins + Gyro (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | [Artifact 14527869199](https://github.com/beyondbday69/starling/actions/runs/44521237888/artifacts/14527869199) | **PASSED (Green)** |
| **#44502832147** | `5199ea1` | **Water Physics v2 (Plain APK build without native_mods input):** Real 3D Displaced Mesh + Baseline Lift + Small High-Res Tiles + 30Hz Zero-Alloc Engine | [Artifact 14518980841](https://github.com/beyondbday69/starling/actions/runs/44502832147/artifacts/14518980841) | **PASSED (Green)** |
| **#44498811898** | `1f03664` | **Water Physics Native Mod v1 (Plain APK build):** 2D Heightfield Leapfrog Wave Simulation + Kelvin Wake + Splash + Specular Mesh Overlay | [Artifact 14516308815](https://github.com/beyondbday69/starling/actions/runs/44498811898/artifacts/14516308815) | **PASSED (Green)** |
| **#44464049499** | `dcf4245` | **3D Skins Real Voxels Fix:** Atlas Extraction + In-Memory Own Skin + CEM Override Prevention + Screen-Split Controls | [Artifact 14501069325](https://github.com/beyondbday69/starling/actions/runs/44464049499/artifacts/14501069325) | **PASSED (Green)** |
| **#44408523280** | `0faef7a` | Screen-Split LMB/RMB Controls + Action Text Indicator + Cinematic Interactions + 3D Skins + Gyro | [Artifact 14475234256](https://github.com/beyondbday69/starling/actions/runs/44408523280/artifacts/14475234256) | PASSED |
| **#44395053407** | `e35ea8e` | Sweep Drag + Cinematic Interactions + 3D Skins + Gyro | [Artifact 14472020982](https://github.com/beyondbday69/starling/actions/runs/44395053407/artifacts/14472020982) | PASSED |
| **#44389836599** | `e35ea8e` | Option 1 Sweep Drag item collection (plain build) | [Artifact 14469510477](https://github.com/beyondbday69/starling/actions/runs/44389836599/artifacts/14469510477) | PASSED |
| **#44355584017** | `34a0af2` | Fixed deadlock crash, safe frame tap, trackpad cursor | [Artifact 14452912493](https://github.com/beyondbday69/starling/actions/runs/44355584017/artifacts/14452912493) | PASSED |
| **#44339845300** | `25802d5` | Added virtual trackpad cursor and 1-finger gestures | [Artifact 14449830847](https://github.com/beyondbday69/starling/actions/runs/44339845300) | PASSED |
| **#44175839634** | `df94b42` | Hold My Items + Zoom + ElytraTrails + Living Motion All-In-One | [Artifact 14371345303](https://github.com/beyondbday69/starling/actions/runs/44175839634) | PASSED |
| **Release** | `58c43de` | `libeyemod.so` (Eye tracking, blink, shock reaction) | [Release `mod-loader-eyemod`](https://github.com/beyondbday69/starling/releases/tag/mod-loader-eyemod) | Published |
| **Release** | `58c43de` | `libshouldersurfing.so` (Third person camera & crosshair) | [Release `mod-loader-shouldersurfing`](https://github.com/beyondbday69/starling/releases/tag/mod-loader-shouldersurfing) | Published |

---

## 5. Important Guidelines for Next Session
1. **Repository Path:** `/root/starling`
2. **Target Branch:** `feat/shader-fsr-upscaling`
3. **VPS Memory Limit:** VPS has only 2GB RAM. **Never run `cargo build` locally** on the VPS. Always build via GitHub Actions workflow (`workflow_dispatch` on `android.yml`).
4. **Git Remote Operations:** Always use `BypassSandbox: true` when running `gh` commands.
5. **Rule for Testing Native Mods:** Every APK build intended for testing native mods MUST pass `-f native_mods="..."` with all desired native mods to `gh workflow run android.yml`. When `native_mods` is omitted or empty, `build_apk.py` builds the plain vanilla APK without any native mods compiled into `libhorizon_android.so`.
6. **Built-in vs. Native Mods:** `holdmyitems`, `zoom`, `elytratrails`, and `livingmotion` are built directly into Starling core (`crates/horizon-client/src/game/`). Do NOT pass them in `native_mods` (doing so causes `E_MODULE_CONFLICT` in `horizon-mixins`). The external native mods are `cinematic_interactions`, `skins_3d`, `gyro`, and `water_physics`.

### 5.1. Exact Working Workflow Dispatch Command for APK Builds with Native Mods
```bash
gh workflow run android.yml --ref feat/water-physics-v2 -f native_mods="cinematic_interactions skins_3d gyro water_physics"
```

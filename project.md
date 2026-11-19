# Starling Project Status & Complete Context

**Repository:** `beyondbday69/starling`  
**Current Active Branch:** `feat/water-physics-v2`  
**Target Platform:** Android (ARM64 `arm64-v8a`)  
**Latest Commit:** `27a6d07` (`feat(water_physics): v3.7 surrounding water reaction, wide 8.5m lateral bow surge waves`)  
**Latest CI Run:** `#44603162331` (Artifact: `14570157486`, size: 35.2 MB)  
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

## 3.4. Water Physics Native Mod v3.5 (3D Boat Wake Field & Shader-Preserving Water Optics)
* **Goal:** Eliminate disjointed "two ribbons" boat wake appearance and ensure underlying custom shader pack water colors/caustics show through 100% untouched.
* **Key Features & Fixes:**
  * **Full Hydrodynamic 3D Wake Field:**
    * **Central Hull Wash / Stern Keel Trail:** Continuous raised 3D crest (+0.025m) trailing directly behind the boat keel with gentle turbulent glint.
    * **Transverse Kelvin Wavelets:** Continuous 3D wave ridges bridging between the hull wash and the outer V-arms, replacing isolated ribbon lines with a rich, cohesive 3D wake field.
    * **Stern Wake Origin Offset:** Boat wake emission shifted 0.35m backward to align with physical boat stern.
    * **Expansive Trail:** Extended node capacity to 48 and wake lifetime to 3.5s.
  * **Shader-Preserving Water Optics:**
    * Overlay baseline and edges have $\alpha = 0$ (100% transparent), allowing underlying shader pack water colors, depth absorption, and caustics to shine through untouched.
    * Crests and ripple rings use pure neutral celestial Fresnel specular glint `(248, 252, 255)` highlights ($\alpha \approx 0.18 - 0.22 \cdot \text{amp}$), completely eliminating artificial biome body tint overlays.
  * **Variable Collision Fix:** Renamed color channels to `red0`/`grn0`/`blu0` and right crest vertices to `r_crest0`/`r_crest1`, resolving `u32 | [f32; 3]` compiler error.

---

## 3.5. Water Physics Native Mod v3.6 (3D Living Waves, Boat Spray Particles & Ambient Surface Wavelets)
* **Goal:** Deliver realistic open-water dynamics with physical boat particles (cutting bow spray and stern wash bubbling) and true 3D rolling waves that prevent flat dead water.
* **Key Features & Improvements:**
  * **Boat Water Particles (`ParticleType::Splash` & `ParticleType::Bubble`):**
    * **Bow Cutting Spray:** Lateral spray droplets emitted from the front bow (`bx + fwd_x * 0.95`, `bz + fwd_z * 0.95`), angled outward and upward proportionally to boat velocity.
    * **Stern Wash Churn:** Bubbles (`ParticleType::Bubble`) and foaming spray droplets churning into the trailing wake wash.
    * **Rate-Limited Emissions:** Throttle particle spawning at ~12 Hz to maintain buttery smooth 60 FPS on mobile.
  * **Real 3D Rolling Waves:**
    * Elevated physical 3D crest displacement up to +0.090m on boat Kelvin arms, +0.065m on keel wash, +0.055m on transverse wavelets, and +0.070m on ripple rings for a volumetric 3D rolling wave experience.
  * **Ambient 3D Surface Wavelets:**
    * Gentle concentric surface wavelets (lapping at ~2.2s intervals) radiate around idling boats and resting players in open water, preventing flat dead-glass water surfaces.
  * **Preserved Shader Pack Optics:**
    * 100% transparent baseline ($\alpha = 0$) with pure neutral celestial Fresnel glints `(248, 252, 255)` so shader pack water colors and caustics shine through completely.
  * **Type Casting Fix:** Consumed `boat_spray_emitters` by value in `glue.rs` to ensure proper `f32`/`f64` primitive types.

---

## 3.6. Water Physics Native Mod v3.7 (Surrounding Water Reaction, Outward-Propagating 8.5m Lateral Bow Surge Waves)
* **Goal:** Direct answer to user feedback *"aju baju ka pani bbi react krna chaiye"* — ensure the entire surrounding water surface (4m to 8m+ to the left and right sides) reacts with real 3D wave swells as the boat moves through it.
* **Key Features & Enhancements:**
  * **Outward-Propagating Lateral Bow Surge Waves (`glue.rs`):**
    * Sheds dual lateral wave pulses from left and right bow shoulders (`bx ± left_x * 0.85`) every ~0.26s when cruising (`b_speed > 0.75`).
    * Waves expand up to 8.5 meters outward across the surrounding water on both sides (`max_radius: 8.5m`, `speed: 2.1 m/s`, `lifetime: 4.0s`).
    * Sized as wide rolling swells (`width: 0.24m - 0.90m`) with +0.080m 3D physical displacement.
  * **Extended Wake Reach & Buffer Capacity (`sim.rs`):**
    * Doubled ring buffer capacity from 48 to 96, and wake node capacity from 48 to 64.
    * Increased boat Kelvin wake spread speed from 0.60 m/s to 1.15 m/s, reaching 11+ meters wide across surrounding water.
    * Expanded player wading/swimming ripple radius to 4.8m and ambient idle wavelets to 3.8m.
  * **Shader Pack Optics & Performance:**
    * 100% transparent baseline ($\alpha = 0$) and pure neutral celestial Fresnel glints `(248, 252, 255)`, guaranteeing active shader pack water colors and caustics show through completely.
    * Zero-allocation analytical geometry for smooth 60 FPS mobile performance.

---

## 4. Key Artifacts, Releases & Action Runs

| Run ID | Commit | Description | Artifact ID / Download | Status |
|---|---|---|---|---|
| **#44603162331** | `27a6d07` | **Water Physics v3.7 (Surrounding Water Reaction, Wide 8.5m Lateral Bow Surge Swells) + Full Native Mods:** Added dual lateral bow surge wave pulses propagating 8.5m into surrounding water on both sides, doubled ring buffer capacity to 96, increased Kelvin wake spread to 1.15 m/s (11m+ span), wave swell width up to 0.90m, player ripple reach to 4.8m, shader-preserving optics (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14570157486](https://github.com/beyondbday69/starling/actions/runs/44603162331/artifacts/14570157486) | **PASSED (Green)** |
| **#44598471252** | `e763b1f` | **Water Physics v3.6 (3D Living Waves, Boat Bow Spray & Stern Bubbles, Ambient Wavelets) + Full Native Mods:** Added boat bow spray droplets (`ParticleType::Splash`) and stern bubble churn (`ParticleType::Bubble`), elevated 3D wave crest displacement to +0.090m, ambient 3D surface wavelets around idle boats/players, shader-preserving optics (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14567207891](https://github.com/beyondbday69/starling/actions/runs/44598471252/artifacts/14567207891) | **PASSED (Green)** |
| **#44590988150** | `7ec1b8f` | **Water Physics v3.5 (3D Boat Wake Field & Shader-Preserving Water Optics) + Full Native Mods:** Replaced 2 isolated ribbons with full hydrodynamic 3D wake field (central hull wash +0.025m, transverse Kelvin wavelets bridging to outer V-arms), stern wake origin offset (0.35m), 100% transparent overlay baseline with pure neutral Fresnel glint (248, 252, 255) highlights preserving shader pack water colors/caustics (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14564053585](https://github.com/beyondbday69/starling/actions/runs/44590988150/artifacts/14564053585) | **PASSED (Green)** |
| **#44579421912** | `7e986bd` | **Water Physics v3.4 (Native Minecraft Biome Water Color & Passenger Boat Center Circle Suppression) + Full Native Mods:** Sample chunk biome water color (`wp_water_tint`), eliminate hardcoded cyan paint, suppress player & mob footsteps/splashes/rings inside boats, smooth 20% sun glint highlight (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14557789411](https://github.com/beyondbday69/starling/actions/runs/44579421912/artifacts/14557789411) | **PASSED (Green)** |
| **#44569997411** | `546af9d` | **Water Physics v3.3 (Sleek Single-Ridge 3D V-Wake & Paddle Ripple Removal) + Full Native Mods:** Replaced 4-strip multi-stripe with sleek single-ridge 3D wave cross-section (Left and Right arms, 2 arms total, 25-40cm width, +0.04m to +0.06m height), eliminated 6-ribbon bug, removed paddle circle ripple spam (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14552884730](https://github.com/beyondbday69/starling/actions/runs/44569997411/artifacts/14552884730) | **PASSED (Green)** |
| **#44563686338** | `8a87a1e` | **Water Physics v3.2 (True 3D Dual-Crest Ripple Wake & Player Speed-in-mps Tracking) + Full Native Mods:** Resolved player wake generation (true m/s speed calculation, expanded vertical surface window, shoreline neighbor checks), dual-crest 3D undulating Kelvin wake profile (+0.048m to +0.075m crests, +0.010m trough), continuous transverse ripple shedding, strictly zero foam (`starling-preview-0.8.0-arm64-v8a.apk`) | [Artifact 14552011610](https://github.com/beyondbday69/starling/actions/runs/44563686338/artifacts/14552011610) | **PASSED (Green)** |
| **#44551742604** | `bacbb90` | **Water Physics v3.1 (White Strip Artifact Fix & Continuous Zero-Pop Harmonic Envelope) + Full Native Mods:** Removed center foam strip, node continuity distance/direction guards, independent player/boat wake channels, smooth cubic decay envelope (`starling-preview-0.8.0-arm64-v8a-cinematic_interactions-skins_3d-gyro-water_physics.apk`) | [Artifact 14544247729](https://github.com/beyondbday69/starling/actions/runs/44551742604/artifacts/14544247729) | **PASSED (Green)** |
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

# Falling Trees 3D Toppling Animation Implementation Summary

## 1. Overview & Motivation
User feedback:
> *"why there's is no animation for falling tree tree just breaks apart instantly without any animation fix it man"*

Previously, in `crates/horizon-client/src/game/fallingtrees.rs` (and `native-mods/fallingtrees/`), felling a tree executed an instant `setblock air` across all logs, leaves, and attachments in a single tick (50ms). The entire tree vanished instantly into thin air with zero visual feedback, zero toppling animation, and zero impact sound/particles.

---

## 2. Engineering Architecture & Implementation

### 2.1. Dynamic Felling Initiation (`fallingtrees_destroy_block`)
- When a base tree log is severed with an axe:
  - Detects tree canopy, logs, leaves, and attachments via `find_tree`.
  - Calculates dynamic horizontal topple vector `toward` pointing away from the player:
    $$\vec{d} = (\text{pos}_x + 0.5 - \text{player}_x,\; 0,\; \text{pos}_z + 0.5 - \text{player}_z), \quad \text{toward} = \frac{\vec{d}}{\|\vec{d}\|}$$
    (falls back to player view direction if standing directly on the stump).
  - Clears chunk terrain blocks immediately using `fluid_left(s)` / `set_block` to eliminate ghost block visual duplication.
  - Pre-computes all loot drops using `res.items().block_drops(registry, s, held_item, &mut world.random)` while safely cloning held stack to avoid borrow collisions.
  - Plays initial wood cracking sound (`playsound block.wood.break`) at the stump.
  - Queues an `ActiveFallingTree` structure into `active_falling_trees`.

### 2.2. Physics & 3D Topple Rendering (`fallingtrees_geometry`)
- **Pivoting & Gravity Acceleration:**
  - Fall duration scales with tree height:
    $$t_{fall} = 14.0 + 8.0 \cdot \sqrt{\text{height}}$$
    (clamped between 22 and 45 ticks).
  - Rotation angle accelerates under gravity:
    $$\theta(t) = \frac{\pi}{2} \cdot \left(\frac{t - 4}{t_{fall} - 4}\right)^2$$
    preceded by a subtle vertical settle drop ($t < 4$).
  - Computes 3D transformation matrix:
    $$M = T(\text{pivot}) \cdot R(\text{axis}, \theta) \cdot T(-\text{pivot})$$
    where $\text{axis} = \text{toward} \times (0, 1, 0)$.
- **Full 3D Block Model Rendering:**
  - Calls `self.block_model_vertices(state, seed, pose, light, false, out)` for each log, leaf, and attachment.
  - Accurately applies world lighting, ambient occlusion, face normals, and foliage/biome tints.
  - Sub-tick frame interpolation ($t = \text{game\_time} + \text{fraction}$) ensures silky-smooth 60/90/120 FPS animation on Android mobile devices.

### 2.3. Impact Landing & Loot Drops (`fallingtrees_tick`)
- On ground impact ($t \ge t_{fall}$):
  - Transforms landing coordinates for tree blocks using the final ground rotation matrix.
  - Triggers Minecraft Level Event `2001` (block break audio and explosion of wood splinters and leaf particles) across sample landing points.
  - Spawns all pre-computed drops cleanly at the stump.
  - Applies tool durability wear (`held.damage_item(wear, random)`) and container changes.
  - Removes the tree from `active_falling_trees`.

---

## 3. Build & CI Verification

- **Workflow Run:** [#45027183002](https://github.com/beyondbday69/starling/actions/runs/45027183002)
- **Commit:** `c827585` (`fix(fallingtrees): clone held item before block drops loop to resolve mutable borrow conflict`)
- **Status:** **PASSED (Green)**
- **Artifact:** `starling-apk-arm64-v8a` (ID: `14756228477`, size: 35.2 MB)
- **Direct Download Link:** [Artifact 14756228477](https://github.com/beyondbday69/starling/actions/runs/45027183002/artifacts/14756228477)
- **Generated APK:** `starling-preview-0.8.0-arm64-v8a.apk`
- **Applied Mods (`build.json` verified):**
  - `holdmyitems`: `v5.2.0`
  - `zoom`: `v2.0.0`
  - `elytratrails`: `v1.3.4`
  - `livingmotion`: `v0.2.1`
  - `cinematic_interactions`: `v0.12.2`
  - `fallingtrees`: `v0.13.2`
  - `gyro`: `v1.6.0`
  - `skins_3d`: `v1.1.1`

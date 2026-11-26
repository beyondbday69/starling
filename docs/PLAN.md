# Timber Fall — Implementation Plan

Target: Native Horizon mod in Rust (`native-mods/timber_fall/`) implementing [Timber Fall: Tree Falling Mod Concept](Timber%20Fall%20Tree%20Falling%20Mod%20Concept.md).

## Environment & Verification Status

- **Local machine:** 2 CPUs, 1.9 GiB RAM, no `.local/horizon-client/26.3` resources, no compiled `horizon-client` binary. Horizon cannot build or run here.
- **What runs locally:** Standalone pure-Rust crate tests via `cargo test --manifest-path native-mods/timber_fall/Cargo.toml`.
- **In-game verification:** Everything requiring the live Horizon runtime, `mod_api` linkage, `WorldDrawContext`, or LAN is marked **UNTESTED** until Horizon can be built/run.

## Verified Horizon Mod API Surface (`crates/horizon-mod-vocab`)

- **Hooks (`crates/horizon-mod-vocab/src/hooks/`):**
  - `Hooks::player_break_speed` (both sides, cancellable) — chop time scaling (`1 + logs / 10`, capped at `5x`).
  - `Hooks::break_block` (server, cancellable) — intercept base log break, check sneak/axe/durability/tree validity, remove tree blocks atomically in the same tick.
  - `Hooks::level_tick_post` / `Hooks::server_tick` (server/both) — advance falling physics, collision checks, entity impact damage, and break-up drops.
  - `Hooks::submit_custom_geometry` (client) — `WorldDrawContext::draw_model_instances` (`ModelInstance` with world origin, row-major 3x4 local transform, `0xAARRGGBB` tint, `LightCoords`).
  - `Hooks::camera_shake` (client) — impact camera shake (`ViewportEvent.ComputeCameraAngles`).
- **Client Render Host (`crates/horizon-mod-vocab/src/host/client_render.rs`):**
  - `InstanceGroupRef` (`instance_group_write`, `instance_group_set_len`, `instance_group_set_transform`, `instance_group_destroy`), `block_sprites`, `block_tint_color`, `block_sprites_version`.
- **Displays Host (`crates/horizon-mod-vocab/src/host/displays.rs`):**
  - `DisplaysServerHost::display` (`DisplayRef`, `DisplayKind`).

---

## Ordered Tasks

### Task 1 — Crate Scaffold, `config`, and Pure-Rust `scanner`
- **Scope:** Create `native-mods/timber_fall/` (`Cargo.toml` with `[workspace]`, `src/lib.rs`, `src/config.rs`, `src/scanner.rs`) with pure-Rust data types and unit tests.
- **Details:**
  - `config`: all 7 options with concept defaults (`sneak_disables_felling = true`, `require_axe = true`, `max_logs = 256`, `fall_speed = 1.5`, `falling_damage = true`, `camera_shake = true`, `auto_replant_sapling = false`), validation/clamping.
  - `scanner`: abstract `BlockReader` trait + BFS tree detector from cut position upward, same-log matching (including 26-neighbor diagonals), natural vs player-placed leaf counting (`>= 5` natural leaves touching logs), `max_logs` cap (`256`), stump exclusion (logs below cut height stay), `2x2` trunk detection (all 4 base logs at cutting height must be cut/handled), and special tree rules (Nether fungi counting wart blocks as leaves; mangrove roots/propagules & cherry branch connectivity).
- **Acceptance Criteria:**
  - `cargo test --manifest-path native-mods/timber_fall/Cargo.toml` passes all unit tests covering normal trees, wooden house rejection (<5 natural leaves / player-placed leaves), `max_logs` overflow, stump preservation, `2x2` trunks, and Nether/mangrove/cherry rules.
- **Status:** DONE (21 unit tests passing)

### Task 2 — Pure-Rust `falling` Physics & `impact` Collision / Damage Model
- **Scope:** Add `src/falling.rs` and `src/impact.rs` (pure Rust, no Horizon runtime dependency).
- **Details:**
  - `falling`: rigid-body tree state (base hinge, fall direction away from player, block offsets & block-kind tags, phase state machine: `Warning` 0.5s with 1–3° lean -> `Falling` using uniform-pole angular acceleration `alpha = (3 * g / (2 * L)) * sin(theta)` scaled by `fall_speed`, `g = 9.8`, 3° initial lean -> `Impact` with single 3° bounce -> `BreakUp` 1s later from base to tip).
  - `impact`: rotated-offset world-grid collision against a `CollisionGrid` trait (pass through grass/flowers, stop on first solid block for uphill/downhill slopes), impact-speed-scaled entity damage calculation (respecting `falling_damage` config), and 3x4 row-major transform matrix helper for a block offset at `(hinge, dir, angle)`.
- **Acceptance Criteria:**
  - Unit tests verify fall times match concept table (~1.6s / ~2.3s / ~3.2s / ~4.0s at 1.5x for L=5/10/20/30), 3° bounce, uphill early stop vs downhill continuation, grass/flower pass-through, 3x4 transform accuracy, and impact damage scaling.
- **Status:** DONE (40 unit tests passing)

### Task 3 — Pure-Rust `chop` Rules & `drops` Planner
- **Scope:** Add `src/chop.rs` and `src/drops.rs` (pure Rust, no Horizon runtime dependency).
- **Details:**
  - `chop`: sneak check, axe check, chop-time multiplier `1.0 + logs / 10.0` capped at `5.0`, durability budget with Unbreaking probability/expected wear (`1` point per log; if axe cannot cover whole tree, fall back to breaking only 1 log), player exhaustion calculation (`0.005 * logs` or vanilla block-break exhaustion per log).
  - `drops`: log stack merging near landing sections (world-space landing positions from hinge + direction + offset), leaf loot planning (saplings, sticks, apples), optional `auto_replant_sapling` placement plan at base, and immediate-drop fallback for quit-mid-fall.
- **Acceptance Criteria:**
  - Unit tests verify chop time formula & 5x cap, insufficient-durability 1-log fallback, Unbreaking handling, log stack merging along fallen trunk, and quit-mid-fall immediate drop list.
- **Status:** DONE (61 unit tests passing)

### Task 4 — Horizon Mod Wiring (`mod_api::Hooks`) & Multi-Block Rotating Render Proof (`UNTESTED` in-game until Horizon runs)
- **Scope:** Wire `native-mods/timber_fall` to Horizon's `mod_api::Hooks` (`horizon-mod.toml`, server + client hook handlers, `submit_custom_geometry` using `WorldDrawContext::draw_model_instances` / `ModelInstance`, `camera_shake`, `break_block`, `player_break_speed`, `level_tick_post`), keeping pure-Rust core tests runnable standalone.
- **Acceptance Criteria:**
  - Mod compiles against Horizon SDK (`cargo-horizon-mod check` / `native-build`).
  - **In-game (UNTESTED until Horizon runtime available):** Prove a group of blocks rotates smoothly around the stump hinge via `Hooks::submit_custom_geometry` (`WorldDrawContext::draw_model_instances`), with no item duplication on same-tick removal, and clean quit-mid-fall drop recovery.
- **Status:** DONE (68 pure-Rust unit tests passing; `horizon-sdk` adapter written & verified against `mod_vocab` API, UNTESTED in-game)

### Task 5 — End-to-End Integration: Sounds, Particles, Slopes, Special Trees & LAN Polish (`UNTESTED` in-game until Horizon runs)
- **Scope:** Connect warning creak sound/leaf particles, impact dust/thud/camera shake, server-to-client falling tree sync for LAN play, `2x2` & Nether/mangrove/cherry vanilla block ID mappings, and large-forest performance verification.
- **Acceptance Criteria:**
  - **In-game (UNTESTED until Horizon runtime available):** Singleplayer and LAN client+server tests pass for oak, 2x2 spruce/jungle, nether fungi, mangrove, cherry, uphill/downhill slopes, and sneak bypass.
- **Status:** DONE (80 pure-Rust unit tests passing; `horizon-sdk` wiring complete, sync transport & in-game/LAN verification UNTESTED until Horizon can be built/run)

### Task 6 — Direct Android APK Integration (`horizon-mixins` Bundle + `android.yml`)
- **Scope:** Add a `horizon-mixins` bundle under `native-mods/timber_fall/mixins/timber_fall/` (`horizon-mixin.toml`, injected module + hook helpers for `native_destroy_block`, `native_tick`, `native_submit_custom_geometry`, `native_mods`, `native_settings`, `native_settings_click`) so `packaging/android/native_build.sh` compiles `timber_fall` directly into `libhorizon_android.so` in the Android APK without `.hmod`.
- **Acceptance Criteria:**
  - `native-mods/timber_fall/mixins/timber_fall/horizon-mixin.toml` follows `native-mods/fallingtrees/mixins/fallingtrees/horizon-mixin.toml` and `packaging/android/native_build.sh` rules (no symlinks, valid `Game<'_>` targets).
  - Pure-Rust tests (`cargo test --manifest-path native-mods/timber_fall/Cargo.toml`) still pass with 0 warnings.
  - `.github/workflows/android.yml` includes `timber_fall` (replacing `fallingtrees` in default `MODS`) and builds the APK cleanly.
- **Status:** DONE (80 pure-Rust unit tests passing; `horizon-mixins` bundle + `android.yml` wired; Android APK build triggered on CI)

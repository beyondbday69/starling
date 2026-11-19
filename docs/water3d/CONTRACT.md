# Starling Water Physics v4 - Shared Module Contract

All parallel workers MUST follow this contract. Each worker owns exactly ONE new file
under `native-mods/water_physics/mixins/water_physics/`. Do NOT edit any existing file.

## Existing code (read-only reference)
- `sim.rs`      : `RippleRing`, `WakeNode`, `WaterSimulation` (analytical rings/wakes)
- `render.rs`   : builds `horizon_client::mesher::Vertex` directly into an overlay Vec
- `glue.rs`     : lifecycle, disturbance detection, particle spawning via
                  `self.particles.add(&ParticleOptions::simple(ParticleType::Splash), pos, vel)`
- `horizon-mixin.toml` : declares modules via `[[module]]` entries

## Hard rules
1. Pure Rust. No external crates except `glam` (already available). Prefer f32/f64 math with no deps.
2. Every public item is `pub`. No `unsafe`. No heap allocation in hot paths where avoidable.
3. Include a `#[cfg(test)] mod tests` with at least 2 meaningful unit tests.
4. Use `#[inline]` on small hot functions.
5. Target mobile: Android ARM64, 60-120 FPS, tight CPU budget. Prefer O(1) or O(n) with small n.
6. Do NOT use `std::sync::Mutex` or globals. State is passed explicitly by the caller.
7. Do NOT reference `crate::game::Game` unless your assigned task explicitly requires it.
   Most modules should be pure math/render-agnostic so they are easy to integrate.
8. Coordinate system: X/Z horizontal, Y up. World units are Minecraft blocks (1 block = 1 m).
   Water surface is at `water_y + 8.0/9.0` (flowing-water height).
9. Angles in radians unless the name says `_deg`.

## Naming
Every new file is prefixed `w3_` (water v4 working name). Example: `w3_gerstner.rs`.

## Integration (done by the root agent, NOT by workers)
The root will add `[[module]]` entries in `horizon-mixin.toml` and wire calls into
`glue.rs` / `render.rs`. Workers only deliver a self-contained file plus a short
report of its public API and how to call it.

## Goal
Upgrade flat-looking overlay water into believable 3D water:
- ambient directional wave field with real per-vertex normals
- shoreline shoaling, foam, flowing water and waterfalls
- volumetric particle systems (droplets, spray, mist, bubbles, foam flecks)
- Fresnel/reflection/refraction/absorption/caustics optics
- deterministic, allocation-light, mobile-friendly

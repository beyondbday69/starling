# Task: Smooth Tree Fall Animation + Dust Particles

## User Request
> "tree fall looks robotic not smopth zero dust psrticels"

## Root Causes Found in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`
1. **Robotic / stuttery motion:**
   - `timber_fall_render` increments `tree.smooth_secs` by `frame.clamp(0.0, 0.05)` independently of `elapsed_secs` and freezes if `last_tick_at.elapsed() >= 0.25`. Instead, lock `smooth_secs` to `(tree.elapsed_secs + tree.last_tick_at.elapsed().as_secs_f64().min(TICK_SECS)).max(tree.smooth_secs).min(DURATION)` so it stays silky 60/120 FPS and never drifts or stutters.
   - `shape_curve()` starts with `w = 0.0` at `th = 0.08`, causing the tree to stall nearly motionless for ~0.8 s and then whip down. Give it a small initial angular velocity (e.g. `w = 0.25..0.4`) and a natural, weighty ~1.0–1.15 s fall (`DURATION = 2.4`, `IMPACT = 0.44`, `BREAK = 0.64`, `LAST = 0.80` or similar well-proportioned timeline).
   - When `tree.impact_angle < FRAC_PI_2` (terrain hit before `IMPACT`), `tree_angle` hard-clamps `angle_at(p).min(tree.impact_angle)` and **freezes flat** until `p = IMPACT`, then suddenly snaps into the bounce. Track `impact_p: Option<f64>` when `timber_fall_hits` lowers `impact_angle`, and start the damped 3-hop bounce, `thud` sound, dust particles, and `timber_fall_camera_shake` **immediately** from `impact_p` (and use `impact_p` in `tree_angle` and `timber_fall_camera_shake`!).
2. **Zero dust particles:**
   - Inspect `crates/horizon-client/src/game/modding/native_level_effects.rs` (`level_send_particles`), `crates/horizon-client/src/game/modding/native_hooks/shared.rs` (`mod_particles_ok`), and `crates/horizon-client-render/src/particles/options.rs` (`ParticleOptions`, `ParticleType`).
   - Add a `timber_fall_particles(&mut self, dimension: &str, options: &ParticleOptions, state: Option<BlockStateId>, at: [f64; 3], spread: [f64; 3], speed: f64, count: i32)` helper using `self.level(dimension)` + `self.level_send_particles(...)`.
   - On **ground impact** (once when `thudded` triggers), emit dust puffs along 3–5 points of the fallen trunk (base, middle, tip using `rotate_point`) using `ParticleOptions::simple(ParticleType::Poof)` / `ParticleOptions::simple(ParticleType::Cloud)` and warm earthy `ParticleOptions::dust_transition([0.55, 0.45, 0.33], [0.72, 0.65, 0.55], 1.6)` (or `Block` crumbs if `mod_particles_ok` allows).
   - On **log pop** (at most once per tick at `popped_at`), emit a light `Poof` / `Smoke` puff (`count: 3`, `spread: [0.25, 0.25, 0.25]`, `speed: 0.02`).

## Verification & Reporting
- Verify every `ParticleOptions` / `ParticleType` path and `mod_particles_ok` acceptance rules in `horizon-client` (plus a small mock compile check for the new `level_send_particles` call and `impact_p` bounce math).
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` on `native-mods/timber_fall/Cargo.toml`.
- Remove `native-mods/timber_fall/target/`, append a concise report to `/root/starling/tasks/REPORT.md`, and run:
  `tmux send-keys -t 1 "review" Enter`

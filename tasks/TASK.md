# Task 10: Tone Down Pop White Flash to Match `voxel-tree-fall.html`

## User Feedback
> "white flash is too much white man check in html how it shoud look"

## Reference (`voxel-tree-fall.html`)
In `voxel-tree-fall.html`:
```js
const w = broken ? 0 : Math.max(0, 1 - (bp - p) * C.duration / .16);
log.scale.setScalar(1 + .35 * w * w);
log.material.forEach(x => x.emissive.setScalar(.7 * w * w * w));
```
- `w > 0.35` with `v.layer = 5` turned the entire log 100% solid pure white for ~0.10 s, which is much harsher than `voxel-tree-fall.html`'s smooth `0.7 * w^3` glow.

## Fix in `native-mods/timber_fall/mixins/timber_fall/runtime.rs`
In `timber_fall_render`:
- Keep the smooth swell (`scale = 1.0 + 0.35 * w * w`) over the last `0.16 s` (`w > 0.0`).
- For `w > 0.45 && w <= 0.86`: brighten the **textured** log smoothly without replacing its texture (`v.normal = [0, 0, 0, 0]` to remove dark directional shade/AO and `v.light = 0xF0F0` fullbright, keeping `v.layer` intact so bark grain stays visible!).
- Only apply `v.layer = 5` (pure white overlay) for the final brief pop-moment `w > 0.86` (~`0.022 s`, i.e. 1–2 frames right before it bursts) — so it gives a subtle bright swell + crisp brief white pop flash instead of turning into a solid white cube for 100 ms.

## Constraints
- Keep `fmt --check`, `clippy --all-targets -- -D warnings`, and `cargo test` passing on `native-mods/timber_fall/Cargo.toml` with 0 warnings.
- Clean up `native-mods/timber_fall/target/` and update `tasks/REPORT.md`.

## Test Commands
```sh
cargo fmt --manifest-path native-mods/timber_fall/Cargo.toml --check
cargo clippy --manifest-path native-mods/timber_fall/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path native-mods/timber_fall/Cargo.toml
```

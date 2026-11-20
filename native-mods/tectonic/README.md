# Tectonic (Rust Port for Starling / Horizon)

Ported from [Apollo's Tectonic](https://github.com/Apollounknowndev/tectonic) (v3.0.25) to native Rust for Starling/Horizon (Minecraft 26.3).

## Features
- **Grand Terrain**: Realistic, massive mountain ranges reaching up to world build heights, smoother valley transitions, and expansive plateaus.
- **Ocean Depths**: Configurable deeper ocean floors and ocean trenches, with automatic Ocean Monument structure depth alignment.
- **Underground Rivers**: Subterranean river systems continuing through mountain ranges with carved ceilings, ice in snowy biomes, and rare lantern decorations.
- **Lava Tunnels**: Rare, deep volcanic tunnels at the bottom of the world.
- **Aquifer Alignment**: Overworld aquifer lava levels dynamically aligned to `min_y + 10` (-54 for vanilla -64).
- **Options > Mods Settings**:
  - `Tectonic Generation: ON / OFF`
  - `Terrain Preset: Default / Large Biomes / Deserted / Frozen Wasteland / Overkill`
  - `Ocean Depth: Normal (-0.22) / Deeper (-0.35) / Trench (-0.45) / Shallow (-0.10)`
  - `Underground Rivers: ON / OFF`
  - `Lava Tunnels: ON / OFF`
  - `Snow Height Offset: +128m` (prevents low taiga snowline while preserving snowy mountain peaks)

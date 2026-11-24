# Sodium for Horizon & Starling

A native performance optimization and rendering engine mod for Minecraft: Java Edition (Starling / Horizon),
ported from **CaffeineMC's Sodium** (version 0.6.6).

## Features

- **Chunk Occlusion Culling**: Hierarchical occlusion culling to avoid vertex transformation and rasterization of hidden chunk faces.
- **Entity Distance Culling**: Configurable distance limits (32m, 64m, 96m, 128m, Unlimited) to prevent rendering offscreen or far-away entities.
- **Particle Management**: Dynamically throttles or limits active particle counts (All, Decreased, Minimal) to stabilize frame rates in intense scenes.
- **Smooth Lighting & Ambient Occlusion**: High-performance multi-level smooth lighting cache.
- **Biome Color Blending**: Optimized 5x5, 3x3, or Off sampling for grass, foliage, and water tints.
- **In-Game Video Settings & Performance Profiler**: Easily configurable under `Options > Mods > Sodium`.
- **Authentic CaffeineMC Branding**: Features the official Sodium cube icon and identity.

## License

GNU Lesser General Public License v3.0 (LGPL-3.0-only), matching CaffeineMC/sodium.

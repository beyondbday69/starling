# In-Game Resource Pack and Shader Downloader

## Overview
Built an in-game Resource Pack and Shader Pack downloader accessible directly from Minecraft's in-game UI (Options -> Resource Packs, Options -> Shaders, Options -> Mods), completely separate from any external launcher.

## Key Features
1. **In-Game Downloader UI**:
   - Integrated into Minecraft's native options screens on all platforms (Android touch & Desktop mouse/keyboard).
   - Dedicated shelves for `Resource Packs`, `Shader Packs`, and `Mods`.
   - Real-time search query box filtering Modrinth and reviewed registries.

2. **Pack Image & Thumbnail Loading**:
   - Asynchronous background download of pack preview icons from Modrinth CDN and registry.
   - Local disk caching at `mod-browser/images/<hash>-128.png`.
   - Decoded to 128x128 RGBA and dynamically uploaded to `HudAtlas` pack icon slots (`horizon/pack-icon0` .. `horizon/pack-icon63`) via `Patch`.
   - Rendered beside each pack entry with rounded corners (10.0 radius) and fallback to `unknown_pack`.

3. **Rich Metadata & Descriptions**:
   - Formatted download counters (e.g., `1.2M`, `45.8K`).
   - Category tags (e.g., `Iris, OptiFine`, `16x, Vanilla-like`).
   - Formatted multi-line summary and description.

4. **"All Versions" Version Selector**:
   - Dedicated "All Versions" modal sheet for any pack.
   - Asynchronously queries all released versions via `starling_catalog::project_versions`.
   - Version filter toggle: `[Version: 26.3 Only]` vs `[Version: All Versions]`.
   - Each version displays version number, loader compatibility, supported Minecraft versions (e.g., `MC: 26.3, 1.21.1, 1.20.4`), file size, and publication date.
   - Direct download and install for any specific version via `starling_catalog::install_version`.

5. **Direct Download & Installation Pipeline**:
   - Downloads `.zip` archives directly into `resourcepacks/` (for resource packs) or `shaderpacks/` (for shader packs).
   - Real-time progress bar showing percentage and downloaded bytes.
   - Automatically reloads pack caches so downloaded packs are immediately ready to use.

6. **Full Desktop & Mobile Control**:
   - Desktop: Left click navigation, mouse wheel list scrolling, arrow key scrolling, and keyboard typing in search.
   - Mobile: Full touch dragging, tap-to-select, and system keyboard integration.

## Verification
- **Unit Tests**: `cargo test -p starling-catalog --test catalog` passed (11 tests).
- **Public Tree Integrity**: `python3 tools/check_public_tree.py` passed.
- **CI Build**: [Run #45069407294](https://github.com/beyondbday69/starling/actions/runs/45069407294) passed.
- **Artifact**: [starling-apk-arm64-v8a (35.7 MB)](https://github.com/beyondbday69/starling/actions/runs/45069407294/artifacts/14772424890)
  - Generated APK: `starling-preview-0.8.0-arm64-v8a.apk`
  - Digest: `sha256:f5987a71774e4ba7a3209aefbd76c94e0976b5cea85514777de1fd8d4c9c2812`
  - Built with commit `5873124` on branch `feat/shader-fsr-upscaling`.


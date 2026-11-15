# Shader FSR 1.0 Upscaling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement AMD FidelityFX Super Resolution 1.0 (FSR 1.0) spatial upscaling (EASU + RCAS) in Starling's 100% Rust / WGSL renderer, so shader packs (Complementary, BSL, etc.) can render at 50%–75% resolution and upscale sharply to native display resolution with high framerates on mobile GPUs.

**Architecture:** In `crates/horizon-client-render/src/gfx/pack_post.rs`, when the shader pack render resolution is below window size (`size != window`), redirect the `final` shader pass into an offscreen target at `size`, followed by a two-pass FSR 1.0 post-process pipeline: Edge-Adaptive Spatial Upsampling (EASU) from `size` to `window`, then Robust Contrast-Adaptive Sharpening (RCAS) directly onto the presentation `screen`.

**Tech Stack:** Rust (2021 edition), `wgpu` 23.0 / Vulkan, WGSL (WebGPU Shading Language), Naga shader validator.

**Spec:** AMD FSR 1.0 Specification & Starling shader pipeline architecture (`docs/starling-shaders.md`).

---

## Global Constraints

- **Language & Runtime:** Pure Rust + WGSL shaders; zero C/C++ or foreign NDK runtime dependencies.
- **Vulkan / Mobile Friendly:** Shaders must compile cleanly with Naga for Vulkan 1.1+ (Adreno 6xx/7xx/8xx and Mali Gxx).
- **Aspect Ratio & Precision:** Maintain exact aspect ratio, subpixel UV alignment, and half-precision FP16 compatibility.
- **Zero Overhead when Inactive:** When render scale is 100% or FSR is disabled, bypass the FSR pipeline with zero extra passes.
- **Non-destructive:** Keep all existing shaders and fallback bilinear blits intact for devices where FSR is toggled off.

---

## Review Focus

1. **UV Coordinate Inversion:** WGPU texture coordinates vs OpenGL texture coordinates. Ensure EASU samples don't flip the Y axis upside down.
2. **Buffer Lifecycle & Resize:** Ensure intermediate FSR textures recreate cleanly when screen orientation changes or window resizes without leaking GPU memory.
3. **No-Final Pack Handling:** Some shader packs do not have a `final.fsh` (e.g. they terminate on `composite`). Ensure upscaling works correctly directly on `colortex0` when no final pass exists.
4. **Adreno Driver Quirks:** Avoid non-uniform texture sampling inside divergent control flow in WGSL; keep EASU sampling pattern unrolled.
5. **Alpha & Color Depth:** Ensure color channels clamp to valid SDR/HDR ranges without clipping emissive lighting marks.

---

## File Structure

- **Create:** `crates/horizon-client-render/src/gfx/fsr.wgsl`
  - Defines `vs_fsr` (full-screen quad), `fs_easu` (edge-adaptive spatial upsampling), and `fs_rcas` (contrast-adaptive sharpening).
- **Create:** `crates/horizon-client-render/src/gfx/fsr.rs`
  - Defines `FsrPipeline`, uniform buffer structures, bind group layouts, and draw execution helpers.
- **Modify:** `crates/horizon-client-render/src/gfx/pack_post.rs`
  - Integrate `FsrPipeline` into `PackPost::new`, `PackPost::resize`, and `PackPost::run`.
- **Modify:** `crates/horizon-client-render/src/graphics.rs`
  - Add `ShaderUpscaling` enum (`Off`, `Fsr`, `Cas`), `fsr_sharpness` setting, and config serialization.
- **Modify:** `crates/horizon-client-render/src/graphics_ui.rs`
  - Add UI controls for "Shader Upscaling" and "FSR Sharpness" in shader settings.
- **Create:** `crates/horizon-client-render/tests/fsr_tests.rs`
  - Unit tests verifying WGSL shader validation, uniform calculation, and pipeline setup.

---

## Tasks

### Task 1: FSR 1.0 WGSL Shaders (EASU & RCAS)

**Files:**
- Create: `crates/horizon-client-render/src/gfx/fsr.wgsl`
- Test: `crates/horizon-client-render/tests/fsr_tests.rs`

- [ ] **Step 1.1: Write `fsr.wgsl` with EASU and RCAS passes**
  - Implement full-screen vertex shader `vs_fsr(@builtin(vertex_index) i: u32) -> VertexOutput`.
  - Implement uniform struct `FsrConstants`:
    ```wgsl
    struct FsrConstants {
        src_size: vec2<f32>,
        dst_size: vec2<f32>,
        sharpness: f32,
        _pad: f32,
    }
    ```
  - Implement EASU fragment pass (`fs_easu`):
    - 12-tap sampling kernel analyzing horizontal/vertical directional gradients.
    - Edge interpolation along edge slopes rather than across boundaries.
  - Implement RCAS fragment pass (`fs_rcas`):
    - 5-tap neighborhood contrast calculation.
    - Negative lobe filtering with min/max clamping to eliminate ringing artifacts.
- [ ] **Step 1.2: Add unit test in `fsr_tests.rs` verifying WGSL compilation**
  - Use `naga::front::wgsl::parse_str` to verify valid syntax without Naga parser errors.
- [ ] **Step 1.3: Run test and commit**
  - Command: `cargo test -p horizon-client-render --test fsr_tests`
  - Commit: `feat(render): add FSR 1.0 EASU and RCAS WGSL shaders`

---

### Task 2: Rust `FsrPipeline` Module & Pipeline Creation

**Files:**
- Create: `crates/horizon-client-render/src/gfx/fsr.rs`
- Modify: `crates/horizon-client-render/src/gfx.rs`
- Test: `crates/horizon-client-render/tests/fsr_tests.rs`

- [ ] **Step 2.1: Define `FsrPipeline` struct in `fsr.rs`**
  - Holds:
    - `easu_pipeline: wgpu::RenderPipeline`
    - `rcas_pipeline: wgpu::RenderPipeline`
    - `bind_group_layout: wgpu::BindGroupLayout`
    - `uniform_buffer: wgpu::Buffer`
    - `intermediate_target: Option<wgpu::TextureView>`
    - `intermediate_size: [u32; 2]`
- [ ] **Step 2.2: Implement `FsrPipeline::new`, `resize`, and `run`**
  - `new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self`
  - `resize(device: &wgpu::Device, dst_size: [u32; 2])`
  - `run(encoder: &mut wgpu::CommandEncoder, src: &wgpu::TextureView, dst: &wgpu::TextureView, ...)`
- [ ] **Step 2.3: Add test in `fsr_tests.rs` for uniform generation & buffer sizing**
- [ ] **Step 2.4: Run test and commit**
  - Command: `cargo test -p horizon-client-render --test fsr_tests`
  - Commit: `feat(render): implement FsrPipeline in Rust`

---

### Task 3: Integrate FSR into Shader Pack Post-Processing (`pack_post.rs`)

**Files:**
- Modify: `crates/horizon-client-render/src/gfx/pack_post.rs:770-800`
- Modify: `crates/horizon-client-render/src/gfx/pack_post.rs:2630-2660`
- Modify: `crates/horizon-client-render/src/gfx/pack_post.rs:4220-4330`

- [ ] **Step 3.1: Allocate `final_out` render target when upscaling is needed**
  - When `size != window`, redirect `final_pass` color attachment to `targets.final_out` (allocated at `size`) instead of direct-to-`screen`.
- [ ] **Step 3.2: Hook FSR execution into `PackPost::run`**
  - If `needs_upscale` and FSR is enabled:
    - Pass 1 (EASU): Sample `final_out` at `size`, render to intermediate texture at `window`.
    - Pass 2 (RCAS): Sample intermediate texture, sharpen directly into `screen`.
  - Fallback: If FSR is disabled, execute existing bilinear blit (`self.copy`).
- [ ] **Step 3.3: Handle packs without `final` program**
  - Apply FSR directly from `colortex0` to `screen` when `shown == false`.
- [ ] **Step 3.4: Test and commit**
  - Command: `cargo test -p horizon-client-render`
  - Commit: `feat(render): wire FSR 1.0 upscaling into PackPost pipeline`

---

### Task 4: Graphics Settings & In-Game UI Controls

**Files:**
- Modify: `crates/horizon-client-render/src/graphics.rs`
- Modify: `crates/horizon-client-render/src/graphics_ui.rs`

- [ ] **Step 4.1: Add `ShaderUpscaling` enum in `graphics.rs`**
  - Presets: `Off`, `FSR`, `CAS`.
  - Default: `FSR`.
  - Add `fsr_sharpness: f32` (default: 0.2).
  - Add options serialization (`shaderUpscaling`, `fsrSharpness`).
- [ ] **Step 4.2: Add UI items in `graphics_ui.rs`**
  - Row for "Shader Upscaling: FSR 1.0 / CAS / Off".
  - Slider row for "FSR Sharpness" (0% to 100%).
- [ ] **Step 4.3: Add options round-trip test**
  - Verify options parse and stringify correctly without data loss.
- [ ] **Step 4.4: Test and commit**
  - Command: `cargo test -p horizon-client-render`
  - Commit: `feat(ui): add shader upscaling and FSR sharpness controls to settings`

---

### Task 5: End-to-End Build Verification

- [ ] **Step 5.1: Build all crates in Starling workspace**
  - Command: `cargo check --manifest-path /root/starling/Cargo.toml`
- [ ] **Step 5.2: Verify Android packaging manifest and assets**
  - Verify `fsr.wgsl` is packaged and accessible on Android runtime.
- [ ] **Step 5.3: Final commit & diff review**
  - Command: `git diff --stat origin/starling`

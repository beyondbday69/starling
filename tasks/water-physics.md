# Water Physics Native Mod Implementation Summary

## 1. Overview & Goal
`water_physics` is a native client-side visual mod for Starling (Minecraft 26.3 Android) that brings realistic fluid surface wave dynamics to water:
- **Boat Kelvin Wakes:** Classic V-cone wave envelope with arms trailing at the theoretical $19.47^\circ$ Kelvin angle ($\sin\alpha = c/v \approx 1/3$), feathery transverse wavelets, bow spray/foam trails, stern suction trough, and distinct oar ripples when paddling.
- **Player Wading / Walking:** Concentric circular ring ripples expanding from feet when moving through shallow water.
- **Player Swimming:** Streamlined V-wake trailing behind swimming players.
- **Falling Splashes:** Kinetic impact detection scaling with downward vertical velocity ($v_y$), creating an initial splash depression crater surrounded by an expanding circular shockwave rim and foamy splash crown.
- **Nearby Living Mobs:** Wakes and ripples from swimming/wading mobs within player vicinity.

---

## 2. Mathematical Simulation & Wave Equation (`sim.rs`)

### 2.1. 2D Heightfield Leapfrog Integration with Dispersion
The fluid simulation models the 2D wave equation on local heightfield grids:
$$\frac{\partial^2 h}{\partial t^2} = c^2 \nabla^2 h$$

Discretized using leapfrog time integration:
$$h_{new}(i, j) = 2 h(i, j) - h_{prev}(i, j) + C^2 \cdot \nabla^2 h(i, j) + \mu \cdot \text{dispersion} - \gamma \cdot (h(i, j) - h_{prev}(i, j))$$

Where:
- **$C^2 = 0.28$:** Dimensionless wave propagation speed parameter, tuned to match boat speeds for the $19.47^\circ$ Kelvin wake envelope.
- **Isotropic Laplacian:** Combined 4-neighbor cardinal laplacian and 4-neighbor diagonal laplacian ($0.5 \cdot \text{lap} + 0.25 \cdot \text{diag}$).
- **Dispersion Term ($\mu \cdot (\text{diag} - \text{lap})$):** Models frequency-dependent phase velocities, generating the characteristic feathery transverse wavelets behind moving disturbances.
- **Damping ($\gamma = 0.025$):** Dissipates energy over $1.5 - 3.0$ seconds without numerical oscillation.
- **Boundary Absorption Ramp:** A 2-cell absorbing perimeter dampens outward-traveling waves to zero at tile boundaries, preventing artificial wall reflections.

### 2.2. Fast-Fading Foam Mask
- Each grid cell maintains a foam intensity $f \in [0.0, 1.0]$.
- Decays multiplicatively by $0.94$ per simulation step (~$1$ second half-life).
- Injected at bow spray, splash impacts, and oar strokes.

### 2.3. Dynamic Wave Tiles & Mobile Performance
- **Local Coverage:** Each tile covers a $12.0 \times 12.0$ block area centered on disturbance points.
- **Energy-Based Sleeping:** Tracks total kinetic and potential energy ($\sum |h| + |h - h_{prev}|$). When energy drops below $0.0003$ for $35$ consecutive ticks, the tile enters an idle sleep state, consuming 0 CPU cycles.
- **Configurable Tile Cap:** Maximum concurrent active tiles capped (default $4$, configurable to $2, 4, 6$).
- **Configurable Grid Resolution:** Default $24 \times 24$ cells (576 floats, optimal for mobile ARM Mali/Adreno GPUs), configurable to $16 \times 16$ or $32 \times 32$.

---

## 3. Surface Overlay Rendering (`render.rs`)

- **Pipeline Pass:** Injected into `campfire_geometry` during entity/translucent world passes.
- **Layer & Blending:** `TRANSLUCENT_PARTICLE_OVERLAY | OIT_ENTITY` (layer 15 + OIT bit 24), rendering after terrain with alpha blending.
- **Heightfield Normals:** Computed on the fly via central finite differences:
  $$\vec{n} = \text{normalize}\left(-\frac{\partial h}{\partial x},\; 1.0,\; -\frac{\partial h}{\partial z}\right)$$
- **Lighting & Specular Shading:**
  - Sun vector $\vec{L} = \text{normalize}(0.161, 0.803, -0.562)$.
  - Blinn-Phong specular highlight $(N \cdot H)^{16}$ reflecting sunlight across wave crests.
  - Slope-dependent opacity: flat water is transparent; wave crests and foam become visible and crisp white.
- **Shoreline Culling:** Cells over non-water blocks (sand, dirt, stone banks) are automatically culled, preventing waves from overflowing onto dry land.
- **Flat Water Culling:** Cells with $|h| < 0.001$ and $foam < 0.01$ emit no geometry, preserving vertex throughput.

---

## 4. Entity Disturbance Detection (`glue.rs`)

1. **Local Player:**
   - Detects foot contact with water surface ($y_{water} = y_{block} + 8/9$).
   - Downward entry velocity $v_y < -0.18$ triggers a splash impact:
     $$\text{intensity} = (|v_y| \times 1.5).clamp(0.3, 2.0)$$
   - Walking/wading emits periodic ring ripples every 4 movement ticks.
   - Swimming emits a trailing V-wake impulse behind body motion.
2. **Boats:**
   - Bow spray impulse at $(x_{pos} + \vec{f} \cdot 0.85, z_{pos} + \vec{f} \cdot 0.85)$.
   - Stern depression trough at $(x_{pos} - \vec{f} \cdot 0.85, z_{pos} - \vec{f} \cdot 0.85)$.
   - Left and right paddle checks (`b.paddling[0]`, `b.paddling[1]`) emit lateral circular oar ripples at $\pm 0.90$m offset.
3. **Mobs:**
   - Active mobs in water within $24$m radius emit gentle ripples proportional to speed.

---

## 5. Mod Configuration & Defaults

- **Default State:** Enabled by default (`game.native_on("water_physics") == true`).
- **Options > Mods Settings Page:**
  - Button 0: Cycle Resolution (`16x16`, `24x24` [Default], `32x32`).
  - Button 1: Cycle Max Active Tiles (`2`, `4` [Default], `6`).
  - Button 2: Cycle Wave Intensity (`Low 0.7x`, `Normal 1.0x` [Default], `High 1.4x`).
- **Settings Persistence:** Stored in `native_data/water_physics/` via `native_set_setting`.
- **Logcat Tracing:** `log::info!` logs mod registration, settings changes, and splash impacts.

// Tectonic worldgen hooks and configuration constants for Starling/Horizon.

pub struct TectonicWorldgenConfig {
    pub vertical_scale: f32,
    pub elevation_boost: f32,
    pub min_y: i32,
    pub max_y: i32,
    pub lava_tunnels: bool,
    pub ocean_offset: f32,
    pub ocean_depth: f32,
    pub deep_ocean_depth: f32,
    pub monument_offset: i32,
    pub snow_start_offset: i32,
    pub underground_rivers: bool,
    pub rolling_hills: bool,
    pub jungle_pillars: bool,
    pub cheese_additive: f32,
    pub noodle_additive: f32,
}

impl Default for TectonicWorldgenConfig {
    fn default() -> Self {
        Self {
            vertical_scale: 1.125,
            elevation_boost: 0.0,
            min_y: -64,
            max_y: 384,
            lava_tunnels: true,
            ocean_offset: -0.8,
            ocean_depth: -0.22,
            deep_ocean_depth: -0.45,
            monument_offset: -30,
            snow_start_offset: 128,
            underground_rivers: true,
            rolling_hills: true,
            jungle_pillars: true,
            cheese_additive: 0.27,
            noodle_additive: -0.075,
        }
    }
}

/// Evaluates Tectonic's dynamic config constants for density functions.
pub fn config_constant(key: &str) -> f32 {
    match key {
        "vertical_scale" => 1.125,
        "elevation_boost" => 0.0,
        "min_y" => -64.0,
        "max_y" => 384.0,
        "lava_tunnels" => 1.0,
        "ocean_offset" => -0.8,
        "alternate_erosion_scale" => 1.0,
        "underground_rivers" => -1.0,
        "flat_terrain_skew" => 0.1,
        "rolling_hills" => 1.0,
        "jungle_pillars" => 1.0,
        "ocean_depth" => -0.22,
        "deep_ocean_depth" => -0.45,
        "depth_cutoff_start" => 0.1,
        "depth_cutoff_size" => 0.1,
        "cheese_enabled" => 1.0,
        "cheese_additive" => 0.27,
        "noodle_enabled" => 1.0,
        "noodle_additive" => -0.075,
        _ => 0.0,
    }
}

/// Offsets snow starting height in cold biomes by `snow_start_offset` (128 blocks up).
#[inline]
pub fn adjust_snow_height(y: i32, snow_start_offset: i32) -> i32 {
    y.saturating_sub(snow_start_offset)
}

/// Offsets Ocean Monument structures down to account for Tectonic's deeper oceans.
#[inline]
pub fn adjust_ocean_monument_y(y: i32, monument_offset: i32) -> i32 {
    y.saturating_add(monument_offset)
}

/// Aquifer lava level dynamically calculated from dimension min_y: min_y + 10 (-54 for -64).
#[inline]
pub fn aquifer_lava_level(min_y: i32) -> i32 {
    min_y + 10
}

//! Centralized level-of-detail presets for water v4 (w3).
//!
//! The v3 glue code hard-codes a few quality values (16/24/32 segments and 12/18/24
//! rings) while the v4 modules need additional budgets: ambient wave count,
//! particle caps, simulation/update rate and a caustics toggle. This module is the
//! single source of truth for those values so the root integration can wire every
//! renderer against one consistent preset.
//!
//! Design notes:
//! * Pure data, no globals and no allocations.
//! * All fields are public so callers can read or clone/modify a preset directly.
//! * [`QualitySettings::from_name`] is case-insensitive and accepts the v3 aliases
//!   (`Low`, `Smooth`/`Balanced`, `Ultra`) as well as `Default`/`Medium`/`High`.
//! * Unknown names fall back to [`QualitySettings::BALANCED`] rather than panicking.

/// Runtime LOD budgets for one water-quality preset.
///
/// Coordinates follow the shared contract: X/Z are horizontal, Y is up, world units
/// are Minecraft blocks and the water surface sits at `water_y + 8.0/9.0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualitySettings {
    /// Maximum number of simultaneous analytic/summed ambient waves.
    pub wave_count: usize,
    /// Angular tessellation segments used for ripple rings and wake cross-sections.
    /// Kept compatible with the existing v3 quality mapping: 16 / 24 / 32.
    pub segments: usize,
    /// Maximum number of active wave rings (v3 default is 48).
    pub max_wave_rings: usize,
    /// Maximum number of active wake nodes (v3 default is 48).
    pub max_wake_nodes: usize,
    /// Maximum number of splash/droplet particles that may be spawned per event.
    pub max_splash_particles: usize,
    /// Maximum number of spray/mist particles that may be active per emitter.
    pub max_spray_particles: usize,
    /// Maximum number of foam fleck particles that may be active per emitter.
    pub max_foam_flecks: usize,
    /// Target simulation/update frequency in hertz (e.g. 30 means every ~33 ms).
    pub update_hz: u32,
    /// Whether expensive refracted caustic projection is enabled.
    pub caustics: bool,
}

impl QualitySettings {
    /// Minimal CPU/GPU cost preset for low-end mobile devices.
    pub const LOW: Self = Self {
        wave_count: 2,
        segments: 16,
        max_wave_rings: 12,
        max_wake_nodes: 12,
        max_splash_particles: 6,
        max_spray_particles: 8,
        max_foam_flecks: 8,
        update_hz: 30,
        caustics: false,
    };

    /// Default preset: a balance between visual fidelity and mobile frame time.
    pub const BALANCED: Self = Self {
        wave_count: 4,
        segments: 24,
        max_wave_rings: 24,
        max_wake_nodes: 24,
        max_splash_particles: 12,
        max_spray_particles: 20,
        max_foam_flecks: 16,
        update_hz: 60,
        caustics: true,
    };

    /// Highest-fidelity preset for powerful devices.
    pub const ULTRA: Self = Self {
        wave_count: 6,
        segments: 32,
        max_wave_rings: 48,
        max_wake_nodes: 48,
        max_splash_particles: 20,
        max_spray_particles: 40,
        max_foam_flecks: 32,
        update_hz: 120,
        caustics: true,
    };

    /// Resolve a quality preset from a settings-string name.
    ///
    /// Matching is ASCII case-insensitive and trims surrounding whitespace.
    /// Recognized names:
    /// * Low tier: `"Low"`, `"Min"`, `"Minimum"`, `"Performance"`.
    /// * Balanced tier: `"Balanced"`, `"Default"`, `"Normal"`, `"Medium"`,
    ///   `"Smooth"` (the v3 name), `"High"`.
    /// * Ultra tier: `"Ultra"`, `"Max"`, `"Maximum"`, `"Very High"`.
    ///
    /// Any unrecognized value returns [`Self::BALANCED`].
    #[inline]
    pub fn from_name(name: &str) -> Self {
        let n = name.trim();
        if n.eq_ignore_ascii_case("Low")
            || n.eq_ignore_ascii_case("Min")
            || n.eq_ignore_ascii_case("Minimum")
            || n.eq_ignore_ascii_case("Performance")
        {
            Self::LOW
        } else if n.eq_ignore_ascii_case("Ultra")
            || n.eq_ignore_ascii_case("Max")
            || n.eq_ignore_ascii_case("Maximum")
            || n.eq_ignore_ascii_case("Very High")
        {
            Self::ULTRA
        } else {
            Self::BALANCED
        }
    }

    /// Seconds between two simulation updates. Always positive and finite.
    #[inline]
    pub fn update_interval(&self) -> f32 {
        let hz = if self.update_hz == 0 { 1 } else { self.update_hz };
        1.0 / hz as f32
    }
}

impl Default for QualitySettings {
    #[inline]
    fn default() -> Self {
        Self::BALANCED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_monotonic_and_sane_budgets() {
        let tiers = [QualitySettings::LOW, QualitySettings::BALANCED, QualitySettings::ULTRA];

        for p in tiers {
            assert!(p.wave_count > 0);
            assert!(p.segments >= 8, "segments must describe a usable ring: {p:?}");
            assert!(p.max_wave_rings > 0);
            assert!(p.max_wake_nodes > 0);
            assert!(p.max_splash_particles > 0);
            assert!(p.max_spray_particles > 0);
            assert!(p.max_foam_flecks > 0);
            assert!(p.update_hz > 0 && p.update_hz <= 240);
            assert!(p.update_interval() > 0.0 && p.update_interval().is_finite());
        }

        assert!(QualitySettings::LOW.wave_count < QualitySettings::BALANCED.wave_count);
        assert!(QualitySettings::BALANCED.wave_count < QualitySettings::ULTRA.wave_count);
        assert!(QualitySettings::LOW.segments < QualitySettings::BALANCED.segments);
        assert!(QualitySettings::BALANCED.segments < QualitySettings::ULTRA.segments);
        assert!(QualitySettings::LOW.max_wave_rings < QualitySettings::ULTRA.max_wave_rings);
        assert!(QualitySettings::LOW.max_spray_particles < QualitySettings::ULTRA.max_spray_particles);
        assert!(QualitySettings::LOW.update_hz < QualitySettings::BALANCED.update_hz);
        assert!(QualitySettings::BALANCED.update_hz < QualitySettings::ULTRA.update_hz);

        assert!(!QualitySettings::LOW.caustics);
        assert!(QualitySettings::BALANCED.caustics);
        assert!(QualitySettings::ULTRA.caustics);

        // Keep compatibility with the existing v3 quality -> segment mapping.
        assert_eq!(QualitySettings::LOW.segments, 16);
        assert_eq!(QualitySettings::BALANCED.segments, 24);
        assert_eq!(QualitySettings::ULTRA.segments, 32);
    }

    #[test]
    fn from_name_is_case_insensitive_and_handles_aliases() {
        assert_eq!(QualitySettings::from_name("Low"), QualitySettings::LOW);
        assert_eq!(QualitySettings::from_name("  low  "), QualitySettings::LOW);
        assert_eq!(QualitySettings::from_name("PERFORMANCE"), QualitySettings::LOW);

        assert_eq!(QualitySettings::from_name("Balanced"), QualitySettings::BALANCED);
        assert_eq!(QualitySettings::from_name("smooth"), QualitySettings::BALANCED);
        assert_eq!(QualitySettings::from_name("Default"), QualitySettings::BALANCED);

        assert_eq!(QualitySettings::from_name("Ultra"), QualitySettings::ULTRA);
        assert_eq!(QualitySettings::from_name(" uLtRa "), QualitySettings::ULTRA);
        assert_eq!(QualitySettings::from_name("maximum"), QualitySettings::ULTRA);
    }

    #[test]
    fn unknown_or_empty_names_fall_back_to_balanced() {
        assert_eq!(QualitySettings::from_name(""), QualitySettings::BALANCED);
        assert_eq!(QualitySettings::from_name("   "), QualitySettings::BALANCED);
        assert_eq!(QualitySettings::from_name("ultra-quality"), QualitySettings::BALANCED);
        assert_eq!(QualitySettings::default(), QualitySettings::BALANCED);
    }
}

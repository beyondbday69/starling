// CaffeineMC Sodium for Horizon (native mixin runtime): modern rendering engine
// replacement and graphics optimization mod for Minecraft.
// Settings are kept in `Options > Mods > Sodium`.

pub const ID: &str = "sodium";
pub const NAME: &str = "Sodium";
pub const VERSION: &str = "0.6.6";
pub const DESCRIPTION: &str =
    "A modern rendering engine replacement for Minecraft, which greatly improves frame rates and reduces micro-stutter. Official port.";

/// Mod settings for Options > Mods > Sodium.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SodiumSettings {
    pub chunk_occlusion: bool,
    pub entity_culling: bool,
    pub entity_distance: u32,
    pub particle_quality: u8,
    pub smooth_lighting: u8,
    pub biome_blend: u8,
    pub profiler: bool,
    pub memory_allocator: u8,
}

impl Default for SodiumSettings {
    fn default() -> Self {
        Self {
            chunk_occlusion: true,
            entity_culling: true,
            entity_distance: 64,
            particle_quality: 0, // 0: All, 1: Decreased, 2: Minimal
            smooth_lighting: 0,  // 0: Maximum, 1: Minimum, 2: Off
            biome_blend: 0,      // 0: 5x5, 1: 3x3, 2: Off
            profiler: false,
            memory_allocator: 0, // 0: Balanced, 1: Aggressive, 2: Conservative
        }
    }
}

impl SodiumSettings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let flag = |key: &str, default: bool| {
            get(key).map_or(default, |v| {
                let trimmed = v.trim();
                trimmed != "false" && trimmed != "off" && trimmed != "0"
            })
        };
        let number = |key: &str, default: u32| {
            get(key).and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(default)
        };
        let byte = |key: &str, default: u8| {
            get(key).and_then(|v| v.trim().parse::<u8>().ok()).unwrap_or(default)
        };

        Self {
            chunk_occlusion: flag("chunk_occlusion", true),
            entity_culling: flag("entity_culling", true),
            entity_distance: match number("entity_distance", 64) {
                32 | 64 | 96 | 128 | 256 => number("entity_distance", 64),
                _ => 64,
            },
            particle_quality: byte("particle_quality", 0).min(2),
            smooth_lighting: byte("smooth_lighting", 0).min(2),
            biome_blend: byte("biome_blend", 0).min(2),
            profiler: flag("profiler", false),
            memory_allocator: byte("memory_allocator", 0).min(2),
        }
    }

    pub fn entity_distance_label(&self) -> &'static str {
        match self.entity_distance {
            32 => "32 blocks",
            64 => "64 blocks",
            96 => "96 blocks",
            128 => "128 blocks",
            _ => "Unlimited",
        }
    }

    pub fn particle_quality_label(&self) -> &'static str {
        match self.particle_quality {
            0 => "All",
            1 => "Decreased",
            _ => "Minimal",
        }
    }

    pub fn smooth_lighting_label(&self) -> &'static str {
        match self.smooth_lighting {
            0 => "Maximum",
            1 => "Minimum",
            _ => "Off",
        }
    }

    pub fn biome_blend_label(&self) -> &'static str {
        match self.biome_blend {
            0 => "5x5 (Fancy)",
            1 => "3x3 (Fast)",
            _ => "Off",
        }
    }

    pub fn memory_allocator_label(&self) -> &'static str {
        match self.memory_allocator {
            0 => "Balanced",
            1 => "Aggressive",
            _ => "Conservative",
        }
    }

    pub fn labels(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Chunk Occlusion: {}", on_off(self.chunk_occlusion)),
            format!("Entity Culling: {}", on_off(self.entity_culling)),
            format!("Entity Distance: {}", self.entity_distance_label()),
            format!("Particles: {}", self.particle_quality_label()),
            format!("Smooth Lighting: {}", self.smooth_lighting_label()),
            format!("Biome Blend: {}", self.biome_blend_label()),
            format!("Profiler: {}", on_off(self.profiler)),
            format!("Memory Allocation: {}", self.memory_allocator_label()),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.chunk_occlusion = !self.chunk_occlusion,
            1 => self.entity_culling = !self.entity_culling,
            2 => {
                self.entity_distance = match self.entity_distance {
                    32 => 64,
                    64 => 96,
                    96 => 128,
                    128 => 256,
                    _ => 32,
                }
            }
            3 => self.particle_quality = (self.particle_quality + 1) % 3,
            4 => self.smooth_lighting = (self.smooth_lighting + 1) % 3,
            5 => self.biome_blend = (self.biome_blend + 1) % 3,
            6 => self.profiler = !self.profiler,
            7 => self.memory_allocator = (self.memory_allocator + 1) % 3,
            _ => {}
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 8] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("chunk_occlusion", flag(self.chunk_occlusion)),
            ("entity_culling", flag(self.entity_culling)),
            ("entity_distance", self.entity_distance.to_string()),
            ("particle_quality", self.particle_quality.to_string()),
            ("smooth_lighting", self.smooth_lighting.to_string()),
            ("biome_blend", self.biome_blend.to_string()),
            ("profiler", flag(self.profiler)),
            ("memory_allocator", self.memory_allocator.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_settings() {
        let s = SodiumSettings::default();
        assert!(s.chunk_occlusion);
        assert!(s.entity_culling);
        assert_eq!(s.entity_distance, 64);
        assert_eq!(s.particle_quality, 0);
        assert_eq!(s.smooth_lighting, 0);
        assert_eq!(s.biome_blend, 0);
        assert!(!s.profiler);
        assert_eq!(s.memory_allocator, 0);
        assert_eq!(s.labels().len(), 8);
    }

    #[test]
    fn button_clicks_cycle() {
        let mut s = SodiumSettings::default();
        // 0: chunk occlusion toggle
        s.click(0);
        assert!(!s.chunk_occlusion);
        s.click(0);
        assert!(s.chunk_occlusion);

        // 1: entity culling toggle
        s.click(1);
        assert!(!s.entity_culling);

        // 2: entity distance cycle (64 -> 96 -> 128 -> 256 -> 32 -> 64)
        s.click(2);
        assert_eq!(s.entity_distance, 96);
        s.click(2);
        assert_eq!(s.entity_distance, 128);
        s.click(2);
        assert_eq!(s.entity_distance, 256);
        assert_eq!(s.entity_distance_label(), "Unlimited");
        s.click(2);
        assert_eq!(s.entity_distance, 32);

        // 3: particles (0 -> 1 -> 2 -> 0)
        s.click(3);
        assert_eq!(s.particle_quality, 1);
        assert_eq!(s.particle_quality_label(), "Decreased");
        s.click(3);
        assert_eq!(s.particle_quality, 2);
        assert_eq!(s.particle_quality_label(), "Minimal");
        s.click(3);
        assert_eq!(s.particle_quality, 0);

        // 6: profiler
        s.click(6);
        assert!(s.profiler);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut original = SodiumSettings::default();
        original.chunk_occlusion = false;
        original.entity_distance = 128;
        original.particle_quality = 2;
        original.profiler = true;

        let mut map: HashMap<String, String> = HashMap::new();
        for (k, v) in original.stored() {
            map.insert(k.to_string(), v);
        }

        let recovered = SodiumSettings::read(|key| map.get(key).cloned());
        assert_eq!(original, recovered);
    }
}

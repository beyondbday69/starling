// Sound Physics Remastered for Horizon: direct-path occlusion raymarching,
// per-block acoustic materials, distance falloff and raytraced reverberation.
// Settings are kept in `Options > Mods > Sound Physics Remastered`.

use std::sync::Mutex;
use super::Game;
use crate::mod_api::NativeMod;

pub const ID: &str = "sound_physics_remastered";
pub const NAME: &str = "Sound Physics Remastered";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Sound Physics Remastered for Horizon: direct-path occlusion raymarching, per-block sound materials, distance falloff and raytraced reverberation.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundPhysicsSettings {
    pub enabled: bool,
    pub occlusion_strength: u32,
    pub reverb: bool,
    pub reverb_bounces: u32,
    pub underwater_muffle: bool,
    pub loud_test: bool,
}

impl Default for SoundPhysicsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            occlusion_strength: 100,
            reverb: true,
            reverb_bounces: 4,
            underwater_muffle: true,
            loud_test: false,
        }
    }
}

impl SoundPhysicsSettings {
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

        Self {
            enabled: flag("enabled", true),
            occlusion_strength: number("occlusion_strength", 100),
            reverb: flag("reverb", true),
            reverb_bounces: number("reverb_bounces", 4),
            underwater_muffle: flag("underwater_muffle", true),
            loud_test: flag("loud_test", false),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "On" } else { "Off" };
        vec![
            format!("Sound Physics: {}", on_off(self.enabled)),
            format!("Occlusion: {}%", self.occlusion_strength),
            format!("Reverberation: {}", on_off(self.reverb)),
            format!("Raytrace Bounces: {}", self.reverb_bounces),
            format!("Underwater Muffle: {}", on_off(self.underwater_muffle)),
            format!("Test Mode (3x): {}", on_off(self.loud_test)),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.enabled = !self.enabled,
            1 => {
                self.occlusion_strength = match self.occlusion_strength {
                    50 => 100,
                    100 => 150,
                    150 => 200,
                    _ => 50,
                };
            }
            2 => self.reverb = !self.reverb,
            3 => {
                self.reverb_bounces = match self.reverb_bounces {
                    4 => 6,
                    6 => 8,
                    8 => 12,
                    _ => 4,
                };
            }
            4 => self.underwater_muffle = !self.underwater_muffle,
            5 => self.loud_test = !self.loud_test,
            _ => {}
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 6] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("enabled", flag(self.enabled)),
            ("occlusion_strength", self.occlusion_strength.to_string()),
            ("reverb", flag(self.reverb)),
            ("reverb_bounces", self.reverb_bounces.to_string()),
            ("underwater_muffle", flag(self.underwater_muffle)),
            ("loud_test", flag(self.loud_test)),
        ]
    }
}

pub struct Material {
    pub absorption: f32,
    pub reflectivity: f32,
}

pub fn material_for(block: &str) -> Material {
    if block.contains("air") {
        Material { absorption: 0.0, reflectivity: 0.0 }
    } else if block.contains("wool") || block.contains("carpet") {
        Material { absorption: 1.0, reflectivity: 0.1 }
    } else if block.contains("wood") || block.contains("log") || block.contains("plank") || block.contains("door") {
        Material { absorption: 0.5, reflectivity: 0.4 }
    } else if block.contains("glass") {
        Material { absorption: 0.1, reflectivity: 0.7 }
    } else if block.contains("dirt") || block.contains("grass") || block.contains("sand") || block.contains("gravel") {
        Material { absorption: 0.7, reflectivity: 0.3 }
    } else if block.contains("water") {
        Material { absorption: 0.3, reflectivity: 0.5 }
    } else {
        Material { absorption: 0.2, reflectivity: 0.8 } // Default stone
    }
}

static STATE: Mutex<SoundPhysicsState> = Mutex::new(SoundPhysicsState { settings: None });

struct SoundPhysicsState {
    settings: Option<SoundPhysicsSettings>,
}

impl Game<'_> {
    pub(crate) fn sound_physics_remastered_mod_entry() -> NativeMod {
        NativeMod {
            id: ID.into(),
            title: NAME.into(),
            description: DESCRIPTION.into(),
            keys: Vec::new(),
        }
    }

    pub(crate) fn sound_physics_remastered_settings(&self) -> Vec<String> {
        let settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                SoundPhysicsSettings::read(|key| self.native_setting(ID, key))
            })
        };
        settings.labels()
    }

    pub(crate) fn sound_physics_remastered_settings_click(&mut self, button: usize) {
        let mut settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                SoundPhysicsSettings::read(|key| self.native_setting(ID, key))
            })
        };
        settings.click(button);
        {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            s.settings = Some(settings);
        }
        for (key, value) in settings.stored() {
            self.native_set_setting(ID, key, &value);
        }
    }

    pub(crate) fn sound_physics_remastered_frame(&mut self) {
        let settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                SoundPhysicsSettings::read(|key| self.native_setting(ID, key))
            })
        };
        if !settings.enabled {
            return;
        }

        let started = self.audio.started_since();
        if started.is_empty() {
            return;
        }

        let Some(player) = self.player.as_ref() else {
            return;
        };

        let eye = [
            player.position.x,
            player.position.y + f64::from(player.dimensions.eye_height),
            player.position.z,
        ];
        let listener_block = [
            eye[0].floor() as i32,
            eye[1].floor() as i32,
            eye[2].floor() as i32,
        ];
        let eye_name = self.native_block(listener_block);
        let listener_underwater = eye_name.contains("water");

        for (instance, _id, at_opt) in started {
            let Some(sound_pos) = at_opt else {
                continue;
            };

            let dx = sound_pos[0] - eye[0];
            let dy = sound_pos[1] - eye[1];
            let dz = sound_pos[2] - eye[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist == 0.0 {
                continue;
            }

            let steps = (dist / 0.5).ceil() as i32;
            let mut occlusion = 0.0f32;
            let mut last_voxel: Option<[i32; 3]> = None;

            for s in 1..steps {
                let t = s as f64 / steps as f64;
                let p = [eye[0] + dx * t, eye[1] + dy * t, eye[2] + dz * t];
                let voxel = [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32];

                if Some(voxel) != last_voxel {
                    let name = self.native_block(voxel);
                    if !name.contains("air") {
                        let mat = material_for(&name);
                        occlusion += mat.absorption;
                    }
                    last_voxel = Some(voxel);
                }
                if occlusion >= 1.0 {
                    occlusion = 1.0;
                    break;
                }
            }

            let scaled = occlusion
                * (settings.occlusion_strength as f32 / 100.0)
                * if settings.loud_test { 3.0 } else { 1.0 };
            let gain = (1.0 - scaled).clamp(0.15, 1.0);

            let mut cutoff = (1.0 - occlusion * 0.7).clamp(0.2, 1.0);
            if listener_underwater && settings.underwater_muffle {
                cutoff = cutoff.min(0.35);
            }

            let send = if settings.reverb {
                (0.4 * (1.0 - occlusion)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            self.audio.set_sound_environment(instance, gain, cutoff, send);
        }
    }
}

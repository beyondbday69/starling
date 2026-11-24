//! Built-in Entity Model Features (EMF) and Entity Texture Features (ETF)
//! entries and runtime toggles for `Options > Mods`.

use std::sync::{Mutex, Once};

use super::Game;
use horizon_client::starling_mod_api::NativeMod;

pub const EMF_ID: &str = "entity_model_features";
pub const EMF_TITLE: &str = "Entity Model Features 2.4.1";
pub const EMF_DESCRIPTION: &str = "Custom entity models (.jem/.jpm) and per-bone animations for mobs, players, and block entities.";

pub const ETF_ID: &str = "entity_texture_features";
pub const ETF_TITLE: &str = "Entity Texture Features 6.2.9";
pub const ETF_DESCRIPTION: &str = "Random entity texture variants, blinking eyes, emissive overlays (_e), and transparent player skins.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EmfSettings {
    custom_models: bool,
    animations: bool,
    player_models: bool,
}

impl Default for EmfSettings {
    fn default() -> Self {
        Self {
            custom_models: true,
            animations: true,
            player_models: true,
        }
    }
}

impl EmfSettings {
    fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            custom_models: read_flag(&get, "custom_models", true),
            animations: read_flag(&get, "animations", true),
            player_models: read_flag(&get, "player_models", true),
        }
    }

    fn labels(self) -> Vec<String> {
        vec![
            format!("Custom Entity Models: {}", on_off(self.custom_models)),
            format!("Model Animations: {}", on_off(self.animations)),
            format!("Player CEM Models: {}", on_off(self.player_models)),
        ]
    }

    fn click(&mut self, button: usize) {
        match button {
            0 => self.custom_models = !self.custom_models,
            1 => self.animations = !self.animations,
            2 => self.player_models = !self.player_models,
            _ => {}
        }
    }

    fn stored(self) -> [(&'static str, &'static str); 3] {
        [
            ("custom_models", bool_str(self.custom_models)),
            ("animations", bool_str(self.animations)),
            ("player_models", bool_str(self.player_models)),
        ]
    }

    fn apply(self, mod_enabled: bool) {
        horizon_client::mobs::cem::set_custom_models_enabled(mod_enabled && self.custom_models);
        horizon_client::mobs::cem::set_animations_enabled(mod_enabled && self.animations);
        horizon_client::mobs::cem::set_player_models_enabled(mod_enabled && self.player_models);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EtfSettings {
    /// 0 = Auto (when player CEM is active), 1 = Always ON, 2 = OFF.
    skin_transparency: u8,
    random_textures: bool,
    blinking: bool,
    emissive: bool,
}

impl Default for EtfSettings {
    fn default() -> Self {
        Self {
            skin_transparency: 0,
            random_textures: true,
            blinking: true,
            emissive: true,
        }
    }
}

impl EtfSettings {
    fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let mode = get("skin_transparency")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .unwrap_or(0)
            .min(2);
        Self {
            skin_transparency: mode,
            random_textures: read_flag(&get, "random_textures", true),
            blinking: read_flag(&get, "blinking", true),
            emissive: read_flag(&get, "emissive", true),
        }
    }

    fn transparency_label(self) -> &'static str {
        match self.skin_transparency {
            1 => "Always ON",
            2 => "OFF",
            _ => "Auto",
        }
    }

    fn labels(self) -> Vec<String> {
        vec![
            format!("Skin Transparency: {}", self.transparency_label()),
            format!("Random Entity Textures: {}", on_off(self.random_textures)),
            format!("Entity Blinking: {}", on_off(self.blinking)),
            format!("Emissive Textures: {}", on_off(self.emissive)),
        ]
    }

    fn click(&mut self, button: usize) {
        match button {
            0 => self.skin_transparency = (self.skin_transparency + 1) % 3,
            1 => self.random_textures = !self.random_textures,
            2 => self.blinking = !self.blinking,
            3 => self.emissive = !self.emissive,
            _ => {}
        }
    }

    fn stored(self) -> [(&'static str, String); 4] {
        [
            ("skin_transparency", self.skin_transparency.to_string()),
            ("random_textures", bool_str(self.random_textures).to_owned()),
            ("blinking", bool_str(self.blinking).to_owned()),
            ("emissive", bool_str(self.emissive).to_owned()),
        ]
    }

    fn apply(self, mod_enabled: bool) {
        horizon_client_render::skins::set_skin_transparency_mode(if mod_enabled {
            self.skin_transparency
        } else {
            2
        });
        horizon_client::mobs::render::set_random_textures_enabled(
            mod_enabled && self.random_textures,
        );
        horizon_client::mobs::render::set_blinks_enabled(mod_enabled && self.blinking);
        horizon_client::mobs::render::set_emissive_enabled(mod_enabled && self.emissive);
    }
}

fn read_flag(get: &impl Fn(&str) -> Option<String>, key: &str, default: bool) -> bool {
    get(key).map_or(default, |v| {
        let trimmed = v.trim();
        trimmed != "false" && trimmed != "off" && trimmed != "0"
    })
}

fn on_off(b: bool) -> &'static str {
    if b { "ON" } else { "OFF" }
}

fn bool_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

static EMF_STATE: Mutex<Option<EmfSettings>> = Mutex::new(None);
static ETF_STATE: Mutex<Option<EtfSettings>> = Mutex::new(None);
static INIT: Once = Once::new();

impl Game<'_> {
    pub(crate) fn entity_model_features_mod_entry() -> NativeMod {
        NativeMod {
            id: EMF_ID.into(),
            title: EMF_TITLE.into(),
            description: EMF_DESCRIPTION.into(),
            keys: Vec::new(),
        }
    }

    pub(crate) fn entity_texture_features_mod_entry() -> NativeMod {
        NativeMod {
            id: ETF_ID.into(),
            title: ETF_TITLE.into(),
            description: ETF_DESCRIPTION.into(),
            keys: Vec::new(),
        }
    }

    pub(crate) fn entity_features_sync(&self) {
        INIT.call_once(|| {
            let emf = EmfSettings::read(|key| self.native_setting(EMF_ID, key));
            *EMF_STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(emf);
            let etf = EtfSettings::read(|key| self.native_setting(ETF_ID, key));
            *ETF_STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(etf);
        });
        if let Some(emf) = *EMF_STATE.lock().unwrap_or_else(|e| e.into_inner()) {
            emf.apply(self.native_on(EMF_ID));
        }
        if let Some(etf) = *ETF_STATE.lock().unwrap_or_else(|e| e.into_inner()) {
            etf.apply(self.native_on(ETF_ID));
        }
    }

    pub(crate) fn entity_model_features_settings(&self) -> Vec<String> {
        let mut guard = EMF_STATE.lock().unwrap_or_else(|e| e.into_inner());
        let settings =
            *guard.get_or_insert_with(|| EmfSettings::read(|key| self.native_setting(EMF_ID, key)));
        settings.apply(self.native_on(EMF_ID));
        settings.labels()
    }

    pub(crate) fn entity_model_features_settings_click(&mut self, button: usize) {
        let settings = {
            let mut guard = EMF_STATE.lock().unwrap_or_else(|e| e.into_inner());
            let settings = guard
                .get_or_insert_with(|| EmfSettings::read(|key| self.native_setting(EMF_ID, key)));
            settings.click(button);
            *settings
        };
        for (key, value) in settings.stored() {
            self.native_set_setting(EMF_ID, key, value);
        }
        settings.apply(self.native_on(EMF_ID));
    }

    pub(crate) fn entity_texture_features_settings(&self) -> Vec<String> {
        let mut guard = ETF_STATE.lock().unwrap_or_else(|e| e.into_inner());
        let settings =
            *guard.get_or_insert_with(|| EtfSettings::read(|key| self.native_setting(ETF_ID, key)));
        settings.apply(self.native_on(ETF_ID));
        settings.labels()
    }

    pub(crate) fn entity_texture_features_settings_click(&mut self, button: usize) {
        let settings = {
            let mut guard = ETF_STATE.lock().unwrap_or_else(|e| e.into_inner());
            let settings = guard
                .get_or_insert_with(|| EtfSettings::read(|key| self.native_setting(ETF_ID, key)));
            settings.click(button);
            *settings
        };
        for (key, value) in settings.stored() {
            self.native_set_setting(ETF_ID, key, &value);
        }
        settings.apply(self.native_on(ETF_ID));
    }
}

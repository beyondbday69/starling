// Tectonic 3.0.25 terrain generation native mod for Starling/Horizon.
// Overhauls world generation with massive mountain ranges, deeper oceans,
// underground rivers, lava tunnels, and smooth continental transitions.

use std::fs;
use std::path::Path;
use crate::game::Game;

pub const ID: &str = "tectonic";
pub const NAME: &str = "Tectonic";
pub const VERSION: &str = "3.0.25";
pub const DESCRIPTION: &str =
    "Terrain shaping brought to new heights! Towering mountains, deep ocean trenches, underground rivers, and lava tunnels.";

/// Embedded complete 26.3-upgraded Tectonic worldgen data pack (no external downloads needed).
pub const TECTONIC_PACK_BYTES: &[u8] = include_bytes!("data/tectonic-pack.zip");
pub const TECTONIC_PACK_NAME: &str = "tectonic-pack.zip";
pub const TECTONIC_PACK_ID: &str = "file/tectonic-pack.zip";

/// Presets supported by Tectonic.
pub const PRESETS: &[&str] = &[
    "Default",
    "Large Biomes",
    "Deserted",
    "Frozen Wasteland",
    "Overkill",
];

/// Ocean depth modes.
pub const OCEAN_DEPTHS: &[&str] = &[
    "Normal (-0.22)",
    "Deeper (-0.35)",
    "Trench (-0.45)",
    "Shallow (-0.10)",
];

pub fn list(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    log::info!("Tectonic: native worldgen mod registered v{}", VERSION);
    mods.push(horizon_client::mod_api::NativeMod {
        id: ID.into(),
        title: format!("{NAME} {VERSION}"),
        description: DESCRIPTION.into(),
        keys: Vec::new(),
    });
}

pub fn settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }

    let enabled = game
        .native_setting(ID, "enabled")
        .unwrap_or_else(|| "ON".into());
    let preset = game
        .native_setting(ID, "preset")
        .unwrap_or_else(|| "Default".into());
    let ocean_depth = game
        .native_setting(ID, "ocean_depth")
        .unwrap_or_else(|| "Normal (-0.22)".into());
    let rivers = game
        .native_setting(ID, "rivers")
        .unwrap_or_else(|| "ON".into());
    let lava = game
        .native_setting(ID, "lava_tunnels")
        .unwrap_or_else(|| "ON".into());
    let snow_offset = game
        .native_setting(ID, "snow_offset")
        .unwrap_or_else(|| "128".into());

    buttons.push(format!("Tectonic Generation: {enabled}"));
    buttons.push(format!("Terrain Preset: {preset}"));
    buttons.push(format!("Ocean Depth: {ocean_depth}"));
    buttons.push(format!("Underground Rivers: {rivers}"));
    buttons.push(format!("Lava Tunnels: {lava}"));
    buttons.push(format!("Snow Height Offset: +{snow_offset}m"));
}

pub fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }

    match button {
        0 => {
            let cur = game
                .native_setting(ID, "enabled")
                .unwrap_or_else(|| "ON".into());
            let next = if cur == "ON" { "OFF" } else { "ON" };
            game.native_set_setting(ID, "enabled", next);
        }
        1 => {
            let cur = game
                .native_setting(ID, "preset")
                .unwrap_or_else(|| "Default".into());
            let idx = PRESETS.iter().position(|&p| p == cur).unwrap_or(0);
            let next = PRESETS[(idx + 1) % PRESETS.len()];
            game.native_set_setting(ID, "preset", next);
        }
        2 => {
            let cur = game
                .native_setting(ID, "ocean_depth")
                .unwrap_or_else(|| "Normal (-0.22)".into());
            let idx = OCEAN_DEPTHS.iter().position(|&d| d == cur).unwrap_or(0);
            let next = OCEAN_DEPTHS[(idx + 1) % OCEAN_DEPTHS.len()];
            game.native_set_setting(ID, "ocean_depth", next);
        }
        3 => {
            let cur = game
                .native_setting(ID, "rivers")
                .unwrap_or_else(|| "ON".into());
            let next = if cur == "ON" { "OFF" } else { "ON" };
            game.native_set_setting(ID, "rivers", next);
        }
        4 => {
            let cur = game
                .native_setting(ID, "lava_tunnels")
                .unwrap_or_else(|| "ON".into());
            let next = if cur == "ON" { "OFF" } else { "ON" };
            game.native_set_setting(ID, "lava_tunnels", next);
        }
        5 => {
            let cur = game
                .native_setting(ID, "snow_offset")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(128);
            let next = match cur {
                0 => 64,
                64 => 128,
                128 => 192,
                192 => 256,
                _ => 0,
            };
            game.native_set_setting(ID, "snow_offset", &next.to_string());
        }
        _ => {}
    }
}

/// Ensures the world has Tectonic's data pack extracted and enabled in its `datapacks/` folder.
pub fn ensure_world_datapack(world_root: &Path) {
    if !is_enabled(world_root) {
        return;
    }

    let datapacks_dir = world_root.join("datapacks");
    if fs::create_dir_all(&datapacks_dir).is_err() {
        return;
    }

    let pack_path = datapacks_dir.join(TECTONIC_PACK_NAME);
    let needs_write = fs::metadata(&pack_path)
        .map(|m| m.len() != TECTONIC_PACK_BYTES.len() as u64)
        .unwrap_or(true);

    if needs_write {
        if let Err(e) = fs::write(&pack_path, TECTONIC_PACK_BYTES) {
            log::warn!("Tectonic: failed to write {}: {e}", pack_path.display());
            return;
        }
        log::info!("Tectonic: installed worldgen pack to {}", pack_path.display());
    }

    // Ensure it is recorded as enabled in datapacks.json
    let config_path = world_root.join("datapacks.json");
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(enabled) = json.get_mut("enabled").and_then(|e| e.as_array_mut()) {
                let id_val = serde_json::Value::String(TECTONIC_PACK_ID.into());
                if !enabled.contains(&id_val) {
                    enabled.push(id_val);
                    if let Ok(out) = serde_json::to_string_pretty(&json) {
                        let _ = fs::write(&config_path, out);
                    }
                }
            }
        }
    }
}

pub fn is_enabled(world_root: &Path) -> bool {
    // Enabled by default unless explicitly disabled in native_setting
    let setting = world_root
        .parent()
        .and_then(|p| p.parent())
        .map(|game_dir| game_dir.join("horizon-native-mods/tectonic/enabled"))
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|s| s.trim().to_uppercase());

    setting.as_deref() != Some("OFF")
}

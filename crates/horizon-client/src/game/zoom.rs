//! Horizon's official Zoom (built-in core feature): hold the key (Z by default)
//! to zoom in, and turn the mouse wheel while holding it to zoom further in or out.
//! Settings are kept in `Options > Mods > Zoom`.

use std::cell::RefCell;
use super::Game;
use crate::mod_api::{NativeMod, NativeZoom};

/// The mod's id: its settings folder and its key's line.
pub const ID: &str = "zoom";
pub const VERSION: &str = "2.0.0";
pub const DESCRIPTION: &str =
    "Hold a key to zoom in; the mouse wheel changes the zoom while you hold it. Horizon's own.";
/// The key's name, and its default.
pub const KEY: &str = "zoom";
pub const DEFAULT_KEY: &str = "key.keyboard.z";
/// The zooms the starting zoom cycles through, and the wheel's limits.
pub const LEVELS: [u8; 6] = [2, 3, 4, 6, 8, 10];
const MIN: f32 = 1.25;
const MAX: f32 = 10.0;
/// Each wheel step multiplies the zoom by this.
const STEP: f32 = 1.25;

/// The settings, as the WebAssembly Zoom kept them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// The zoom the key starts at, one of `LEVELS`.
    pub level: u8,
    /// Ease in and out like the spyglass.
    pub smooth: bool,
    /// Turn slower the more you zoom in.
    pub sensitivity: bool,
}

impl Settings {
    /// From the stored values (`get` reads one key's file); the extra's
    /// defaults (4x, smooth, slower turning) for the ones missing.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let level = get("level")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .map_or(4, |v| v.clamp(2, MAX as i64) as u8);
        let flag = |key: &str| get(key).is_none_or(|v| v.trim() != "false");
        Self {
            level,
            smooth: flag("smooth"),
            sensitivity: flag("sensitivity"),
        }
    }
    /// The settings page's buttons, as the WebAssembly Zoom labelled them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Starting zoom: {}x", self.level),
            format!("Smooth zoom: {}", on(self.smooth)),
            format!("Slower turning: {}", on(self.sensitivity)),
        ]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => {
                let at = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(at + 1) % LEVELS.len()];
            }
            1 => self.smooth = !self.smooth,
            2 => self.sensitivity = !self.sensitivity,
            _ => {}
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 3] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("level", self.level.to_string()),
            ("smooth", flag(self.smooth)),
            ("sensitivity", flag(self.sensitivity)),
        ]
    }
}

#[derive(Default)]
struct State {
    settings: Option<Settings>,
    held: Option<f32>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

/// Replaces the settings (the settings page changed them).
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

/// The zoom key went down (only while playing) or up.
pub fn key(playing: bool, pressed: bool, get: impl Fn(&str) -> Option<String>) {
    if pressed && !playing {
        return;
    }
    let level = settings(get).level;
    STATE.with_borrow_mut(|s| {
        s.held = pressed.then(|| f32::from(level).clamp(MIN, MAX));
    });
}

/// Wheel steps: true when the key is held and they zoom.
pub fn wheel(steps: f32) -> bool {
    STATE.with_borrow_mut(|s| {
        let Some(z) = &mut s.held else {
            return false;
        };
        *z = (*z * STEP.powf(steps)).clamp(MIN, MAX);
        true
    })
}

/// The zoom in effect: (factor, smooth, turning scale), none when the key is up.
pub fn view() -> Option<(f32, bool, f32)> {
    STATE.with_borrow(|s| {
        let factor = s.held?;
        let settings = s.settings?;
        let turn = if settings.sensitivity {
            1.0 / factor
        } else {
            1.0
        };
        Some((factor, settings.smooth, turn))
    })
}

impl Game<'_> {
    /// The zoom key went down or up.
    pub(crate) fn zoom_key(&mut self, key: &str, pressed: bool) {
        if !self.native_on(ID) {
            return;
        }
        let bound = self.native_mod_key(ID, KEY, DEFAULT_KEY);
        if key != bound {
            return;
        }
        let playing = self.native_playing();
        self::key(playing, pressed, |name| self.native_setting(ID, name));
    }

    /// While the key is held, the wheel zooms instead of moving the hotbar.
    pub(crate) fn zoom_wheel(&self, steps: f32, claimed: &mut bool) {
        if self.native_on(ID) && self::wheel(steps) {
            *claimed = true;
        }
    }

    /// The zoom in effect, unless another mod's came first.
    pub(crate) fn zoom_view(&self, zoom: &mut Option<NativeZoom>) {
        if zoom.is_none()
            && self.native_on(ID)
            && let Some((factor, smooth, turn)) = self::view()
        {
            *zoom = Some(NativeZoom {
                factor,
                smooth,
                turn,
            });
        }
    }

    /// Zoom's row in Options > Mods, with its key.
    pub(crate) fn zoom_listed(mods: &mut Vec<NativeMod>) {
        mods.push(NativeMod {
            id: ID.into(),
            title: format!("Zoom {VERSION}"),
            description: DESCRIPTION.into(),
            keys: vec![[KEY.into(), "Zoom key".into(), DEFAULT_KEY.into()]],
        });
    }

    /// Zoom's settings page: buttons.
    pub(crate) fn zoom_page(&self, mod_id: &str, buttons: &mut Vec<String>) {
        if mod_id != ID {
            return;
        }
        let settings = self::settings(|key| self.native_setting(ID, key));
        buttons.extend(settings.labels());
    }

    /// A click on Zoom's settings page: change the setting and keep it.
    pub(crate) fn zoom_click(&mut self, mod_id: &str, button: usize) {
        if mod_id != ID {
            return;
        }
        let mut settings = self::settings(|key| self.native_setting(ID, key));
        settings.click(button);
        self::set_settings(settings);
        for (key, value) in settings.stored() {
            self.native_set_setting(ID, key, &value);
        }
    }
}

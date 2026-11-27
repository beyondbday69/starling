// Horizon's official Zoom, formerly the WebAssembly mod and before that a
// built-in extra: hold the key (Z by default) to zoom in, and turn the mouse
// wheel while holding it to zoom further in or out. With the cinematic bars on
// (the default) the HUD gives way while the key is held and two black bars
// slide in from the top and the bottom of the view, back out once it is let go.
// Its settings are the WebAssembly mod's files
// (`horizon-mods/.horizon-mod-data/zoom/level`, `smooth`, `sensitivity`,
// `cinematic`) and its key `key.zoom.zoom` in `horizon-mods.txt`, so a player
// keeps them. This file is both the mod's Cargo library (its tests) and the
// module the bundle adds to the game.
use std::cell::RefCell;
use std::time::Instant;

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
/// How long the cinematic bars take to slide in or back out.
const BAR_SECONDS: f32 = 0.18;
/// The share of the view's height one bar covers when it is fully in.
const BAR_HEIGHT: f32 = 0.12;
/// The longest frame the slide moves by: a stalled frame does not snap them in.
const BAR_MAX_STEP: f32 = 0.05;

/// The settings, as the WebAssembly Zoom kept them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// The zoom the key starts at, one of `LEVELS`.
    pub level: u8,
    /// Ease in and out like the spyglass.
    pub smooth: bool,
    /// Turn slower the more you zoom in.
    pub sensitivity: bool,
    /// Slide cinematic bars over the view while zooming, and hide the HUD.
    pub cinematic: bool,
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
            cinematic: flag("cinematic"),
        }
    }
    /// The settings page's buttons, as the WebAssembly Zoom labelled them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Starting zoom: {}x", self.level),
            format!("Smooth zoom: {}", on(self.smooth)),
            format!("Slower turning: {}", on(self.sensitivity)),
            format!("Cinematic bars: {}", on(self.cinematic)),
        ]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => {
                // As the extra cycled: an unknown level counts as the first.
                let at = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(at + 1) % LEVELS.len()];
            }
            1 => self.smooth = !self.smooth,
            2 => self.sensitivity = !self.sensitivity,
            3 => self.cinematic = !self.cinematic,
            _ => {}
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 4] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("level", self.level.to_string()),
            ("smooth", flag(self.smooth)),
            ("sensitivity", flag(self.sensitivity)),
            ("cinematic", flag(self.cinematic)),
        ]
    }
}

/// The HUD mode the bars hid, kept here so this crate needs no game or mod API
/// dependency: the same three modes the mod API's `HudMode` has, mapped by
/// the key hook (`key.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hud {
    /// HUD and hands shown.
    AllVisible,
    /// HUD hidden, first-person hands kept (F1's mode).
    NoHud,
    /// HUD and hands hidden.
    AllHidden,
}

#[derive(Default)]
struct State {
    /// Read on the first key press, then kept.
    settings: Option<Settings>,
    /// The zoom while the key is held; None when it is up.
    held: Option<f32>,
    /// The HUD mode before cinematic bars hid it, while the key is held.
    saved_hud: Option<Hud>,
    /// How far the cinematic bars have slid in, 0..=1.
    bar_progress: f32,
    /// When the bars were last advanced, for the frame's delta.
    last_bar_frame: Option<Instant>,
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

/// The bars took the HUD (`prev`) away: true when it was saved now, so the
/// caller hides it; a repeated press keeps the mode the hold found.
pub fn hide_hud(prev: Hud) -> bool {
    STATE.with_borrow_mut(|s| {
        let saved = s.saved_hud.is_none();
        s.saved_hud.get_or_insert(prev);
        saved
    })
}

/// The key is up (or Zoom is off): the HUD to put back, once; none when the
/// bars hid nothing.
pub fn show_hud() -> Option<Hud> {
    STATE.with_borrow_mut(|s| s.saved_hud.take())
}

/// `progress` after `dt` seconds sliding toward `target` (0 or 1) over
/// `BAR_SECONDS`: linear, so `eased` shapes it.
fn advance(progress: f32, dt: f32, target: f32) -> f32 {
    let step = dt.clamp(0.0, BAR_MAX_STEP) / BAR_SECONDS;
    if target >= progress {
        (progress + step).min(target)
    } else {
        (progress - step).max(target)
    }
}

/// Ease-out: the bars reach their height quickly and settle into it.
fn eased(progress: f32) -> f32 {
    let rest = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - rest * rest * rest
}

/// The height of each cinematic bar this frame, in GUI pixels, `0.0` when they
/// are out: in while the key is held with the bars on, back out once it is let
/// go.
pub fn bar_height(height: f32) -> f32 {
    let now = Instant::now();
    STATE.with_borrow_mut(|s| {
        let dt = s
            .last_bar_frame
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        s.last_bar_frame = Some(now);
        let down = s.held.is_some() && s.settings.is_none_or(|set| set.cinematic);
        s.bar_progress = advance(s.bar_progress, dt, if down { 1.0 } else { 0.0 });
        height * BAR_HEIGHT * eased(s.bar_progress)
    })
}

/// The zoom in effect: (factor, smooth, turning scale), none when the key
/// is up.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    #[test]
    fn reads_the_webassembly_mods_settings() {
        assert_eq!(
            Settings::read(stored(&[])),
            Settings {
                level: 4,
                smooth: true,
                sensitivity: true,
                cinematic: true,
            }
        );
        let s = Settings::read(stored(&[
            ("level", "8\n"),
            ("smooth", "false"),
            ("sensitivity", "true"),
        ]));
        assert_eq!(
            s,
            Settings {
                level: 8,
                smooth: false,
                sensitivity: true,
                cinematic: true,
            }
        );
        assert_eq!(Settings::read(stored(&[("level", "99")])).level, 10);
        let round: Vec<(&str, String)> = s.stored().into();
        let back: Vec<(&str, &str)> = round.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&back)), s);
    }

    #[test]
    fn the_cinematic_bars_are_on_unless_turned_off_and_stay_off() {
        // A player who never touched it gets the bars.
        assert!(Settings::read(stored(&[])).cinematic);
        for off in ["false", "false\n"] {
            assert!(!Settings::read(stored(&[("cinematic", off)])).cinematic);
        }
        // Button 3 flips it, off and on again, and the file keeps it.
        let mut s = Settings {
            level: 4,
            smooth: true,
            sensitivity: true,
            cinematic: true,
        };
        s.click(3);
        assert_eq!(s.labels()[3], "Cinematic bars: OFF");
        let off: Vec<(&str, String)> = s.stored().into();
        assert!(off.contains(&("cinematic", "false".to_owned())));
        let back: Vec<(&str, &str)> = off.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert!(!Settings::read(stored(&back)).cinematic);
        s.click(3);
        assert_eq!(s.labels()[3], "Cinematic bars: ON");
        assert!(s.cinematic);
    }

    #[test]
    fn a_stored_level_out_of_range_clamps_whatever_its_size() {
        let level = |v: &str| Settings::read(stored(&[("level", v)])).level;
        // Too high: the largest zoom, as for 99.
        for v in ["99", "255", "256", "300", "100000"] {
            assert_eq!(level(v), 10, "{v}");
        }
        // Too low: the smallest.
        for v in ["1", "0", "-5"] {
            assert_eq!(level(v), 2, "{v}");
        }
        // Not a number: the default.
        for v in ["", "x", "4x", "2.5"] {
            assert_eq!(level(v), 4, "{v:?}");
        }
    }

    #[test]
    fn buttons_cycle_as_the_extra_did() {
        let mut s = Settings {
            level: 10,
            smooth: true,
            sensitivity: false,
            cinematic: true,
        };
        s.click(0);
        assert_eq!(s.level, 2);
        s.level = 5;
        s.click(0);
        assert_eq!(s.level, 3);
        s.click(1);
        s.click(2);
        assert_eq!(
            s.labels(),
            [
                "Starting zoom: 3x",
                "Smooth zoom: OFF",
                "Slower turning: ON",
                "Cinematic bars: ON",
            ]
        );
    }

    #[test]
    fn the_hud_comes_back_once_the_hold_that_hid_it_ends() {
        // The first press saves the mode the bars found and asks to hide it.
        assert_eq!(show_hud(), None, "nothing hidden at the start");
        assert!(hide_hud(Hud::NoHud));
        // A repeat while held keeps the mode the hold found, not this one.
        assert!(!hide_hud(Hud::AllVisible));
        assert_eq!(show_hud(), Some(Hud::NoHud));
        // Letting go puts it back once: nothing is left to restore.
        assert_eq!(show_hud(), None);
    }

    #[test]
    fn the_bars_slide_in_and_out_over_their_time() {
        // In from nothing: a frame at a time, reaching 1 within BAR_SECONDS.
        let frame = BAR_SECONDS / 10.0;
        let mut progress = 0.0;
        for _ in 0..10 {
            progress = advance(progress, frame, 1.0);
        }
        assert!((progress - 1.0).abs() < 1e-5, "{progress}");
        // Never past the target, and a long frame (a stall) does not overshoot.
        assert_eq!(advance(0.9, 10.0, 1.0), 1.0);
        assert_eq!(advance(0.1, 10.0, 0.0), 0.0);
        // One step back out is the same length as one in.
        assert!((advance(1.0, frame, 0.0) - 0.9).abs() < 1e-5);
        // Ease-out: quick at first, settled at the end.
        assert_eq!(eased(0.0), 0.0);
        assert_eq!(eased(1.0), 1.0);
        assert!(eased(0.5) > 0.5);
        assert!(eased(0.25) > eased(0.0) && eased(0.75) > eased(0.25));
        // The bar's height is the share of the view the eased slide gives it.
        assert_eq!(180.0 * BAR_HEIGHT * eased(1.0), 21.6);
    }

    #[test]
    fn holds_zooms_with_the_wheel_and_lets_go() {
        let none = stored(&[]);
        key(false, true, &none);
        assert_eq!(view(), None, "not while a screen is open");
        assert!(!wheel(1.0));
        key(true, true, &none);
        assert_eq!(view(), Some((4.0, true, 0.25)));
        assert!(wheel(1.0));
        assert_eq!(view().map(|v| v.0), Some(5.0));
        assert!(wheel(40.0));
        assert_eq!(view().map(|v| v.0), Some(10.0));
        key(false, false, &none);
        assert_eq!(view(), None);
        assert!(!wheel(1.0));
    }
}

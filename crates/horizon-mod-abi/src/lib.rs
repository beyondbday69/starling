//! The boundary between this client and a mod built as a shared library.
//!
//! A mod is a `cdylib` that exports one symbol, [`ENTRY`], returning a [`ModDesc`]. The client
//! hands it a [`HostVTable`] and calls the function pointers it offers. Nothing that Rust owns
//! crosses this boundary — no `String`, no `Vec`, no `&str`, no trait object, no allocator — so a
//! mod carrying its own copy of `std` is fine.
//!
//! **A panic cannot cross it either.** An `extern "C"` function that unwinds aborts the process,
//! so a mod must catch its own panics inside each callback. [`guard`] does that.

use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The version of this boundary. A mod is refused when it disagrees.
pub const ABI: u32 = 7;

/// The symbol a mod exports.
pub const ENTRY: &str = "horizon_mod_entry";

/// The longest text a host call writes back, including the NUL.
pub const TEXT: usize = 1024;

/// What the client offers a mod.
///
/// Every function is called only with the client's thread, never from a thread of the mod's own,
/// and never while the client holds the mod's registration.
#[repr(C)]
pub struct HostVTable {
    /// [`ABI`] as the client sees it, so a mod can refuse to run rather than crash.
    pub abi: u32,
    /// The client's mod API level, so a mod can adapt instead of refusing (`horizon_mods::api_level::LEVEL`).
    pub api_level: u32,
    /// One line for the client's log. `level`: 0 error, 1 warn, 2 info, 3 debug.
    pub log: unsafe extern "C" fn(level: u32, msg: *const c_char),
    /// Read one of the mod's own settings (`key`), writing at most `len` bytes to `out` with a NUL.
    /// Returns the number of bytes written, or 0 when the setting is not set.
    pub setting: unsafe extern "C" fn(key: *const c_char, out: *mut c_char, len: usize) -> usize,
    /// Write one of the mod's own settings. An empty `value` clears it.
    pub set_setting: unsafe extern "C" fn(key: *const c_char, value: *const c_char),
    /// The listener's eye, in world coordinates. False when no world is open.
    pub listener: unsafe extern "C" fn(out: *mut [f64; 3]) -> u8,
    /// The block at `(x, y, z)` as its id (`minecraft:stone`), written to `out` with a NUL.
    /// Returns how many bytes were written, or 0 when no world is open.
    pub block_name:
        unsafe extern "C" fn(x: i32, y: i32, z: i32, out: *mut c_char, len: usize) -> usize,
    /// Whether the block's face on `side` (`0..5` = x-, x+, y-, y+, z-, z+) is sturdy.
    pub face_is_sturdy: unsafe extern "C" fn(x: i32, y: i32, z: i32, side: u32) -> u8,
    /// Whether the block at `(x, y, z)` is water.
    pub is_water: unsafe extern "C" fn(x: i32, y: i32, z: i32) -> u8,
    /// The first block between `from` and `to`, written to `out`; false when the segment is clear.
    pub raycast:
        unsafe extern "C" fn(from: *const [f64; 3], to: *const [f64; 3], out: *mut RayHitOut) -> u8,
    /// The sounds that started since this was last asked, up to `max`, into `out`. The client
    /// clears the list, so one call is one frame's worth.
    pub sounds_started: unsafe extern "C" fn(out: *mut StartedSound, max: usize) -> usize,
    /// How a started sound sounds: `gain` scales its volume, `cutoff` closes a low-pass over it
    /// (`0..1`, 1 open), `send` feeds the client's reverb (`0..1`). False when the sound has
    /// already finished.
    pub set_sound: unsafe extern "C" fn(instance: u64, gain: f32, cutoff: f32, send: f32) -> u8,
    /// The player, in a world. False on the title screen.
    pub player: unsafe extern "C" fn(out: *mut PlayerInfo) -> u8,
    /// A slot of the player's: `0` the held one, `1` the off hand, `2..=5` armor feet to head,
    /// `6..=41` the rest of the inventory. Writes the item's id into `out` (with a NUL) and its
    /// numbers into `info`. False when the slot is empty.
    pub item:
        unsafe extern "C" fn(slot: u32, out: *mut c_char, len: usize, info: *mut ItemInfo) -> u8,
    /// The world's clock. False on the title screen.
    pub clock: unsafe extern "C" fn(out: *mut WorldClock) -> u8,
    /// A key by name (`attack`, `use`, `forward`, `jump`, `sneak`, `sprint`, `inventory`, ...):
    /// bit 0 says the game knows it, bit 1 that it is held down right now.
    pub key: unsafe extern "C" fn(name: *const c_char) -> u32,
    /// Where this mod may keep files: written into `out` (with a NUL), never longer than `len`.
    /// Returns the bytes written, as every text call here does.
    pub data_dir: unsafe extern "C" fn(out: *mut c_char, len: usize) -> usize,
    /// One of this mod's own files, beside its settings (`assets/<name>`): read into `out`.
    /// Returns the bytes written, or -1 when there is no such file or it does not fit.
    pub asset: unsafe extern "C" fn(name: *const c_char, out: *mut c_char, len: usize) -> isize,
}

/// The player, as a mod sees them.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInfo {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Blocks a tick, as the client last sent it.
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    /// `0` survival, `1` creative, `2` adventure, `3` spectator.
    pub mode: u32,
    /// Bit 0 in water, 1 on ground, 2 sprinting, 3 sneaking.
    pub flags: u32,
}

impl PlayerInfo {
    pub fn in_water(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn on_ground(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn sprinting(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn sneaking(&self) -> bool {
        self.flags & 8 != 0
    }
}

/// One item, without its id (that arrives in the buffer beside it).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemInfo {
    pub count: u32,
    /// Damage taken, in the item's own units.
    pub damage: u32,
    /// How much it takes before it breaks; 0 when the item does not wear out.
    pub max_damage: u32,
    /// Bit 0 enchanted, 1 damaged at all.
    pub flags: u32,
}

/// The world's time, as a client reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldClock {
    /// Ticks since the world was made.
    pub game_time: i64,
    /// 0 at sunrise, 24000 to a day; it keeps counting past 24000.
    pub day_time: i64,
}

/// Where a ray met a block, in the mod's own coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RayHitOut {
    pub at: [f64; 3],
    pub block: [i32; 3],
    /// The face hit, as a unit vector along one axis.
    pub normal: [f64; 3],
}

/// A sound that started, as the client saw it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StartedSound {
    /// Its handle, for [`HostVTable::set_sound`].
    pub instance: u64,
    /// Its id (`minecraft:block.stone.break`), valid only for the call that filled this in.
    pub id: *const c_char,
    /// Whether `at` is where it plays; False for a relative sound.
    pub has_at: u8,
    pub at: [f64; 3],
}

/// What a camera mod asks of the perspective calculation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraOffset {
    /// 1 if this camera offset is active, 0 otherwise.
    pub active: u8,
    /// 1 to show the crosshair ('+') in third person for aiming, 0 to keep vanilla behavior.
    pub crosshair: u8,
    pub _pad: [u8; 2],
    /// Local right offset (positive = right shoulder, negative = left shoulder).
    pub right: f32,
    /// Local up offset (positive = higher).
    pub up: f32,
    /// Local forward offset.
    pub forward: f32,
    /// Third person camera distance. If <= 0, vanilla distance applies.
    pub distance: f32,
}

/// What an eye/face animation mod offers for the player model's face.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerEyes {
    /// 1 if this mod has active eye/face animation, 0 otherwise.
    pub active: u8,
    /// Blink state: 0 = open, 1 = half/squint, 2 = closed.
    pub blink: u8,
    /// Horizontal look: -1 = left, 0 = center, 1 = right.
    pub look_x: i8,
    /// Vertical look: -1 = down, 0 = center, 1 = up.
    pub look_y: i8,
    /// Blush level: 0 = off, 1 = soft, 2 = rosy, 3 = intense.
    pub blush: u8,
    /// Emotional expression: 0 = normal, 1 = happy/squint, 2 = wide/surprised.
    pub expression: u8,
    /// 1 if face_pixels holds an 8x8 RGBA face patch to blit onto the player skin.
    pub has_face_patch: u8,
    pub _pad: u8,
    /// 8x8 RGBA pixels (row-major: y=0..8, x=0..8) for the face front (x=8..16, y=8..16).
    pub face_pixels: [u32; 64],
}

impl Default for PlayerEyes {
    fn default() -> Self {
        Self {
            active: 0,
            blink: 0,
            look_x: 0,
            look_y: 0,
            blush: 0,
            expression: 0,
            has_face_patch: 0,
            _pad: 0,
            face_pixels: [0; 64],
        }
    }
}

/// First-person item holding settings from a mod.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoldMyItems {
    /// 1 if this mod wants to control first-person hand holding.
    pub active: u8,
    /// 1 to render the player's arm holding the main hand item.
    pub show_main_arm: u8,
    /// 1 to render the player's arm holding the offhand item.
    pub show_off_arm: u8,
    /// 1 to enable two-handed grip for swords & large tools when offhand is empty.
    pub two_handed: u8,
    /// Arm position offset (X, Y, Z in blocks).
    pub offset_x: f32,
    pub offset_y: f32,
    pub offset_z: f32,
    /// Arm rotation angles (Pitch, Yaw, Roll in degrees).
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl Default for HoldMyItems {
    fn default() -> Self {
        Self {
            active: 0,
            show_main_arm: 1,
            show_off_arm: 1,
            two_handed: 1,
            offset_x: 0.0,
            offset_y: 0.0,
            offset_z: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,
        }
    }
}

/// A row the client asks a mod for, through [`ButtonSink::push`].
#[repr(C)]
pub struct ButtonSink {
    /// The client's own pointer; pass it back unchanged.
    pub ctx: *mut c_void,
    /// Adds one label. The text is copied before the call returns.
    pub push: unsafe extern "C" fn(*mut c_void, *const c_char),
}

/// What a mod offers the client.
///
/// The three text fields must outlive the client's use of them: they are read once, right after
/// [`ENTRY`] returns, and never again.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ModDesc {
    /// [`ABI`] as the mod was built against.
    pub abi: u32,
    /// The mod API level it was built against.
    pub api_level: u32,
    /// Its id: `soundphysics`. Its settings and its folder are named after it.
    pub id: *const c_char,
    /// `Name 1.0`, as Options > Mods shows it.
    pub title: *const c_char,
    pub description: *const c_char,
    /// Called once, before the first frame. Returning false leaves the mod listed and idle.
    pub init: Option<unsafe extern "C" fn(*const HostVTable) -> bool>,
    /// Once a tick, at 20 Hz, in world, on the client's thread.
    pub tick: Option<unsafe extern "C" fn()>,
    /// Once a frame, with the part of a tick that has passed since the last one.
    pub frame: Option<unsafe extern "C" fn(f32)>,
    /// The mod's rows in Options > Mods, asked again each time its page opens.
    pub settings: Option<unsafe extern "C" fn(*const ButtonSink)>,
    /// A tap on row `button` of the mod's own page.
    pub settings_click: Option<unsafe extern "C" fn(u32)>,
    /// A camera offset for third person perspective.
    pub camera_offset: Option<unsafe extern "C" fn(f32, *mut CameraOffset)>,
    /// Eye animations, blinking, looking, and blushing for the player model.
    pub player_eyes: Option<unsafe extern "C" fn(f32, *mut PlayerEyes)>,
    /// First-person item holding configuration and arm rendering.
    pub hold_my_items: Option<unsafe extern "C" fn(f32, *mut HoldMyItems)>,
}

// A mod's description is read on one thread and its callbacks run on one thread, but the loader
// holds it behind the registry's lock, so it must be shareable.
unsafe impl Send for ModDesc {}
unsafe impl Sync for ModDesc {}

/// `p` as a `&str`, or None when it is null, not UTF-8 or empty.
///
/// # Safety
/// `p` is null or a NUL-terminated string that stays valid for the call.
pub unsafe fn text<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

/// Runs a mod's callback, catching a panic so it cannot unwind into the client, and returns the
/// panic's message so the caller can log it.
///
/// A mod's own callbacks must do this too — an `extern "C"` function that unwinds aborts the
/// process before the client can catch anything. This is the client's belt on top of that brace.
pub fn guard<R>(run: impl FnOnce() -> R) -> Result<R, String> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(value) => Ok(value),
        Err(payload) => Err(payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic with no message".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn text_reads_a_c_string_and_refuses_null() {
        let s = CString::new("soundphysics").unwrap();
        assert_eq!(unsafe { text(s.as_ptr()) }, Some("soundphysics"));
        assert_eq!(unsafe { text(std::ptr::null()) }, None);
    }

    #[test]
    fn an_empty_string_is_not_a_name() {
        let s = CString::new("").unwrap();
        assert_eq!(unsafe { text(s.as_ptr()) }, Some(""));
    }

    #[test]
    fn guard_keeps_a_value_and_catches_a_panic() {
        assert_eq!(guard(|| 7), Ok(7));
        assert_eq!(guard(|| panic!("boom")), Err("boom".to_owned()));
    }

    #[test]
    fn the_table_is_pointer_sized_and_ordered() {
        // The mod and the client must agree on the layout, so it is checked, not assumed.
        let pointer = std::mem::size_of::<*const ()>();
        // Two `u32`s, then ten function pointers. The calls only ever grow at the end, so a mod
        // built against an older ABI still reads the first ones correctly.
        assert_eq!(std::mem::size_of::<HostVTable>(), 8 + 16 * pointer);
        assert_eq!(std::mem::size_of::<ButtonSink>(), 2 * pointer);
        assert_eq!(std::mem::offset_of!(HostVTable, abi), 0);
        assert_eq!(
            std::mem::offset_of!(HostVTable, set_setting),
            8 + 2 * pointer
        );
        assert_eq!(std::mem::offset_of!(HostVTable, listener), 8 + 3 * pointer);
        assert_eq!(std::mem::offset_of!(HostVTable, player), 8 + 10 * pointer);
        assert_eq!(std::mem::offset_of!(ModDesc, abi), 0);
        assert_eq!(
            std::mem::offset_of!(ModDesc, camera_offset),
            8 + 8 * pointer
        );
        assert_eq!(std::mem::offset_of!(ModDesc, player_eyes), 8 + 9 * pointer);
        assert_eq!(
            std::mem::offset_of!(ModDesc, hold_my_items),
            8 + 10 * pointer
        );
        assert_eq!(std::mem::size_of::<CameraOffset>(), 20);
        assert_eq!(std::mem::offset_of!(CameraOffset, crosshair), 1);
        assert_eq!(std::mem::size_of::<PlayerEyes>(), 264);
        assert_eq!(std::mem::offset_of!(PlayerEyes, has_face_patch), 6);
        assert_eq!(std::mem::offset_of!(PlayerEyes, face_pixels), 8);
        assert_eq!(std::mem::size_of::<HoldMyItems>(), 28);
        assert_eq!(std::mem::offset_of!(HoldMyItems, offset_x), 4);
    }
}

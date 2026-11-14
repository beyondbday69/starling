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
pub const ABI: u32 = 2;

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
    /// Sets how loud a started sound plays (`gain` scales its volume, `pitch` its speed). False when
    /// the sound has already finished.
    pub set_sound: unsafe extern "C" fn(instance: u64, gain: f32, pitch: f32) -> u8,
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
        // Two `u32`s, then three function pointers.
        assert_eq!(std::mem::size_of::<HostVTable>(), 8 + 3 * pointer);
        assert_eq!(std::mem::size_of::<ButtonSink>(), 2 * pointer);
        assert_eq!(std::mem::offset_of!(HostVTable, abi), 0);
        assert_eq!(std::mem::offset_of!(ModDesc, abi), 0);
    }
}

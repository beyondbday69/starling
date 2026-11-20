//! The test mod for the mod loader.
//!
//! It exists to prove one chain end to end: a `.so` in the game folder is listed in Options > Mods,
//! started once, called once a frame, and its row changes the game's settings. It does nothing a
//! player would notice inside a world.
//!
//! Every callback catches its own panic: an `extern "C"` function that unwinds aborts the process,
//! so a mod that lets a panic out kills the game instead of itself.

use horizon_mod_abi::{ButtonSink, HostVTable, ModDesc, guard};
use std::ffi::{CString, c_char};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

/// Counts frames, so a row can show that `frame` is really being called.
static FRAMES: AtomicU32 = AtomicU32::new(0);
/// The client's table, once it has been handed over.
static HOST: OnceLock<&'static HostVTable> = OnceLock::new();
/// Set when the row that raises a panic is tapped, so a caught panic can be seen working.
static BOOM: AtomicU32 = AtomicU32::new(0);

fn host() -> Option<&'static HostVTable> {
    HOST.get().copied()
}

fn log(level: u32, text: &str) {
    let Some(host) = host() else {
        return;
    };
    let Ok(text) = CString::new(text) else {
        return;
    };
    // SAFETY: the table is the client's, and the text outlives the call.
    unsafe { (host.log)(level, text.as_ptr()) }
}

unsafe extern "C" fn init(table: *const HostVTable) -> bool {
    if table.is_null() {
        return false;
    }
    // SAFETY: the client keeps the table for the life of the process.
    let table = unsafe { &*table };
    if table.abi != horizon_mod_abi::ABI {
        return false;
    }
    let _ = HOST.set(table);
    log(2, "hello: started, abi agrees");
    true
}

unsafe extern "C" fn tick() {
    guard(|| ()).ok();
}

unsafe extern "C" fn frame(_partial: f32) {
    let _ = guard(|| {
        let seen = FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
        if seen == 100 {
            log(2, "hello: 100 frames");
        }
    });
}

unsafe extern "C" fn settings(sink: *const ButtonSink) {
    let _ = guard(|| {
        if sink.is_null() {
            return;
        }
        // SAFETY: the client's sink is valid for this call and copies each label.
        let sink = unsafe { &*sink };
        for label in [
            format!("Hello: {} frames", FRAMES.load(Ordering::Relaxed)),
            "Hello: log a line".to_owned(),
            "Hello: raise a panic (it is caught)".to_owned(),
        ] {
            if let Ok(text) = CString::new(label) {
                // SAFETY: as above.
                unsafe { (sink.push)(sink.ctx, text.as_ptr()) };
            }
        }
    });
}

unsafe extern "C" fn settings_click(button: u32) {
    let _ = guard(|| match button {
        1 => log(2, "hello: the row was tapped"),
        2 => {
            BOOM.fetch_add(1, Ordering::Relaxed);
            panic!("hello: a panic on purpose, to show the client catching it");
        }
        _ => {}
    });
}

static ID: &[u8] = b"hello\0";
static TITLE: &[u8] = b"Hello 0.1\0";
static DESCRIPTION: &[u8] = b"The loader's test mod: listed, started, one row that logs.\0";

static DESC: ModDesc = ModDesc {
    abi: horizon_mod_abi::ABI,
    api_level: 7,
    id: ID.as_ptr() as *const c_char,
    title: TITLE.as_ptr() as *const c_char,
    description: DESCRIPTION.as_ptr() as *const c_char,
    init: Some(init),
    tick: Some(tick),
    frame: Some(frame),
    settings: Some(settings),
    settings_click: Some(settings_click),
    camera_offset: None,
    player_eyes: None,
};

/// The one symbol a mod exports: what it is, and the callbacks it offers.
///
/// # Safety
/// Called by the client with a table that stays valid for the life of the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(
    table: *const HostVTable,
) -> *const horizon_mod_abi::ModDesc {
    let _ = table;
    &DESC
}

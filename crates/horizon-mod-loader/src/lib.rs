//! Loads a mod built as a shared library, instead of compiling it into the client.
//!
//! The client finds mods in `horizon-native-mods/<id>/lib*.so` (or directly in that folder), calls
//! [`horizon_mod_abi::ENTRY`] once per library to get its `ModDesc`, checks that the two agree on
//! the boundary, and from then on calls the mod's function pointers. Nothing about the client is
//! patched, so a mod costs no build and no `.so` the size of the whole game.
//!
//! All of the pointer work is here, so the client's own code needs no `unsafe`: it implements
//! [`Host`], and asks this crate for the mods, the frames and the settings rows.
//!
//! A library is never unloaded. A renderer hook would keep a pointer into it for the life of the
//! process anyway (`RenderHooks` is a `OnceLock`), so unloading could only be safe for mods that
//! never touch the frame, and those are not worth a different rule.

use horizon_mod_abi::{
    ABI, ButtonSink, CameraOffset, HostVTable, ItemInfo, ModDesc, PlayerInfo, RayHitOut,
    StartedSound, WorldClock, guard,
};

#[cfg(target_os = "android")]
mod android;
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// What a mod may ask of the client while it runs.
///
/// The client implements this; this crate turns it into the [`HostVTable`] a mod is handed. The
/// `id` is always the id of the mod being called, so one implementation serves every mod.
pub trait Host {
    /// One line for the client's log. `level`: 0 error, 1 warn, 2 info, 3 debug.
    fn log(&self, level: u32, text: &str);
    /// One of `id`'s settings, or None when it is not set.
    fn setting(&self, id: &str, key: &str) -> Option<String>;
    /// Keeps one of `id`'s settings; an empty `value` clears it.
    fn set_setting(&mut self, id: &str, key: &str, value: &str);
    /// The listener's eye. None while no world is open.
    fn listener(&self) -> Option<[f64; 3]> {
        None
    }
    /// The block at `pos` as its id (`minecraft:stone`). None while no world is open.
    fn block_name(&self, _pos: [i32; 3]) -> Option<String> {
        None
    }
    /// Whether the block's face on `side` (`0..5` = x-, x+, y-, y+, z-, z+) is sturdy.
    fn face_is_sturdy(&self, _pos: [i32; 3], _side: usize) -> bool {
        false
    }
    /// Whether the block at `pos` is water.
    fn is_water(&self, _pos: [i32; 3]) -> bool {
        false
    }
    /// The first block between two points. None when the segment is clear or no world is open.
    fn raycast(&self, _from: [f64; 3], _to: [f64; 3]) -> Option<horizon_mod_abi::RayHitOut> {
        None
    }
    /// The sounds that started since this was last asked.
    fn sounds_started(&mut self) -> Vec<StartedInfo> {
        Vec::new()
    }
    /// How a started sound sounds: `gain` scales it, `cutoff` closes a low-pass over it (1 open),
    /// `send` feeds the reverb. False when it has already finished.
    fn set_sound(&mut self, _instance: u64, _gain: f32, _cutoff: f32, _send: f32) -> bool {
        false
    }
    /// The player, in a world.
    fn player(&self) -> Option<PlayerInfo> {
        None
    }
    /// A slot of the player's: `(item id, its numbers)`; slot 0 is the held one.
    fn item(&self, _slot: u32) -> Option<(String, ItemInfo)> {
        None
    }
    /// The world's clock.
    fn clock(&self) -> Option<WorldClock> {
        None
    }
    /// A key by name: whether the game knows it, and whether it is held down.
    fn key(&self, _name: &str) -> (bool, bool) {
        (false, false)
    }
    /// Where mod `id` keeps files. Empty when the client offers nowhere.
    fn data_dir(&self, _id: &str) -> String {
        String::new()
    }
    /// One of mod `id`'s own files, read whole.
    fn asset(&self, _id: &str, _name: &str) -> Option<Vec<u8>> {
        None
    }
}

/// Writes `text` into a mod's buffer with a NUL, and answers with the bytes written.
unsafe fn write_text(text: &str, out: *mut c_char, len: usize) -> usize {
    if out.is_null() || len == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let take = bytes.len().min(len - 1);
    // SAFETY: the mod passed a buffer of `len` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), take);
        *out.add(take) = 0;
    }
    take
}

/// A mod's string argument, or "" when it passed nothing usable.
unsafe fn text_argument(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    // SAFETY: the mod passed a NUL-terminated string for this call.
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

/// A sound that started, for a mod that shapes how loud it plays.
#[derive(Debug, Clone, PartialEq)]
pub struct StartedInfo {
    /// Its handle, for [`Host::set_sound`].
    pub instance: u64,
    /// Its id (`minecraft:block.stone.break`).
    pub id: String,
    /// Where it plays; None for a relative sound.
    pub at: Option<[f64; 3]>,
}

/// What a loaded mod says about itself, for the client's Mods page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInfo {
    pub id: String,
    pub title: String,
    pub description: String,
}

/// Why a library was not loaded. Every one of these is a message, never a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The file is there but the system would not open it: not a library, wrong architecture, or
    /// a linker namespace that does not reach this folder.
    Dlopen(String),
    /// No `horizon_mod_entry` symbol: not a Horizon mod.
    NoEntry,
    /// It was built against a different boundary.
    Abi { built: u32, client: u32 },
    /// It came back with no id, so there is nothing to call it.
    Nameless,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("id", &self.id)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dlopen(e) => write!(f, "it could not be opened: {e}"),
            Self::NoEntry => write!(f, "it is not a Horizon mod: no horizon_mod_entry"),
            Self::Abi { built, client } => write!(
                f,
                "it was built for abi {built} and this client speaks abi {client}"
            ),
            Self::Nameless => write!(f, "it gave no id"),
        }
    }
}

/// A loaded mod: what it says about itself, and the functions it offers.
struct Loaded {
    id: String,
    title: String,
    description: String,
    path: PathBuf,
    desc: ModDesc,
    /// Whether `init` has been called, so a mod starts once and not once a frame.
    started: Cell<bool>,
    /// Held for the life of the process: dropping it would unmap code the client may still call.
    _lib: libloading::Library,
}

/// Every loaded mod, in the order their files were found.
static MODS: OnceLock<Mutex<Vec<Loaded>>> = OnceLock::new();

/// Whether the folder has been read, so a missing folder costs one `read_dir` and not one a frame.
static SCANNED: OnceLock<()> = OnceLock::new();

// The client, while one of a mod's callbacks is running. A trait object, so it is two words; this
// is the one place the client's `&mut` is turned into something a mod's C function can call back.
thread_local! {
    static CALLING: RefCell<Option<*mut (dyn Host + 'static)>> = const { RefCell::new(None) };
    static CALLING_ID: RefCell<String> = const { RefCell::new(String::new()) };
}

fn mods() -> &'static Mutex<Vec<Loaded>> {
    MODS.get_or_init(|| Mutex::new(Vec::new()))
}

impl Loaded {
    /// Opens `path` and reads what it offers, refusing anything the client cannot honour.
    ///
    /// # Safety
    /// `host` must stay valid for the life of the process, which this crate guarantees by keeping
    /// it in a `OnceLock` of its own.
    unsafe fn open(path: &Path, host: *const HostVTable) -> Result<Self, Refused> {
        // On Android a path cannot be opened at all: the linker's namespace reaches only the APK,
        // and the app's data folder has no execute permission. There the library is mapped from
        // memory instead, which is the one route Android allows; on a computer it is opened.
        #[cfg(target_os = "android")]
        let lib = android::open(path).map_err(Refused::Dlopen)?;
        // SAFETY: opening a library is unsafe because its initialisers may do anything; that risk
        // is the whole point of installing a mod, and it is a decision the player made.
        #[cfg(not(target_os = "android"))]
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| Refused::Dlopen(e.to_string()))?;
        let entry: libloading::Symbol<unsafe extern "C" fn(*const HostVTable) -> *const ModDesc> =
            unsafe { lib.get(horizon_mod_abi::ENTRY.as_bytes()) }.map_err(|_| Refused::NoEntry)?;
        // SAFETY: the signature is the one this boundary documents, and the mod was built against
        // it (checked just below, before anything else in it is used).
        let desc = unsafe { entry(host) };
        if desc.is_null() {
            return Err(Refused::NoEntry);
        }
        // SAFETY: the mod returned a pointer to its own static description.
        let desc = unsafe { *desc };
        if desc.abi != ABI {
            return Err(Refused::Abi {
                built: desc.abi,
                client: ABI,
            });
        }
        // SAFETY: the three strings are read here and copied; the mod keeps owning them.
        let (id, title, description) = unsafe {
            (
                horizon_mod_abi::text(desc.id),
                horizon_mod_abi::text(desc.title),
                horizon_mod_abi::text(desc.description),
            )
        };
        let Some(id) = id.filter(|id| !id.trim().is_empty()).map(str::to_owned) else {
            return Err(Refused::Nameless);
        };
        Ok(Self {
            title: title.unwrap_or(&id).to_owned(),
            description: description.unwrap_or("").to_owned(),
            id,
            path: path.to_owned(),
            started: Cell::new(false),
            desc,
            _lib: lib,
        })
    }

    fn info(&self) -> ModInfo {
        ModInfo {
            id: self.id.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
        }
    }

    /// Runs one of the mod's callbacks with `host` reachable from its C functions, logging a panic
    /// rather than letting it reach the frame.
    fn ran<R>(&self, host: &mut dyn Host, what: &str, run: impl FnOnce() -> R) -> Option<R> {
        let answer = with_host(host, &self.id, run);
        match answer {
            Ok(Some(value)) => Some(value),
            Ok(None) => None,
            Err(why) => {
                host.log(0, &format!("{} panicked in {what}: {why}", self.id));
                None
            }
        }
    }

    /// The mod's own setup, once.
    fn start(&self, host: &mut dyn Host) -> bool {
        let Some(init) = self.desc.init else {
            return true;
        };
        self.ran(host, "init", || {
            // SAFETY: init takes the table the client kept for the life of the process.
            unsafe { init(host_table()) }
        })
        .unwrap_or(false)
    }

    fn tick(&self, host: &mut dyn Host) {
        let Some(tick) = self.desc.tick else {
            return;
        };
        self.ran(host, "tick", || {
            // SAFETY: the mod's own callback, on the client's thread.
            unsafe { tick() }
        });
    }

    fn frame(&self, host: &mut dyn Host, partial: f32) {
        let Some(frame) = self.desc.frame else {
            return;
        };
        // A NaN would travel into the mod's maths and never come back out.
        let partial = if partial.is_finite() {
            partial.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.ran(host, "frame", || {
            // SAFETY: as above.
            unsafe { frame(partial) }
        });
    }

    fn settings(&self, host: &mut dyn Host, rows: &mut Vec<String>) {
        let Some(settings) = self.desc.settings else {
            return;
        };
        let sink = ButtonSink {
            ctx: rows as *mut Vec<String> as *mut c_void,
            push: push_button,
        };
        self.ran(host, "settings", || {
            // SAFETY: the sink outlives this call, and the mod only pushes through it.
            unsafe { settings(&sink) }
        });
    }

    fn settings_click(&self, host: &mut dyn Host, button: u32) {
        let Some(click) = self.desc.settings_click else {
            return;
        };
        self.ran(host, "settings_click", || {
            // SAFETY: the mod's own callback.
            unsafe { click(button) }
        });
    }

    fn camera_offset(&self, host: &mut dyn Host, partial: f32) -> Option<CameraOffset> {
        let callback = self.desc.camera_offset?;
        let mut offset = CameraOffset::default();
        self.ran(host, "camera_offset", || {
            // SAFETY: the pointer points to a local CameraOffset on the stack.
            unsafe { callback(partial, &mut offset) }
        })?;
        Some(offset)
    }
}

/// Runs `run` with `host` and `id` reachable from a mod's C functions.
///
/// The outer `Result` is the mod's panic; the inner one is a callback that produced nothing.
fn with_host<R>(
    host: &mut dyn Host,
    id: &str,
    run: impl FnOnce() -> R,
) -> Result<Option<R>, String> {
    // SAFETY: the lifetime is erased, not extended: the pointer is stored and cleared between two
    // statements of this function, and no mod callback outlives the call that runs it.
    let address: *mut (dyn Host + 'static) = unsafe {
        std::mem::transmute::<*mut dyn Host, *mut (dyn Host + 'static)>(host as *mut dyn Host)
    };
    CALLING.with(|c| *c.borrow_mut() = Some(address));
    CALLING_ID.with(|c| *c.borrow_mut() = id.to_owned());
    let out = guard(run);
    CALLING.with(|c| *c.borrow_mut() = None);
    CALLING_ID.with(|c| c.borrow_mut().clear());
    out.map(Some)
}

/// The client, while one of a mod's callbacks runs. None outside one.
fn calling<'a>() -> Option<&'a mut (dyn Host + 'static)> {
    let address = CALLING.with(|c| *c.borrow())?;
    // SAFETY: the pointer is the `&mut dyn Host` `with_host` set for the length of one call, on
    // this thread, and nothing else reaches it while that call runs.
    Some(unsafe { &mut *address })
}

fn calling_id() -> String {
    CALLING_ID.with(|c| c.borrow().clone())
}

/// The table the client offers a mod. Built once, and its address never changes.
fn table() -> *const HostVTable {
    static TABLE: OnceLock<HostVTable> = OnceLock::new();
    TABLE.get_or_init(|| HostVTable {
        abi: ABI,
        api_level: 7,
        log: host_log,
        setting: host_setting,
        set_setting: host_set_setting,
        listener: host_listener,
        block_name: host_block_name,
        face_is_sturdy: host_face_is_sturdy,
        is_water: host_is_water,
        raycast: host_raycast,
        sounds_started: host_sounds_started,
        set_sound: host_set_sound,
        player: host_player,
        item: host_item,
        clock: host_clock,
        key: host_key,
        data_dir: host_data_dir,
        asset: host_asset,
    }) as *const HostVTable
}

/// The table a mod is handed, for the call that has it.
fn host_table() -> *const HostVTable {
    table()
}

unsafe extern "C" fn host_log(level: u32, msg: *const c_char) {
    let Some(host) = calling() else {
        return;
    };
    if msg.is_null() {
        return;
    }
    // SAFETY: the mod promises a NUL-terminated string for the call.
    if let Ok(text) = unsafe { CStr::from_ptr(msg) }.to_str() {
        host.log(level, text);
    }
}

unsafe extern "C" fn host_setting(key: *const c_char, out: *mut c_char, len: usize) -> usize {
    if key.is_null() || out.is_null() || len == 0 {
        return 0;
    }
    let Some(host) = calling() else {
        return 0;
    };
    // SAFETY: the key is the mod's own buffer, valid for the call.
    let Ok(key) = (unsafe { CStr::from_ptr(key) }).to_str() else {
        return 0;
    };
    let Some(value) = host.setting(&calling_id(), key) else {
        return 0;
    };
    let bytes = value.as_bytes();
    let take = bytes.len().min(len - 1);
    // SAFETY: `take` is at most `len - 1`, so the NUL fits in the mod's buffer.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), take);
        *out.add(take) = 0;
    }
    take
}

unsafe extern "C" fn host_set_setting(key: *const c_char, value: *const c_char) {
    if key.is_null() {
        return;
    }
    let Some(host) = calling() else {
        return;
    };
    // SAFETY: both are the mod's own buffers, valid for the call.
    let (Ok(key), value) = (
        (unsafe { CStr::from_ptr(key) }).to_str(),
        if value.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("")
        },
    ) else {
        return;
    };
    let id = calling_id();
    host.set_setting(&id, key, value);
}

unsafe extern "C" fn host_listener(out: *mut [f64; 3]) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    let Some(at) = host.listener() else {
        return 0;
    };
    if !out.is_null() {
        // SAFETY: the mod passed a buffer for one `[f64; 3]`.
        unsafe { *out = at };
    }
    1
}

unsafe extern "C" fn host_block_name(
    x: i32,
    y: i32,
    z: i32,
    out: *mut c_char,
    len: usize,
) -> usize {
    let Some(host) = calling() else {
        return 0;
    };
    let Some(name) = host.block_name([x, y, z]) else {
        return 0;
    };
    if out.is_null() || len == 0 {
        return 0;
    }
    let bytes = name.as_bytes();
    let take = bytes.len().min(len - 1);
    // SAFETY: the mod passed a buffer of `len` bytes for the name.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), take);
        *out.add(take) = 0;
    }
    take
}

unsafe extern "C" fn host_face_is_sturdy(x: i32, y: i32, z: i32, side: u32) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    host.face_is_sturdy([x, y, z], side as usize) as u8
}

unsafe extern "C" fn host_is_water(x: i32, y: i32, z: i32) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    host.is_water([x, y, z]) as u8
}

unsafe extern "C" fn host_raycast(
    from: *const [f64; 3],
    to: *const [f64; 3],
    out: *mut RayHitOut,
) -> u8 {
    if from.is_null() || to.is_null() {
        return 0;
    }
    let Some(host) = calling() else {
        return 0;
    };
    // SAFETY: both are the mod's own points, valid for the call.
    let hit = host.raycast(unsafe { *from }, unsafe { *to });
    let Some(hit) = hit else {
        return 0;
    };
    if !out.is_null() {
        // SAFETY: the mod passed a buffer for one hit.
        unsafe { *out = hit };
    }
    1
}

unsafe extern "C" fn host_sounds_started(out: *mut StartedSound, max: usize) -> usize {
    let Some(host) = calling() else {
        return 0;
    };
    let started = host.sounds_started();
    if out.is_null() || max == 0 || started.is_empty() {
        return 0;
    }
    let take = started.len().min(max);
    thread_local! {
        static IDS: std::cell::RefCell<Vec<CString>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    IDS.with(|cell| {
        let mut ids = cell.borrow_mut();
        ids.clear();
        ids.extend(
            started[..take]
                .iter()
                .map(|s| CString::new(s.id.as_str()).unwrap_or_default()),
        );
        write_sounds(out, &started[..take], &ids);
    });
    take
}

fn write_sounds(out: *mut StartedSound, sounds: &[StartedInfo], ids: &[CString]) {
    for (i, sound) in sounds.iter().enumerate() {
        // SAFETY: the mod passed a buffer for `max` entries.
        unsafe {
            *out.add(i) = StartedSound {
                instance: sound.instance,
                id: ids[i].as_ptr(),
                has_at: sound.at.is_some() as u8,
                at: sound.at.unwrap_or([0.0; 3]),
            };
        }
    }
}

unsafe extern "C" fn host_set_sound(instance: u64, gain: f32, cutoff: f32, send: f32) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    host.set_sound(instance, gain, cutoff, send) as u8
}

unsafe extern "C" fn host_player(out: *mut PlayerInfo) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    let Some(player) = host.player() else {
        return 0;
    };
    if !out.is_null() {
        // SAFETY: the mod passed a buffer for one player.
        unsafe { *out = player };
    }
    1
}

unsafe extern "C" fn host_item(slot: u32, out: *mut c_char, len: usize, info: *mut ItemInfo) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    let Some((id, numbers)) = host.item(slot) else {
        return 0;
    };
    // SAFETY: both are the mod's own buffers, sized for what it asked for.
    unsafe {
        write_text(&id, out, len);
        if !info.is_null() {
            *info = numbers;
        }
    }
    1
}

unsafe extern "C" fn host_clock(out: *mut WorldClock) -> u8 {
    let Some(host) = calling() else {
        return 0;
    };
    let Some(clock) = host.clock() else {
        return 0;
    };
    if !out.is_null() {
        // SAFETY: the mod passed a buffer for one clock.
        unsafe { *out = clock };
    }
    1
}

unsafe extern "C" fn host_key(name: *const c_char) -> u32 {
    let Some(host) = calling() else {
        return 0;
    };
    let (known, down) = host.key(&unsafe { text_argument(name) });
    u32::from(known) | (u32::from(down) << 1)
}

unsafe extern "C" fn host_data_dir(out: *mut c_char, len: usize) -> usize {
    let Some(host) = calling() else {
        return 0;
    };
    // SAFETY: the mod passed a buffer of `len` bytes.
    unsafe { write_text(&host.data_dir(&calling_id()), out, len) }
}

unsafe extern "C" fn host_asset(name: *const c_char, out: *mut c_char, len: usize) -> isize {
    let Some(host) = calling() else {
        return 0;
    };
    let Some(bytes) = host.asset(&calling_id(), &unsafe { text_argument(name) }) else {
        return -1;
    };
    if bytes.len() > len {
        return -1;
    }
    if !out.is_null() && !bytes.is_empty() {
        // SAFETY: the mod passed a buffer it said is `len` bytes long, and this fits.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.cast::<u8>(), bytes.len()) };
    }
    bytes.len() as isize
}

/// A row a mod asked for, pushed into the sink the client passed.
unsafe extern "C" fn push_button(ctx: *mut c_void, text: *const c_char) {
    if ctx.is_null() || text.is_null() {
        return;
    }
    // SAFETY: `ctx` is the `Vec<String>` passed as a sink for this one call.
    let rows = unsafe { &mut *(ctx as *mut Vec<String>) };
    // SAFETY: as above.
    if let Ok(text) = unsafe { CStr::from_ptr(text) }.to_str() {
        rows.push(text.to_owned());
    }
}

/// Whether `name` looks like a mod library this client could load.
fn is_library(name: &str) -> bool {
    name.starts_with("lib") && name.ends_with(".so")
}

/// Every library directly inside `folder`, sorted by name so two runs agree.
fn libraries(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(is_library)
        })
        .collect();
    found.sort();
    found
}

/// Reads `folders` once: every mod library in them, and in each of their subfolders
/// (`<id>/lib*.so`), which is where a downloaded mod is unpacked.
///
/// A file that is refused is logged and left out: one broken mod must not cost the player the rest.
pub fn scan(folders: &[std::path::PathBuf], host: &mut dyn Host) {
    if SCANNED.set(()).is_err() {
        return;
    }
    let mut paths = Vec::new();
    for folder in folders {
        paths.extend(libraries(folder));
        for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                paths.extend(libraries(&path));
            }
        }
    }
    paths.sort();
    paths.dedup();
    let mut loaded: Vec<Loaded> = Vec::new();
    for path in paths {
        // SAFETY: the table is a `OnceLock`, so it outlives every mod.
        match unsafe { Loaded::open(&path, table()) } {
            Ok(module) => {
                if loaded.iter().any(|other| other.id == module.id) {
                    host.log(1, &format!("{}: {} is left out", module.id, path.display()));
                    continue;
                }
                host.log(
                    2,
                    &format!(
                        "loaded {} ({}) from {}",
                        module.id,
                        module.title,
                        path.display()
                    ),
                );
                loaded.push(module);
            }
            Err(why) => host.log(1, &format!("{} is not loaded: {why}", path.display())),
        }
    }
    mods()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .extend(loaded);
}

/// The loaded mods, for the client's Mods page.
pub fn list() -> Vec<ModInfo> {
    mods()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(Loaded::info)
        .collect()
}

/// Starts every mod that has not started yet: `init`, once each.
pub fn start(host: &mut dyn Host) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        if module.started.get() {
            continue;
        }
        module.started.set(true);
        if !module.start(host) {
            host.log(1, &format!("{} did not start", module.id));
        }
    }
}

/// Once a tick, in world.
pub fn tick(host: &mut dyn Host) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        module.tick(host);
    }
}

/// Once a frame.
pub fn frame(host: &mut dyn Host, partial: f32) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        module.frame(host, partial);
    }
}

/// The rows of `id`'s page in Options > Mods.
pub fn settings(host: &mut dyn Host, id: &str, rows: &mut Vec<String>) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter().filter(|m| m.id == id) {
        module.settings(host, rows);
    }
}

/// A tap on row `button` of `id`'s page.
pub fn settings_click(host: &mut dyn Host, id: &str, button: usize) {
    let button = u32::try_from(button).unwrap_or(u32::MAX);
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter().filter(|m| m.id == id) {
        module.settings_click(host, button);
    }
}

/// Query third-person camera offset from loaded mods. Returns the first active offset, or None.
pub fn camera_offset(host: &mut dyn Host, partial: f32) -> Option<CameraOffset> {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        if let Some(offset) = module.camera_offset(host, partial) {
            if offset.active != 0 {
                return Some(offset);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host that records what it was told, so a mod's calls can be read back.
    #[derive(Default)]
    struct Recorder {
        settings: std::cell::RefCell<std::collections::HashMap<(String, String), String>>,
    }

    impl Host for Recorder {
        fn listener(&self) -> Option<[f64; 3]> {
            Some([1.0, 2.0, 3.0])
        }

        fn block_name(&self, pos: [i32; 3]) -> Option<String> {
            match pos {
                [0, 0, 0] => Some("minecraft:stone".to_owned()),
                [1, 0, 0] => Some("minecraft:air".to_owned()),
                _ => None,
            }
        }

        fn face_is_sturdy(&self, pos: [i32; 3], side: usize) -> bool {
            pos == [0, 0, 0] && side == 3
        }

        fn is_water(&self, pos: [i32; 3]) -> bool {
            pos == [5, 5, 5]
        }

        fn raycast(&self, from: [f64; 3], to: [f64; 3]) -> Option<horizon_mod_abi::RayHitOut> {
            (from[0] < 0.5 && to[0] > 0.5).then_some(horizon_mod_abi::RayHitOut {
                at: [0.5, from[1], from[2]],
                block: [0, 0, 0],
                normal: [-1.0, 0.0, 0.0],
            })
        }

        fn sounds_started(&mut self) -> Vec<StartedInfo> {
            vec![StartedInfo {
                instance: 7,
                id: "minecraft:block.stone.break".to_owned(),
                at: Some([4.0, 5.0, 6.0]),
            }]
        }

        fn set_sound(&mut self, instance: u64, gain: f32, cutoff: f32, send: f32) -> bool {
            instance == 7 && gain < 1.0 && cutoff < 1.0 && send < 1.0
        }

        fn player(&self) -> Option<PlayerInfo> {
            Some(PlayerInfo {
                x: 1.5,
                y: 64.0,
                z: -2.5,
                vx: 0.1,
                vy: 0.0,
                vz: -0.1,
                yaw: 90.0,
                pitch: 10.0,
                health: 17.0,
                mode: 1,
                flags: 0b1010,
            })
        }

        fn item(&self, slot: u32) -> Option<(String, ItemInfo)> {
            (slot == 0).then(|| {
                (
                    "minecraft:diamond_sword".to_owned(),
                    ItemInfo {
                        count: 1,
                        damage: 12,
                        max_damage: 1561,
                        flags: 2,
                    },
                )
            })
        }

        fn clock(&self) -> Option<WorldClock> {
            Some(WorldClock {
                game_time: 400,
                day_time: 6000,
            })
        }

        fn key(&self, name: &str) -> (bool, bool) {
            (name == "key.attack", name == "key.attack")
        }

        fn data_dir(&self, id: &str) -> String {
            format!("/mods/{id}")
        }

        fn asset(&self, _id: &str, name: &str) -> Option<Vec<u8>> {
            (name == "hello.txt").then(|| b"hi".to_vec())
        }

        fn log(&self, _level: u32, _text: &str) {}

        fn setting(&self, id: &str, key: &str) -> Option<String> {
            self.settings
                .borrow()
                .get(&(id.to_owned(), key.to_owned()))
                .cloned()
        }

        fn set_setting(&mut self, id: &str, key: &str, value: &str) {
            self.settings
                .borrow_mut()
                .insert((id.to_owned(), key.to_owned()), value.to_owned());
        }
    }

    #[test]
    fn only_lib_star_dot_so_is_a_library() {
        assert!(is_library("libhello.so"));
        assert!(!is_library("hello.so"));
        assert!(!is_library("libhello.dylib"));
        assert!(!is_library("libhello.so.bak"));
    }

    #[test]
    fn a_missing_folder_has_no_libraries() {
        assert!(libraries(Path::new("/nonexistent/for/a/test")).is_empty());
    }

    #[test]
    fn a_refusal_says_what_is_wrong() {
        assert_eq!(
            Refused::Abi {
                built: 2,
                client: 1
            }
            .to_string(),
            "it was built for abi 2 and this client speaks abi 1"
        );
        assert_eq!(Refused::Nameless.to_string(), "it gave no id");
    }

    #[test]
    fn a_file_that_is_not_a_library_is_refused_not_fatal() {
        let dir = std::env::temp_dir().join("horizon-mod-loader-not-a-library");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("libjunk.so");
        std::fs::write(&path, b"not an ELF file").unwrap();
        let got = std::panic::catch_unwind(|| {
            // SAFETY: a null table is never dereferenced, because the library does not load.
            unsafe { Loaded::open(&path, std::ptr::null()) }
        })
        .unwrap();
        assert!(matches!(got, Err(Refused::Dlopen(_))), "{got:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_host_that_is_not_calling_answers_nothing() {
        assert!(calling().is_none());
        assert!(calling_id().is_empty());
    }

    #[test]
    fn the_world_and_the_sounds_cross_the_boundary() {
        let mut host = Recorder::default();
        let (listener, name, sturdy, water, hit, started, applied) =
            with_host(&mut host, "world", || unsafe {
                let mut eye = [0.0; 3];
                let has_eye = host_listener(&mut eye) == 1;
                let mut buffer = [0 as c_char; horizon_mod_abi::TEXT];
                let len = host_block_name(0, 0, 0, buffer.as_mut_ptr(), buffer.len());
                let name = if len == 0 {
                    String::new()
                } else {
                    CStr::from_ptr(buffer.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                };
                let mut hit = RayHitOut::default();
                let mut sounds = [StartedSound {
                    instance: 0,
                    id: std::ptr::null(),
                    has_at: 0,
                    at: [0.0; 3],
                }; 4];
                let count = host_sounds_started(sounds.as_mut_ptr(), sounds.len());
                let first = if count == 0 {
                    (String::new(), None)
                } else {
                    (
                        CStr::from_ptr(sounds[0].id).to_string_lossy().into_owned(),
                        (sounds[0].has_at == 1).then_some(sounds[0].at),
                    )
                };
                (
                    (has_eye, eye),
                    name,
                    host_face_is_sturdy(0, 0, 0, 3) == 1,
                    host_is_water(5, 5, 5) == 1,
                    (host_raycast(&[0.0, 1.0, 1.0], &[1.0, 1.0, 1.0], &mut hit) == 1)
                        .then_some(hit),
                    (count, first),
                    host_set_sound(7, 0.4, 0.7, 0.9) == 1,
                )
            })
            .unwrap()
            .unwrap();
        assert_eq!(listener, (true, [1.0, 2.0, 3.0]));
        assert_eq!(name, "minecraft:stone");
        assert!(sturdy && water && applied);
        assert_eq!(hit.unwrap().normal, [-1.0, 0.0, 0.0]);
        assert_eq!(started.0, 1);
        assert_eq!(started.1.0, "minecraft:block.stone.break");
        assert_eq!(started.1.1, Some([4.0, 5.0, 6.0]));
    }

    #[test]
    fn the_player_and_their_body_cross_the_boundary() {
        let mut host = Recorder::default();
        let (player, item, clock, key, dir, asset) = with_host(&mut host, "world", || unsafe {
            let mut player = PlayerInfo::default();
            let has_player = host_player(&mut player) == 1;
            let mut name = [0 as c_char; horizon_mod_abi::TEXT];
            let mut info = ItemInfo::default();
            let has_item = host_item(0, name.as_mut_ptr(), name.len(), &mut info) == 1;
            let held = CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned();
            let mut clock = WorldClock::default();
            let has_clock = host_clock(&mut clock) == 1;
            let mut dir = [0 as c_char; horizon_mod_abi::TEXT];
            let dir_len = host_data_dir(dir.as_mut_ptr(), dir.len());
            let mut file = [0 as c_char; 8];
            let read = host_asset(c"hello.txt".as_ptr(), file.as_mut_ptr(), file.len());
            let missing = host_asset(c"nope.txt".as_ptr(), file.as_mut_ptr(), file.len());
            (
                (has_player, player),
                (has_item, held, info),
                (has_clock, clock),
                host_key(c"key.attack".as_ptr()),
                CStr::from_ptr(dir.as_ptr()).to_string_lossy().into_owned(),
                (
                    read,
                    if read >= 0 {
                        String::from_utf8_lossy(&file.map(|c| c as u8)[..read as usize])
                            .into_owned()
                    } else {
                        String::new()
                    },
                    missing,
                    std::ptr::addr_of!(dir_len).read(),
                ),
            )
        })
        .unwrap()
        .unwrap();
        assert!(player.0);
        assert_eq!(player.1.y, 64.0);
        assert!(player.1.on_ground() && player.1.sneaking() && !player.1.in_water());
        assert!(item.0);
        assert_eq!(item.1, "minecraft:diamond_sword");
        assert_eq!(item.2.max_damage, 1561);
        assert_eq!(clock.1.day_time, 6000);
        assert_eq!(key, 3);
        assert_eq!(dir, "/mods/world");
        assert_eq!(asset.1, "hi");
        assert_eq!(asset.2, -1);
    }

    #[test]
    fn with_host_sets_and_clears_what_a_mod_sees() {
        let mut host = Recorder {
            settings: std::cell::RefCell::new(
                [(("hello".to_owned(), "frames".to_owned()), "42".to_owned())]
                    .into_iter()
                    .collect(),
            ),
        };
        let seen = with_host(&mut host, "hello", || (calling_id(), calling().is_some())).unwrap();
        assert_eq!(seen, Some(("hello".to_owned(), true)));
        // After the call, a mod's stray callback finds nothing rather than a dangling host.
        assert!(calling().is_none());
        assert!(calling_id().is_empty());
    }
}

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

use horizon_mod_abi::{ABI, ButtonSink, HostVTable, ModDesc, guard};
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_char, c_void};
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
    fn set_setting(&self, id: &str, key: &str, value: &str);
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
    static CALLING: RefCell<Option<*const (dyn Host + 'static)>> = const { RefCell::new(None) };
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
        // SAFETY: opening a library is unsafe because its initialisers may do anything; that risk
        // is the whole point of installing a mod, and it is a decision the player made.
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
    fn ran<R>(&self, host: &dyn Host, what: &str, run: impl FnOnce() -> R) -> Option<R> {
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
    fn start(&self, host: &dyn Host) -> bool {
        let Some(init) = self.desc.init else {
            return true;
        };
        self.ran(host, "init", || {
            // SAFETY: init takes the table the client kept for the life of the process.
            unsafe { init(host_table()) }
        })
        .unwrap_or(false)
    }

    fn tick(&self, host: &dyn Host) {
        let Some(tick) = self.desc.tick else {
            return;
        };
        self.ran(host, "tick", || {
            // SAFETY: the mod's own callback, on the client's thread.
            unsafe { tick() }
        });
    }

    fn frame(&self, host: &dyn Host, partial: f32) {
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

    fn settings(&self, host: &dyn Host, rows: &mut Vec<String>) {
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

    fn settings_click(&self, host: &dyn Host, button: u32) {
        let Some(click) = self.desc.settings_click else {
            return;
        };
        self.ran(host, "settings_click", || {
            // SAFETY: the mod's own callback.
            unsafe { click(button) }
        });
    }
}

/// Runs `run` with `host` and `id` reachable from a mod's C functions.
///
/// The outer `Result` is the mod's panic; the inner one is a callback that produced nothing.
fn with_host<R>(host: &dyn Host, id: &str, run: impl FnOnce() -> R) -> Result<Option<R>, String> {
    // SAFETY: the lifetime is erased, not extended: the pointer is stored and cleared between two
    // statements of this function, and no mod callback outlives the call that runs it.
    let address: *const (dyn Host + 'static) = unsafe {
        std::mem::transmute::<*const dyn Host, *const (dyn Host + 'static)>(host as *const dyn Host)
    };
    CALLING.with(|c| *c.borrow_mut() = Some(address));
    CALLING_ID.with(|c| *c.borrow_mut() = id.to_owned());
    let out = guard(run);
    CALLING.with(|c| *c.borrow_mut() = None);
    CALLING_ID.with(|c| c.borrow_mut().clear());
    out.map(Some)
}

/// The client, while one of a mod's callbacks runs. None outside one.
fn calling<'a>() -> Option<&'a (dyn Host + 'static)> {
    let address = CALLING.with(|c| *c.borrow())?;
    // SAFETY: the pointer is the `&dyn Host` `with_host` set for the length of one call, on this
    // thread, and nothing else reaches it while that call runs.
    Some(unsafe { &*address })
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
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, take);
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

/// Reads `folder` once: every mod library in it, and in each of its subfolders (`<id>/lib*.so`),
/// which is where a downloaded mod is unpacked.
///
/// A file that is refused is logged and left out: one broken mod must not cost the player the rest.
pub fn scan(folder: &Path, host: &dyn Host) {
    if SCANNED.set(()).is_err() {
        return;
    }
    let mut paths = libraries(folder);
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            paths.extend(libraries(&path));
        }
    }
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
pub fn start(host: &dyn Host) {
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
pub fn tick(host: &dyn Host) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        module.tick(host);
    }
}

/// Once a frame.
pub fn frame(host: &dyn Host, partial: f32) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter() {
        module.frame(host, partial);
    }
}

/// The rows of `id`'s page in Options > Mods.
pub fn settings(host: &dyn Host, id: &str, rows: &mut Vec<String>) {
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter().filter(|m| m.id == id) {
        module.settings(host, rows);
    }
}

/// A tap on row `button` of `id`'s page.
pub fn settings_click(host: &dyn Host, id: &str, button: usize) {
    let button = u32::try_from(button).unwrap_or(u32::MAX);
    let guard_mods = mods().lock().unwrap_or_else(|e| e.into_inner());
    for module in guard_mods.iter().filter(|m| m.id == id) {
        module.settings_click(host, button);
    }
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
        fn log(&self, _level: u32, _text: &str) {}

        fn setting(&self, id: &str, key: &str) -> Option<String> {
            self.settings
                .borrow()
                .get(&(id.to_owned(), key.to_owned()))
                .cloned()
        }

        fn set_setting(&self, id: &str, key: &str, value: &str) {
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
    fn with_host_sets_and_clears_what_a_mod_sees() {
        let host = Recorder {
            settings: std::cell::RefCell::new(
                [(("hello".to_owned(), "frames".to_owned()), "42".to_owned())]
                    .into_iter()
                    .collect(),
            ),
        };
        let seen = with_host(&host, "hello", || (calling_id(), calling().is_some())).unwrap();
        assert_eq!(seen, Some(("hello".to_owned(), true)));
        // After the call, a mod's stray callback finds nothing rather than a dangling host.
        assert!(calling().is_none());
        assert!(calling_id().is_empty());
    }
}

//! Loading a mod's library on Android, where the file system cannot hold it.
//!
//! `dlopen` by path does not work for a mod. Two separate rules stop it:
//!
//! * the linker's namespace for an app reaches only the APK's own `lib/` folder, so a path under
//!   `/sdcard` or the app's data folder answers `is not accessible for the namespace "clns-10"`;
//! * since Android 10 an app's data folder has no execute permission at all (SELinux), so even a
//!   path the namespace allowed would be refused there.
//!
//! What is allowed is loading from memory, which is a documented NDK API: a `memfd` is an anonymous
//! file with no path and no label to block, and `android_dlopen_ext` with
//! `ANDROID_DLEXT_USE_LIBRARY_FD` maps it. AOSP's linker maintainer, on android/ndk#168: "it's
//! expected that you can load anything you want from RAM — there's a public API for that".
//!
//! So a mod is read into a `memfd` and mapped from there. The file on disk is only ever read, which
//! is all a player's folder ever needed to allow.

use std::ffi::{CString, c_char, c_void};
use std::path::Path;

/// `android_dlextinfo` from `<android/dlext.h>`, in the platform's own field order.
#[repr(C)]
struct Dlextinfo {
    flags: u64,
    reserved_addr: *mut c_void,
    reserved_size: usize,
    relro_fd: i32,
    library_fd: i32,
    library_fd_offset: i64,
    library_namespace: *mut c_void,
}

/// `ANDROID_DLEXT_USE_LIBRARY_FD`: map the library from `library_fd`, not from a path.
const ANDROID_DLEXT_USE_LIBRARY_FD: u64 = 0x10;

// `memfd_create` has no bionic wrapper before API 30, so the syscall is called by number. The
// numbers are the kernel's, per architecture, and do not change.
#[cfg(target_arch = "aarch64")]
const SYS_MEMFD_CREATE: libc::c_long = 279;
#[cfg(target_arch = "arm")]
const SYS_MEMFD_CREATE: libc::c_long = 385;
#[cfg(target_arch = "x86_64")]
const SYS_MEMFD_CREATE: libc::c_long = 319;
#[cfg(target_arch = "x86")]
const SYS_MEMFD_CREATE: libc::c_long = 356;

type Dlopenext = unsafe extern "C" fn(*const c_char, i32, *const Dlextinfo) -> *mut c_void;

/// The last word from the dynamic linker, for a message.
fn last_error() -> String {
    // SAFETY: a null return means dlerror has something to say.
    unsafe {
        let message = libc::dlerror();
        if message.is_null() {
            return "the dynamic linker gave no reason".to_owned();
        }
        std::ffi::CStr::from_ptr(message)
            .to_str()
            .unwrap_or("unreadable")
            .to_owned()
    }
}

/// Reads `path` into a `memfd` and maps it, which is the only way Android lets an app run code it
/// downloaded.
pub(super) fn open(path: &Path) -> Result<libloading::Library, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("it could not be read: {e}"))?;
    let name = CString::new("horizon-mod").map_err(|e| e.to_string())?;
    // SAFETY: a plain syscall with a valid name pointer and no flags.
    let fd = unsafe { libc::syscall(SYS_MEMFD_CREATE, name.as_ptr(), 0u32) } as i32;
    if fd < 0 {
        return Err(format!(
            "no memory file: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut written = 0;
    while written < bytes.len() {
        // SAFETY: the slice and the count are inside `bytes`, and `fd` is the memory file above.
        let n = unsafe {
            libc::write(
                fd,
                bytes[written..].as_ptr() as *const c_void,
                bytes.len() - written,
            )
        };
        if n <= 0 {
            // SAFETY: `fd` is open and owned here.
            unsafe { libc::close(fd) };
            return Err(format!(
                "the memory file would not take it: {}",
                std::io::Error::last_os_error()
            ));
        }
        written += n as usize;
    }

    // `android_dlopen_ext` lives in libdl, which every process has; the handle is leaked so the
    // symbol stays valid for as long as a mod can be called.
    // SAFETY: opening a system library by name, and looking one documented symbol up in it.
    let dl = unsafe {
        let lib =
            libloading::os::unix::Library::new("libdl.so").map_err(|e| format!("no libdl: {e}"))?;
        Box::leak(Box::new(lib))
    };
    // SAFETY: the signature is `<android/dlext.h>`'s.
    let dlopen_ext: libloading::os::unix::Symbol<Dlopenext> =
        unsafe { dl.get(b"android_dlopen_ext\0") }
            .map_err(|e| format!("no android_dlopen_ext: {e}"))?;

    let soname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("libmod.so")
        .to_owned();
    let soname = CString::new(soname).map_err(|e| e.to_string())?;
    let info = Dlextinfo {
        flags: ANDROID_DLEXT_USE_LIBRARY_FD,
        reserved_addr: std::ptr::null_mut(),
        reserved_size: 0,
        relro_fd: -1,
        library_fd: fd,
        library_fd_offset: 0,
        library_namespace: std::ptr::null_mut(),
    };
    // SAFETY: the soname, the fd and the info all outlive the call.
    let handle = unsafe { dlopen_ext(soname.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL, &info) };
    if handle.is_null() {
        let why = last_error();
        // SAFETY: `fd` is open and owned here.
        unsafe { libc::close(fd) };
        return Err(why);
    }
    // The mappings keep the memory file alive, so the fd can go.
    // SAFETY: `fd` is open and owned here.
    unsafe { libc::close(fd) };
    // SAFETY: the linker's handle is a live `dlopen` handle, which is what `from_raw` takes.
    Ok(libloading::Library::from(unsafe {
        libloading::os::unix::Library::from_raw(handle)
    }))
}

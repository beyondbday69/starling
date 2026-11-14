# The mod loader: mods as libraries, not client patches

Today every native mod is source that is spliced into this client and compiled with it.
That costs a 20-minute build on a server, a 73 MB `.so` per player, and a recompile of the
whole client for every change to a mod. It also means a player can only run mods the build
server agrees to build.

This document is the plan for the alternative: a mod is a shared library the client loads at
runtime. Nothing in the client is patched. A mod is written once, compiled once (by its
author, on any machine or in CI), and downloaded by players as a small `.so`.

## Status: Phase A is written and runs on a computer

`horizon-mod-abi`, `horizon-mod-loader` and `horizon-mod-hello` are in, with 10 tests, and the
client's four hooks (`native_mods`, `native_frame`, `native_tick`, `native_settings[+_click]`) call
them. `cargo run -p horizon-mod-loader --example load_hello -- target/debug` builds the hello mod,
opens it, starts it, runs two frames, reads its three rows and taps one.

The one thing still unchecked is the fact Phase A exists to check: that an Android app may
`dlopen` a library from its own files folder. `.github/workflows/mod-loader.yml` builds
`libhello.so` for arm64 and publishes it, so that is one install away from an answer.

## What exists, and what does not

| Piece | State |
|---|---|
| `Hooks` trait, 506 methods, `api_level` 7 with 46 versioned functions | exists, works |
| `dispatch_*` functions, 529 of them | exists, works |
| `NativeMod` (id, title, description, keys) in `native_hooks.rs` | exists, works |
| Source-injection build service | exists, but needs a 20-minute server build |
| WebAssembly mods | **removed**: `horizon-mods/src/index/native.rs:8`, `WASM_GONE` |
| Loading a mod from a `.so` at runtime | **does not exist** |
| `libloading 0.8.9` in the workspace | exists (`horizon-narrator`), so `dlopen` is already proven on Android |
| JNI `#[unsafe(no_mangle)]` surface | exists (`crates/horizon-client/src/android_lib.rs`) |

So the missing half is small and the half it plugs into is already 500 hooks wide.

## The shape

```
horizon-mod-loader  (new crate, ~600 lines)
    dlopen("…/horizon-mods/<id>/lib<id>.so", RTLD_NOW | RTLD_LOCAL)
    dlsym("horizon_mod_entry") -> *const ModDesc
    abi check  ->  wrap as NativeMod + Hooks impl  ->  existing dispatch

horizon-mod-abi     (new crate, ~400 lines, no dependencies)
    #[repr(C)] HostVTable  — what the client offers a mod
    #[repr(C)] ModDesc     — what a mod offers the client
    both are plain function pointers and C integers, nothing else
```

Nothing crosses the boundary that Rust owns: no `String`, no `Vec`, no `&str`, no trait
object, no allocator. Buffers are `(ptr, len)` into caller-owned memory. This is what makes
it safe for a mod to carry its own copy of `std` — which it will, and which is fine.

## Phase A — prove it end to end (this is the first thing to build)

The whole design rests on one fact that has not been checked on this device: that Android
lets this app `dlopen` a library from its own files directory. Tinker and AndFix do exactly
this, but it is cheap to confirm before writing 1,000 lines.

**Deliverable:** a mod that adds a row to Options > Mods and writes one line to the log.

1. `crates/horizon-mod-abi/src/lib.rs`
   ```rust
   pub const ABI: u32 = 1;

   #[repr(C)]
   pub struct HostVTable {
       pub abi: u32,
       pub api_level: u32,              // matches horizon_mods::api_level::LEVEL
       pub log:    unsafe extern "C" fn(*const c_char),
       pub setting: unsafe extern "C" fn(*const c_char, *const c_char, *mut c_char, usize) -> usize,
       pub set_setting: unsafe extern "C" fn(*const c_char, *const c_char),
   }

   #[repr(C)]
   pub struct ModDesc {
       pub abi: u32,
       pub id: *const c_char,
       pub title: *const c_char,
       pub description: *const c_char,
       pub init: Option<unsafe extern "C" fn(*const HostVTable) -> bool>,
       pub tick: Option<unsafe extern "C" fn()>,
       pub frame: Option<unsafe extern "C" fn(f32)>,
   }
   ```

2. `crates/horizon-mod-loader/src/lib.rs`
   - `Loader::open(path) -> Result<Loaded, LoadError>`
   - checks `desc.abi == ABI`; a mismatch is a refusal with a message, never a crash
   - keeps the `Library` alive in a process-lifetime list: a renderer mod can never be
     unloaded anyway (`RenderHooks` is a `OnceLock`), so unloading is not attempted
   - every call from the client into a mod goes through `catch_unwind`; the existing
     two-strike guard (`files/native/starts`) already covers a mod that dies before its
     first frame

3. `crates/horizon-client/src/game/mod_loader.rs`
   - scans `horizon-mods/*/lib*.so` next to the `.hnmod` files
   - registers each as a `NativeMod`, so Options > Mods lists it with no new UI
   - forwards `frame`/`tick` from `native_frame`/`native_tick`

4. `native-mods/hello` — the test mod, in this repository, built by the same workflow that
   builds the client, so the first `.so` exists the moment the loader does.

**Pass condition:** install, drop `libhello.so` in `horizon-mods/hello/`, start the client,
see "Hello" in Options > Mods and one line in `logcat`. No client rebuild after that first
one.

## Phase B — the host surface a real mod needs

Phase A proves the mechanism with 3 host calls. Phase B grows the vtable to what Hold My
Items and Grassier Grass actually use, measured from their own sources rather than guessed:

- player: position, rotation, eye position, held item, game mode
- world: block at, block facts (`StateFacts.sturdy_full`), height, dimension
- sound: `play`, and the `sound_play` hook's `(volume, pitch, cancel)` out-parameters
- render: no. A renderer needs `Vertex` buffers and `RenderHooks`; that is a separate ABI
  and a separate decision, because it is the one place a bad mod can crash the frame loop
  outright.
- settings: read, write, and the label/click pair so a mod gets its own rows

**Pass condition:** Hold My Items and Grassier Grass recompiled against the ABI produce the
same behaviour as their injected builds, on the same commit.

## Phase C — distribution

- a mod's `.so` goes in its GitHub release; the catalog (or a plain download) puts it in
  `horizon-mods/<id>/`
- `horizon-mod.json` next to it: id, title, version, abi, api_level
- abi mismatch, wrong arch, corrupt file: refused with a message, client unaffected
- no build server in the loop at all

## Risks, in the order they would hurt

1. **`dlopen` from app storage is refused.** Fallback: extract to a directory the linker
   namespace allows, or `android_dlopen_ext` with a private namespace (API 26+). Phase A
   exists to find this out on day one, before the rest is written.
2. **Two copies of `std`.** Harmless with a C-only boundary; fatal if a `Vec` or `String` is
   ever passed across it. The rule is mechanical and reviewable, and Phase A's 3 calls show
   the pattern.
3. **A mod that hangs.** A panic is caught; an infinite loop in `frame` inside a mod that
   only has `tick` is not. Mods get no thread of their own, so a hang stalls a frame — the
   existing watchdog covers it the same way it covers an injected mod today.
4. **Renderer mods stay unloadable.** Unchanged from today, and the reason render hooks are
   left out of Phase A/B.

## What this does not do

It does not compile Rust on the phone. `rustc` has no Android host target; the client will
never build a mod. What it removes is the need to: a mod is compiled once by whoever wrote
it, and every player after that only downloads a file.

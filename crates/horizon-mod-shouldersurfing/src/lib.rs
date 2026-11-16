//! Shoulder Surfing as a runtime library mod for Starling/Horizon.
//!
//! Provides a modern over-the-shoulder third-person camera perspective with:
//! - Configurable shoulder side (Right / Left / Center)
//! - Configurable distance and vertical height
//! - Smooth camera transitions and interpolation
//! - Dynamic block collision avoidance using raycasting to prevent clipping into walls
//! - Persistence across game restarts via client settings
//! - In-game configuration UI in Options > Mods

use horizon_mod_abi::{ABI, ButtonSink, CameraOffset, HostVTable, ModDesc, guard};
use std::ffi::{CStr, CString, c_char};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The client's host table once initialized.
static HOST: AtomicPtr<HostVTable> = AtomicPtr::new(std::ptr::null_mut());

/// Statistics for the settings screen.
static FRAMES_COUNT: AtomicUsize = AtomicUsize::new(0);
static COLLISIONS_CLAMPED: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShoulderSide {
    Right,
    Left,
    Center,
}

impl ShoulderSide {
    pub fn offset_x(self) -> f32 {
        match self {
            Self::Right => 0.75,
            Self::Left => -0.75,
            Self::Center => 0.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Center,
            Self::Center => Self::Right,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Right => "Right",
            Self::Left => "Left",
            Self::Center => "Center",
        }
    }
}

/// User configurable settings.
#[derive(Clone, Debug)]
pub struct Settings {
    pub enabled: bool,
    pub side: ShoulderSide,
    pub height: f32,
    pub distance: f32,
    pub smooth: bool,
    pub dynamic_raycast: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            side: ShoulderSide::Right,
            height: 0.2,
            distance: 4.0,
            smooth: true,
            dynamic_raycast: true,
        }
    }
}

fn settings() -> &'static Mutex<Settings> {
    static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();
    SETTINGS.get_or_init(|| Mutex::new(Settings::default()))
}

/// Interpolated runtime state.
struct CameraState {
    current_right: f32,
    current_up: f32,
    current_distance: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            current_right: 0.75,
            current_up: 0.2,
            current_distance: 4.0,
        }
    }
}

fn camera_state() -> &'static Mutex<CameraState> {
    static STATE: OnceLock<Mutex<CameraState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(CameraState::default()))
}

fn host() -> Option<&'static HostVTable> {
    let ptr = HOST.load(Ordering::Relaxed);
    if ptr.is_null() {
        None
    } else {
        // SAFETY: host table is maintained by the client for the lifetime of the process.
        Some(unsafe { &*ptr })
    }
}

fn log(level: u32, text: &str) {
    let Some(host) = host() else {
        return;
    };
    if let Ok(c_text) = CString::new(text) {
        unsafe { (host.log)(level, c_text.as_ptr()) };
    }
}

fn read_setting(key: &str) -> Option<String> {
    let host = host()?;
    let key_c = CString::new(key).ok()?;
    let mut buf = [0 as c_char; 128];
    let len = unsafe { (host.setting)(key_c.as_ptr(), buf.as_mut_ptr(), buf.len()) };
    if len == 0 {
        return None;
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_str()
        .ok()
        .map(|s| s.trim().to_owned())
}

fn write_setting(key: &str, value: &str) {
    let Some(host) = host() else {
        return;
    };
    if let (Ok(k), Ok(v)) = (CString::new(key), CString::new(value)) {
        unsafe { (host.set_setting)(k.as_ptr(), v.as_ptr()) };
    }
}

fn load_settings_from_host() {
    let mut cfg = settings().lock().expect("settings");
    if let Some(val) = read_setting("enabled") {
        cfg.enabled = val == "true" || val == "1";
    }
    if let Some(val) = read_setting("side") {
        cfg.side = match val.as_str() {
            "Left" | "left" => ShoulderSide::Left,
            "Center" | "center" => ShoulderSide::Center,
            _ => ShoulderSide::Right,
        };
    }
    if let Some(val) = read_setting("height") {
        if let Ok(num) = val.parse::<f32>() {
            cfg.height = num;
        }
    }
    if let Some(val) = read_setting("distance") {
        if let Ok(num) = val.parse::<f32>() {
            cfg.distance = num;
        }
    }
    if let Some(val) = read_setting("smooth") {
        cfg.smooth = val != "false" && val != "0";
    }
    if let Some(val) = read_setting("dynamic_raycast") {
        cfg.dynamic_raycast = val != "false" && val != "0";
    }
}

fn save_settings_to_host() {
    let cfg = settings().lock().expect("settings").clone();
    write_setting("enabled", if cfg.enabled { "true" } else { "false" });
    write_setting("side", cfg.side.as_str());
    write_setting("height", &format!("{:.2}", cfg.height));
    write_setting("distance", &format!("{:.2}", cfg.distance));
    write_setting("smooth", if cfg.smooth { "true" } else { "false" });
    write_setting(
        "dynamic_raycast",
        if cfg.dynamic_raycast { "true" } else { "false" },
    );
}

unsafe fn push_row(sink: *const ButtonSink, text: &str) {
    let Some(sink) = (unsafe { sink.as_ref() }) else {
        return;
    };
    if let Ok(c_text) = CString::new(text) {
        unsafe { (sink.push)(sink.ctx, c_text.as_ptr()) };
    }
}

unsafe extern "C" fn init(table: *const HostVTable) -> bool {
    guard(|| {
        if table.is_null() {
            return false;
        }
        let t = unsafe { &*table };
        if t.abi != ABI {
            return false;
        }
        HOST.store(table.cast_mut(), Ordering::Relaxed);
        load_settings_from_host();
        log(2, "shoulder surfing: initialized successfully, abi matches");
        true
    })
    .unwrap_or(false)
}

unsafe extern "C" fn frame(partial: f32) {
    let _ = guard(|| {
        FRAMES_COUNT.fetch_add(1, Ordering::Relaxed);
        let cfg = settings().lock().expect("settings").clone();
        if !cfg.enabled {
            return;
        }
        let mut state = camera_state().lock().expect("camera_state");
        let target_right = cfg.side.offset_x();
        let target_up = cfg.height;
        let target_dist = cfg.distance;

        if cfg.smooth {
            let speed = (partial * 0.35).clamp(0.05, 1.0);
            state.current_right += (target_right - state.current_right) * speed;
            state.current_up += (target_up - state.current_up) * speed;
            state.current_distance += (target_dist - state.current_distance) * speed;
        } else {
            state.current_right = target_right;
            state.current_up = target_up;
            state.current_distance = target_dist;
        }
    });
}

unsafe extern "C" fn camera_offset(_partial: f32, out: *mut CameraOffset) {
    let _ = guard(|| {
        if out.is_null() {
            return;
        }
        let cfg = settings().lock().expect("settings").clone();
        if !cfg.enabled {
            unsafe {
                (*out).active = 0;
            }
            return;
        }

        let state = camera_state().lock().expect("camera_state");
        let mut right = state.current_right;
        let up = state.current_up;
        let dist = state.current_distance;

        // Wall collision check via host raycast if enabled
        if cfg.dynamic_raycast && right.abs() > 0.05 {
            if let Some(host) = host() {
                let mut eye = [0.0; 3];
                if unsafe { (host.listener)(&mut eye) } == 1 {
                    // Check lateral ray along yaw from eye to shoulder offset
                    if let Some(player) = {
                        let mut p = horizon_mod_abi::PlayerInfo::default();
                        if unsafe { (host.player)(&mut p) } == 1 {
                            Some(p)
                        } else {
                            None
                        }
                    } {
                        let yaw_rad = f64::from(player.yaw).to_radians();
                        // Local right vector in Minecraft horizontal coordinates:
                        // forward = (-sin(yaw), 0, cos(yaw))
                        // right = (cos(yaw), 0, sin(yaw))
                        let cos_y = yaw_rad.cos();
                        let sin_y = yaw_rad.sin();
                        let shoulder_x = eye[0] + cos_y * f64::from(right);
                        let shoulder_y = eye[1] + f64::from(up);
                        let shoulder_z = eye[2] + sin_y * f64::from(right);
                        let target = [shoulder_x, shoulder_y, shoulder_z];

                        let mut hit = horizon_mod_abi::RayHitOut::default();
                        if unsafe { (host.raycast)(&eye, &target, &mut hit) } == 1 {
                            // Hit a block: clamp shoulder distance
                            let dx = hit.at[0] - eye[0];
                            let dz = hit.at[2] - eye[2];
                            let hit_dist = (dx * dx + dz * dz).sqrt() as f32;
                            let max_safe = (hit_dist - 0.15).max(0.0);
                            if max_safe < right.abs() {
                                right = right.signum() * max_safe;
                                COLLISIONS_CLAMPED.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
        }

        unsafe {
            *out = CameraOffset {
                active: 1,
                _pad: [0; 3],
                right,
                up,
                forward: 0.0,
                distance: dist,
            };
        }
    });
}

unsafe extern "C" fn settings_rows(sink: *const ButtonSink) {
    let _ = guard(|| {
        let cfg = settings().lock().expect("settings").clone();
        let state = camera_state().lock().expect("camera_state");
        let frames = FRAMES_COUNT.load(Ordering::Relaxed);
        let clamped = COLLISIONS_CLAMPED.load(Ordering::Relaxed);

        let rows = [
            format!(
                "Shoulder Surfing: {}",
                if cfg.enabled { "Enabled" } else { "Disabled" }
            ),
            format!(
                "Shoulder Side: {} ({:+.2})",
                cfg.side.as_str(),
                cfg.side.offset_x()
            ),
            format!("Camera Distance: {:.1} blocks", cfg.distance),
            format!("Camera Height: {:+.1} blocks", cfg.height),
            format!(
                "Smooth Interpolation: {}",
                if cfg.smooth { "Enabled" } else { "Off" }
            ),
            format!(
                "Wall Collision Raycast: {}",
                if cfg.dynamic_raycast {
                    "Enabled"
                } else {
                    "Off"
                }
            ),
            format!(
                "Active: R={:+.2} U={:+.2} D={:.2} ({} hits, {} frames)",
                state.current_right, state.current_up, state.current_distance, clamped, frames
            ),
        ];

        for row in &rows {
            unsafe { push_row(sink, row) };
        }
    });
}

unsafe extern "C" fn settings_click(button: u32) {
    let _ = guard(|| {
        {
            let mut cfg = settings().lock().expect("settings");
            match button {
                0 => cfg.enabled = !cfg.enabled,
                1 => cfg.side = cfg.side.next(),
                2 => {
                    cfg.distance = match cfg.distance as u32 {
                        2 => 3.0,
                        3 => 4.0,
                        4 => 5.0,
                        5 => 6.0,
                        _ => 2.0,
                    };
                }
                3 => {
                    cfg.height = if (cfg.height - 0.6).abs() < 0.05 {
                        -0.2
                    } else if (cfg.height - (-0.2)).abs() < 0.05 {
                        0.0
                    } else if (cfg.height - 0.0).abs() < 0.05 {
                        0.2
                    } else if (cfg.height - 0.2).abs() < 0.05 {
                        0.4
                    } else {
                        0.6
                    };
                }
                4 => cfg.smooth = !cfg.smooth,
                5 => cfg.dynamic_raycast = !cfg.dynamic_raycast,
                _ => {}
            }
        }
        save_settings_to_host();
    });
}

static ID: &[u8] = b"shouldersurfing\0";
static TITLE: &[u8] = b"Shoulder Surfing 5.3\0";
static DESCRIPTION: &[u8] =
    b"Over-the-shoulder third-person camera with smooth motion and wall collision avoidance.\0";

static DESC: ModDesc = ModDesc {
    abi: ABI,
    api_level: 7,
    id: ID.as_ptr() as *const c_char,
    title: TITLE.as_ptr() as *const c_char,
    description: DESCRIPTION.as_ptr() as *const c_char,
    init: Some(init),
    tick: None,
    frame: Some(frame),
    settings: Some(settings_rows),
    settings_click: Some(settings_click),
    camera_offset: Some(camera_offset),
};

/// Entry point looked up by `horizon-mod-loader`.
///
/// # Safety
/// Called once when loading the shared library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(table: *const HostVTable) -> *const ModDesc {
    if !table.is_null() {
        HOST.store(table.cast_mut(), Ordering::Relaxed);
    }
    &DESC
}

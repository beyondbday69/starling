//! EyeMod: Procedural animated eyes, silky smooth blinking, sub-pixel look tracking,
//! and blushing for the Starling player model.

use horizon_mod_abi::{ABI, ButtonSink, HostVTable, ModDesc, PlayerEyes, PlayerInfo, guard, text};
use std::ffi::c_char;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};

static HOST: AtomicPtr<HostVTable> = AtomicPtr::new(std::ptr::null_mut());

fn host() -> Option<&'static HostVTable> {
    let ptr = HOST.load(Ordering::Relaxed);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BlushMode {
    Off,
    Soft,
    Rosy,
    Intense,
    Reactive,
}

impl BlushMode {
    fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Soft,
            2 => Self::Rosy,
            3 => Self::Intense,
            4 => Self::Reactive,
            _ => Self::Off,
        }
    }

    fn to_index(self) -> usize {
        match self {
            Self::Off => 0,
            Self::Soft => 1,
            Self::Rosy => 2,
            Self::Intense => 3,
            Self::Reactive => 4,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Soft => "Soft Pink",
            Self::Rosy => "Rosy (Cute)",
            Self::Intense => "Intense",
            Self::Reactive => "Reactive (Sneak / Low HP)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BlinkRate {
    Normal,
    Frequent,
    Relaxed,
}

impl BlinkRate {
    fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Frequent,
            2 => Self::Relaxed,
            _ => Self::Normal,
        }
    }

    fn to_index(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Frequent => 1,
            Self::Relaxed => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal (3-6s)",
            Self::Frequent => "Frequent (2-4s)",
            Self::Relaxed => "Relaxed (5-9s)",
        }
    }

    fn min_max_secs(self) -> (f32, f32) {
        match self {
            Self::Frequent => (2.0, 4.0),
            Self::Normal => (3.5, 6.5),
            Self::Relaxed => (5.5, 9.5),
        }
    }
}

struct State {
    blinking_enabled: bool,
    tracking_enabled: bool,
    blush_mode: BlushMode,
    blink_rate: BlinkRate,

    // Smooth animation state
    time_to_next_blink: f32,
    blink_progress: f32, // < 0 means not blinking, >= 0 means active
    is_double_blink: bool,

    // Smooth look tracking state
    last_yaw: f32,
    last_pitch: f32,
    current_look_x: f32, // -1.0 (left) .. 1.0 (right)
    current_look_y: f32, // -1.0 (down) .. 1.0 (up)

    // Smooth blushing state
    current_blush: f32,

    // Idle glance
    time_to_next_glance: f32,
    glance_target_x: f32,

    // Random generator state (simple XorShift)
    rng_seed: u32,
}

impl State {
    fn new() -> Self {
        Self {
            blinking_enabled: true,
            tracking_enabled: true,
            blush_mode: BlushMode::Rosy,
            blink_rate: BlinkRate::Normal,

            time_to_next_blink: 3.5,
            blink_progress: -1.0,
            is_double_blink: false,

            last_yaw: 0.0,
            last_pitch: 0.0,
            current_look_x: 0.0,
            current_look_y: 0.0,

            current_blush: 0.65,

            time_to_next_glance: 2.5,
            glance_target_x: 0.0,

            rng_seed: 0x1234_5678,
        }
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_seed ^= self.rng_seed << 13;
        self.rng_seed ^= self.rng_seed >> 17;
        self.rng_seed ^= self.rng_seed << 5;
        (self.rng_seed as f32) / (u32::MAX as f32)
    }

    fn rand_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.rand_f32()
    }

    fn load(&mut self) {
        let Some(host) = host() else { return };
        let mut buf = [0; horizon_mod_abi::TEXT];

        if unsafe { (host.setting)(c"blink".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.blinking_enabled = val != "0";
            }
        }
        if unsafe { (host.setting)(c"tracking".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                self.tracking_enabled = val != "0";
            }
        }
        if unsafe { (host.setting)(c"blush".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                if let Ok(idx) = val.parse::<usize>() {
                    self.blush_mode = BlushMode::from_index(idx);
                }
            }
        }
        if unsafe { (host.setting)(c"blink_rate".as_ptr(), buf.as_mut_ptr(), buf.len()) } > 0 {
            if let Some(val) = unsafe { text(buf.as_ptr()) } {
                if let Ok(idx) = val.parse::<usize>() {
                    self.blink_rate = BlinkRate::from_index(idx);
                }
            }
        }
    }

    fn save(&self) {
        let Some(host) = host() else { return };
        let s_blink = if self.blinking_enabled { c"1" } else { c"0" };
        let s_track = if self.tracking_enabled { c"1" } else { c"0" };
        let s_blush = match self.blush_mode {
            BlushMode::Off => c"0",
            BlushMode::Soft => c"1",
            BlushMode::Rosy => c"2",
            BlushMode::Intense => c"3",
            BlushMode::Reactive => c"4",
        };
        let s_rate = match self.blink_rate {
            BlinkRate::Normal => c"0",
            BlinkRate::Frequent => c"1",
            BlinkRate::Relaxed => c"2",
        };

        unsafe {
            (host.set_setting)(c"blink".as_ptr(), s_blink.as_ptr());
            (host.set_setting)(c"tracking".as_ptr(), s_track.as_ptr());
            (host.set_setting)(c"blush".as_ptr(), s_blush.as_ptr());
            (host.set_setting)(c"blink_rate".as_ptr(), s_rate.as_ptr());
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    let mut lock = STATE.lock().ok()?;
    let state = lock.get_or_insert_with(State::new);
    Some(f(state))
}

// Colors tailored for user's anime ruby/wine-red skin:
const C_LASH: u32 = 0xFF170F38; // #380f17 (ABGR in u32: A=FF, B=17, G=0F, R=38)
const C_PUPIL_TOP: u32 = 0xFF37236D; // #6d2337 (R=6D, G=23, B=37)
const C_PUPIL_BOT: u32 = 0xFF5B52A8; // #a8525b (R=A8, G=52, B=5B)
const C_WHITE_TOP: u32 = 0xFFBDB2D3; // #d3b2bd (R=D3, G=B2, B=BD)
const C_WHITE_BOT: u32 = 0xFFFBFAFF; // #fffafb (R=FF, G=FA, B=FB)
const C_SKIN_TOP: u32 = 0xFF94ABE9; // #e9ab94 (R=E9, G=AB, B=94)

/// Sub-pixel color linear interpolation with clamping.
#[inline]
fn lerp_u32(c1: u32, c2: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;

    let r1 = (c1 & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = ((c1 >> 16) & 0xFF) as f32;
    let a1 = (c1 >> 24) & 0xFF;

    let r2 = (c2 & 0xFF) as f32;
    let g2 = ((c2 >> 8) & 0xFF) as f32;
    let b2 = ((c2 >> 16) & 0xFF) as f32;

    let r = (r1 * inv + r2 * t).round() as u32;
    let g = (g1 * inv + g2 * t).round() as u32;
    let b = (b1 * inv + b2 * t).round() as u32;

    r | (g << 8) | (b << 16) | (a1 << 24)
}

/// Continuous eyelid coverage blending.
///
/// `y_lid` runs smoothly from `0.0` (fully open) to `2.0` (fully closed).
#[inline]
fn apply_eyelid(base: u32, lash: u32, skin: u32, row: usize, y_lid: f32) -> u32 {
    if row == 5 {
        if y_lid <= 0.0 {
            base
        } else if y_lid >= 1.0 {
            // Eyelid line has passed row 5; smoothly fade from lash into eyelid skin
            let f_skin = (y_lid - 1.0).clamp(0.0, 1.0);
            lerp_u32(lash, skin, f_skin)
        } else {
            // Eyelid edge is moving through row 5
            lerp_u32(base, lash, y_lid)
        }
    } else {
        // row == 6
        if y_lid <= 1.0 {
            base
        } else {
            // Eyelid edge is moving through row 6 towards full closure
            let f_lash = (y_lid - 1.0).clamp(0.0, 1.0);
            lerp_u32(base, lash, f_lash)
        }
    }
}

fn blend_color(base: u32, r_target: u8, g_target: u8, b_target: u8, alpha: f32) -> u32 {
    let b_r = (base & 0xFF) as f32;
    let b_g = ((base >> 8) & 0xFF) as f32;
    let b_b = ((base >> 16) & 0xFF) as f32;
    let b_a = (base >> 24) & 0xFF;

    let r = (b_r * (1.0 - alpha) + (r_target as f32) * alpha).clamp(0.0, 255.0) as u32;
    let g = (b_g * (1.0 - alpha) + (g_target as f32) * alpha).clamp(0.0, 255.0) as u32;
    let b = (b_b * (1.0 - alpha) + (b_target as f32) * alpha).clamp(0.0, 255.0) as u32;

    r | (g << 8) | (b << 16) | (b_a << 24)
}

unsafe extern "C" fn init(table: *const HostVTable) -> bool {
    let _ = guard(|| {
        if !table.is_null() {
            HOST.store(table.cast_mut(), Ordering::Relaxed);
        }
        with_state(|s| s.load());
    });
    true
}

unsafe extern "C" fn frame(partial: f32) {
    let _ = guard(|| {
        let Some(host) = host() else { return };
        let mut player = PlayerInfo::default();
        let in_world = unsafe { (host.player)(&mut player) != 0 };

        with_state(|state| {
            // Delta time estimation from tick fraction (partial)
            let dt = (partial * 0.05).clamp(0.005, 0.05);

            // 1. Procedural Smooth Blinking update
            let blink_duration = if state.is_double_blink { 0.36 } else { 0.22 };
            if state.blinking_enabled {
                if state.blink_progress >= 0.0 {
                    state.blink_progress += dt;
                    if state.blink_progress >= blink_duration {
                        state.blink_progress = -1.0;
                        state.is_double_blink = false;
                        let (min_s, max_s) = state.blink_rate.min_max_secs();
                        state.time_to_next_blink = state.rand_range(min_s, max_s);
                    }
                } else {
                    state.time_to_next_blink -= dt;
                    if state.time_to_next_blink <= 0.0 {
                        state.blink_progress = 0.0;
                        // 15% chance of a natural double blink
                        state.is_double_blink = state.rand_f32() < 0.15;
                    }
                }
            } else {
                state.blink_progress = -1.0;
            }

            // 2. Smooth Look Tracking update
            if in_world && state.tracking_enabled {
                let mut yaw_diff = player.yaw - state.last_yaw;
                while yaw_diff > 180.0 {
                    yaw_diff -= 360.0;
                }
                while yaw_diff < -180.0 {
                    yaw_diff += 360.0;
                }

                state.last_yaw = player.yaw;
                state.last_pitch = player.pitch;

                // Continuous head turn factor
                let turn_factor = (yaw_diff * 0.06).clamp(-1.0, 1.0);
                let pitch_factor = (-player.pitch * 0.02).clamp(-1.0, 1.0);

                // Idle wandering glance timer
                state.time_to_next_glance -= dt;
                if state.time_to_next_glance <= 0.0 {
                    let r = state.rand_f32();
                    state.glance_target_x = if r < 0.33 {
                        -0.65
                    } else if r > 0.66 {
                        0.65
                    } else {
                        0.0
                    };
                    state.time_to_next_glance = state.rand_range(2.0, 4.5);
                }

                let target_x = if turn_factor.abs() > 0.03 {
                    turn_factor
                } else {
                    state.glance_target_x
                };
                let target_y = pitch_factor;

                // Smooth exponential dampening towards target look
                let smooth_speed = 6.0;
                state.current_look_x +=
                    (target_x - state.current_look_x) * (dt * smooth_speed).min(1.0);
                state.current_look_y +=
                    (target_y - state.current_look_y) * (dt * smooth_speed).min(1.0);
            } else {
                state.current_look_x *= 0.9;
                state.current_look_y *= 0.9;
            }

            // 3. Smooth Blushing update
            let target_blush = match state.blush_mode {
                BlushMode::Off => 0.0,
                BlushMode::Soft => 0.35,
                BlushMode::Rosy => 0.65,
                BlushMode::Intense => 0.90,
                BlushMode::Reactive => {
                    if in_world && (player.sneaking() || player.health < 6.0) {
                        0.80
                    } else {
                        0.10
                    }
                }
            };
            state.current_blush += (target_blush - state.current_blush) * (dt * 3.0).min(1.0);
        });
    });
}

unsafe extern "C" fn player_eyes(_partial: f32, out: *mut PlayerEyes) {
    let _ = guard(|| {
        if out.is_null() {
            return;
        }
        let eyes = unsafe { &mut *out };

        with_state(|state| {
            eyes.active = 1;
            eyes.has_face_patch = 1;

            // 1. Calculate continuous blink closure (0.0: wide open, 1.0: fully closed)
            let mut closure = 0.0f32;
            if state.blink_progress >= 0.0 {
                let duration = if state.is_double_blink { 0.36 } else { 0.22 };
                let norm = (state.blink_progress / duration).clamp(0.0, 1.0);

                closure = if state.is_double_blink {
                    // Double blink: two smooth sine-squared peaks
                    if norm < 0.5 {
                        let sub = norm / 0.5;
                        (sub * std::f32::consts::PI).sin().powi(2)
                    } else {
                        let sub = (norm - 0.5) / 0.5;
                        (sub * std::f32::consts::PI).sin().powi(2)
                    }
                } else {
                    // Single smooth blink: peak at center with zero velocity at endpoints
                    (norm * std::f32::consts::PI).sin().powi(2)
                };
            }
            eyes.blink = (closure * 2.0).round() as u8;

            // 2. Continuous sub-pixel pupil positions
            let look_x = state.current_look_x.clamp(-1.0, 1.0);
            let look_y = state.current_look_y.clamp(-1.0, 1.0);
            eyes.look_x = look_x.round() as i8;
            eyes.look_y = look_y.round() as i8;

            // Right Eye base colors before eyelid:
            // Center (0.0): Col 1 is White, Col 2 is Pupil.
            let (r1_5, r2_5, r1_6, r2_6) = if look_x < 0.0 {
                // Glancing left (pupils smoothly shift towards col 1):
                let t = -look_x; // 0.0 .. 1.0
                (
                    lerp_u32(C_WHITE_TOP, C_PUPIL_TOP, t),
                    lerp_u32(C_PUPIL_TOP, C_WHITE_TOP, t),
                    lerp_u32(C_WHITE_BOT, C_PUPIL_BOT, t),
                    lerp_u32(C_PUPIL_BOT, C_WHITE_BOT, t),
                )
            } else {
                // Glancing right:
                let t = look_x;
                (
                    C_WHITE_TOP,
                    lerp_u32(C_PUPIL_TOP, C_PUPIL_TOP, t),
                    C_WHITE_BOT,
                    lerp_u32(C_PUPIL_BOT, C_PUPIL_BOT, t),
                )
            };

            // Left Eye base colors before eyelid:
            // Center (0.0): Col 5 is Pupil, Col 6 is White.
            let (l1_5, l2_5, l1_6, l2_6) = if look_x > 0.0 {
                // Glancing right (pupils smoothly shift towards col 6):
                let t = look_x; // 0.0 .. 1.0
                (
                    lerp_u32(C_PUPIL_TOP, C_WHITE_TOP, t),
                    lerp_u32(C_WHITE_TOP, C_PUPIL_TOP, t),
                    lerp_u32(C_PUPIL_BOT, C_WHITE_BOT, t),
                    lerp_u32(C_WHITE_BOT, C_PUPIL_BOT, t),
                )
            } else {
                // Glancing left:
                let t = -look_x;
                (
                    lerp_u32(C_PUPIL_TOP, C_PUPIL_TOP, t),
                    C_WHITE_TOP,
                    lerp_u32(C_PUPIL_BOT, C_PUPIL_BOT, t),
                    C_WHITE_BOT,
                )
            };

            // 3. Smooth vertical eyelid coverage
            // Looking down gives a natural subtle eyelid droop:
            let droop = if look_y < 0.0 { -look_y * 0.35 } else { 0.0 };
            let y_lid = (closure * 2.0 + droop).clamp(0.0, 2.0);

            // Apply smooth eyelid coverage to Right Eye (cols 1, 2)
            eyes.face_pixels[5 * 8 + 1] = apply_eyelid(r1_5, C_LASH, C_SKIN_TOP, 5, y_lid);
            eyes.face_pixels[5 * 8 + 2] = apply_eyelid(r2_5, C_LASH, C_SKIN_TOP, 5, y_lid);
            eyes.face_pixels[6 * 8 + 1] = apply_eyelid(r1_6, C_LASH, C_SKIN_TOP, 6, y_lid);
            eyes.face_pixels[6 * 8 + 2] = apply_eyelid(r2_6, C_LASH, C_SKIN_TOP, 6, y_lid);

            // Apply smooth eyelid coverage to Left Eye (cols 5, 6)
            eyes.face_pixels[5 * 8 + 5] = apply_eyelid(l1_5, C_LASH, C_SKIN_TOP, 5, y_lid);
            eyes.face_pixels[5 * 8 + 6] = apply_eyelid(l2_5, C_LASH, C_SKIN_TOP, 5, y_lid);
            eyes.face_pixels[6 * 8 + 5] = apply_eyelid(l1_6, C_LASH, C_SKIN_TOP, 6, y_lid);
            eyes.face_pixels[6 * 8 + 6] = apply_eyelid(l2_6, C_LASH, C_SKIN_TOP, 6, y_lid);

            // 4. Smooth Blushing Effect on cheeks (row 7: y=15)
            eyes.blush = (state.current_blush * 3.0).round() as u8;
            if state.current_blush > 0.01 {
                let b = state.current_blush;
                let r_blush = 242u8;
                let g_blush = 120u8;
                let b_blush = 142u8;

                // Right cheek (cols 1, 2)
                let base_r1 = eyes.face_pixels[7 * 8 + 1];
                let base_r2 = eyes.face_pixels[7 * 8 + 2];
                eyes.face_pixels[7 * 8 + 1] =
                    blend_color(base_r1, r_blush, g_blush, b_blush, b * 0.85);
                eyes.face_pixels[7 * 8 + 2] = blend_color(base_r2, r_blush, g_blush, b_blush, b);

                // Left cheek (cols 5, 6)
                let base_l1 = eyes.face_pixels[7 * 8 + 5];
                let base_l2 = eyes.face_pixels[7 * 8 + 6];
                eyes.face_pixels[7 * 8 + 5] = blend_color(base_l1, r_blush, g_blush, b_blush, b);
                eyes.face_pixels[7 * 8 + 6] =
                    blend_color(base_l2, r_blush, g_blush, b_blush, b * 0.85);

                // Soft blush on nose bridge when blushing intensely:
                if b > 0.6 {
                    let nose_b = (b - 0.6) / 0.4 * 0.35;
                    let base_b1 = eyes.face_pixels[6 * 8 + 3];
                    let base_b2 = eyes.face_pixels[6 * 8 + 4];
                    eyes.face_pixels[6 * 8 + 3] =
                        blend_color(base_b1, r_blush, g_blush, b_blush, nose_b);
                    eyes.face_pixels[6 * 8 + 4] =
                        blend_color(base_b2, r_blush, g_blush, b_blush, nose_b);
                }
            }
        });
    });
}

unsafe extern "C" fn settings_rows(sink: *const ButtonSink) {
    let _ = guard(|| {
        if sink.is_null() {
            return;
        }
        let sink = unsafe { &*sink };

        with_state(|state| {
            let row0 = format!(
                "Blinking: {}",
                if state.blinking_enabled { "ON" } else { "OFF" }
            );
            let row1 = format!(
                "Eye Tracking: {}",
                if state.tracking_enabled { "ON" } else { "OFF" }
            );
            let row2 = format!("Blushing: {}", state.blush_mode.name());
            let row3 = format!("Blink Frequency: {}", state.blink_rate.name());

            for row in &[row0, row1, row2, row3] {
                let mut c_str = row.as_bytes().to_vec();
                c_str.push(0);
                unsafe { (sink.push)(sink.ctx, c_str.as_ptr() as *const c_char) };
            }
        });
    });
}

unsafe extern "C" fn settings_click(button: u32) {
    let _ = guard(|| {
        with_state(|state| {
            match button {
                0 => state.blinking_enabled = !state.blinking_enabled,
                1 => state.tracking_enabled = !state.tracking_enabled,
                2 => {
                    let next = (state.blush_mode.to_index() + 1) % 5;
                    state.blush_mode = BlushMode::from_index(next);
                }
                3 => {
                    let next = (state.blink_rate.to_index() + 1) % 3;
                    state.blink_rate = BlinkRate::from_index(next);
                }
                _ => {}
            }
            state.save();
        });
    });
}

static ID: &[u8] = b"eyemod\0";
static TITLE: &[u8] = b"Dynamic Eyes & Blush 1.0\0";
static DESCRIPTION: &[u8] =
    b"Procedural animated eyes, blinking, camera look tracking, and rosy blushing for player skins.\0";

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
    camera_offset: None,
    player_eyes: Some(player_eyes),
};

/// The entry point looked up by `horizon-mod-loader`.
///
/// # Safety
/// `table` is the client's table, valid for the process's life.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn horizon_mod_entry(table: *const HostVTable) -> *const ModDesc {
    if !table.is_null() {
        HOST.store(table.cast_mut(), Ordering::Relaxed);
    }
    with_state(|s| s.load());
    &DESC
}

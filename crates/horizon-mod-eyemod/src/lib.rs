//! EyeMod: Procedural animated eyes, blinking, camera look tracking, and blushing
//! for the Starling player model.

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

    // Animation state
    time_to_next_blink: f32,
    blink_progress: f32, // < 0 means not blinking, 0.0..0.18 means blinking
    is_double_blink: bool,

    // Look tracking state
    last_yaw: f32,
    last_pitch: f32,
    current_look_x: f32, // -1.0 (left) .. 1.0 (right)
    current_look_y: f32, // -1.0 (down) .. 1.0 (up)

    // Idle glance
    time_to_next_glance: f32,
    glance_target_x: f32,
    glance_duration: f32,

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

            time_to_next_glance: 2.5,
            glance_target_x: 0.0,
            glance_duration: 0.0,

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
            // Delta time estimation (frame tick fraction ~ partial)
            let dt = (partial * 0.05).clamp(0.005, 0.1);

            // 1. Procedural Blinking update
            if state.blinking_enabled {
                if state.blink_progress >= 0.0 {
                    state.blink_progress += dt;
                    let blink_duration = if state.is_double_blink { 0.28 } else { 0.16 };
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
                        // 15% chance of a double blink
                        state.is_double_blink = state.rand_f32() < 0.15;
                    }
                }
            } else {
                state.blink_progress = -1.0;
            }

            // 2. Look Tracking update
            if in_world && state.tracking_enabled {
                let yaw_diff = player.yaw - state.last_yaw;
                state.last_yaw = player.yaw;
                state.last_pitch = player.pitch;

                // Shift target look based on yaw turning rate
                let turn_factor = (yaw_diff * 0.08).clamp(-1.0, 1.0);
                let pitch_factor = (-player.pitch * 0.03).clamp(-1.0, 1.0);

                // Idle wandering glance
                state.time_to_next_glance -= dt;
                if state.time_to_next_glance <= 0.0 {
                    let r = state.rand_f32();
                    state.glance_target_x = if r < 0.3 {
                        -0.8
                    } else if r > 0.7 {
                        0.8
                    } else {
                        0.0
                    };
                    state.glance_duration = state.rand_range(0.8, 1.8);
                    state.time_to_next_glance = state.rand_range(2.0, 4.5);
                }

                let target_x = if turn_factor.abs() > 0.05 {
                    turn_factor
                } else {
                    state.glance_target_x
                };

                // Smooth approach towards target look
                state.current_look_x += (target_x - state.current_look_x) * (dt * 8.0).min(1.0);
                state.current_look_y += (pitch_factor - state.current_look_y) * (dt * 6.0).min(1.0);
            } else {
                state.current_look_x *= 0.9;
                state.current_look_y *= 0.9;
            }
        });
    });
}

unsafe extern "C" fn player_eyes(_partial: f32, out: *mut PlayerEyes) {
    let _ = guard(|| {
        if out.is_null() {
            return;
        }
        let eyes = unsafe { &mut *out };

        let Some(host) = host() else { return };
        let mut player = PlayerInfo::default();
        let in_world = unsafe { (host.player)(&mut player) != 0 };

        with_state(|state| {
            eyes.active = 1;
            eyes.has_face_patch = 1;

            // Compute blink phase (0: open, 1: half, 2: full closed)
            let mut blink_phase = 0u8;
            if state.blink_progress >= 0.0 {
                let p = state.blink_progress;
                if state.is_double_blink {
                    // Double blink timeline: 0.0..0.12 (blink 1), 0.12..0.16 (open), 0.16..0.28 (blink 2)
                    if (0.02..0.10).contains(&p) || (0.18..0.26).contains(&p) {
                        blink_phase = 2;
                    } else {
                        blink_phase = 1;
                    }
                } else if (0.03..0.13).contains(&p) {
                    blink_phase = 2; // full closed
                } else {
                    blink_phase = 1; // half blink
                }
            }
            eyes.blink = blink_phase;

            // Discrete look direction: -1, 0, 1
            let look_x = if state.current_look_x < -0.3 {
                -1
            } else if state.current_look_x > 0.3 {
                1
            } else {
                0
            };
            let look_y = if state.current_look_y < -0.3 {
                -1
            } else if state.current_look_y > 0.3 {
                1
            } else {
                0
            };
            eyes.look_x = look_x;
            eyes.look_y = look_y;

            // Blushing determination
            let blush_intensity = match state.blush_mode {
                BlushMode::Off => 0.0,
                BlushMode::Soft => 0.35,
                BlushMode::Rosy => 0.65,
                BlushMode::Intense => 0.90,
                BlushMode::Reactive => {
                    if in_world && (player.sneaking() || player.health < 6.0) {
                        0.75
                    } else {
                        0.15
                    }
                }
            };

            // When blushing is active:
            eyes.blush = if blush_intensity > 0.5 {
                2
            } else if blush_intensity > 0.1 {
                1
            } else {
                0
            };

            // Render face pixels (8x8 grid: y=0..8, x=0..8)
            // Rows correspond to skin face y=8..15:
            // row 4: y=12 (eyelash top)
            // row 5: y=13 (upper eye)
            // row 6: y=14 (lower eye)
            // row 7: y=15 (cheeks under eyes)

            if blink_phase == 2 {
                // FULL BLINK: eyes completely closed
                // Row 5 (y=13) eyelid skin
                eyes.face_pixels[5 * 8 + 1] = C_SKIN_TOP;
                eyes.face_pixels[5 * 8 + 2] = C_SKIN_TOP;
                eyes.face_pixels[5 * 8 + 5] = C_SKIN_TOP;
                eyes.face_pixels[5 * 8 + 6] = C_SKIN_TOP;

                // Row 6 (y=14) closed eyelash line
                eyes.face_pixels[6 * 8 + 1] = C_LASH;
                eyes.face_pixels[6 * 8 + 2] = C_LASH;
                eyes.face_pixels[6 * 8 + 5] = C_LASH;
                eyes.face_pixels[6 * 8 + 6] = C_LASH;
            } else if blink_phase == 1 {
                // HALF BLINK: squint / half closed
                // Row 5 gets eyelash
                eyes.face_pixels[5 * 8 + 1] = C_LASH;
                eyes.face_pixels[5 * 8 + 2] = C_LASH;
                eyes.face_pixels[5 * 8 + 5] = C_LASH;
                eyes.face_pixels[5 * 8 + 6] = C_LASH;

                // Row 6 narrow slit
                eyes.face_pixels[6 * 8 + 1] = C_WHITE_BOT;
                eyes.face_pixels[6 * 8 + 2] = C_PUPIL_BOT;
                eyes.face_pixels[6 * 8 + 5] = C_PUPIL_BOT;
                eyes.face_pixels[6 * 8 + 6] = C_WHITE_BOT;
            } else {
                // EYES OPEN: apply look direction
                // Right Eye (cols 1, 2) and Left Eye (cols 5, 6)
                if look_x == -1 {
                    // Looking LEFT (screen left, player right): pupils shift left
                    // Right eye: pupil at col 1, sclera at col 2
                    eyes.face_pixels[5 * 8 + 1] = C_PUPIL_TOP;
                    eyes.face_pixels[5 * 8 + 2] = C_WHITE_TOP;
                    eyes.face_pixels[6 * 8 + 1] = C_PUPIL_BOT;
                    eyes.face_pixels[6 * 8 + 2] = C_WHITE_BOT;

                    // Left eye: pupil at col 5, sclera at col 6
                    eyes.face_pixels[5 * 8 + 5] = C_PUPIL_TOP;
                    eyes.face_pixels[5 * 8 + 6] = C_WHITE_TOP;
                    eyes.face_pixels[6 * 8 + 5] = C_PUPIL_BOT;
                    eyes.face_pixels[6 * 8 + 6] = C_WHITE_BOT;
                } else if look_x == 1 {
                    // Looking RIGHT (screen right, player left): pupils shift right
                    // Right eye: sclera at col 1, pupil at col 2
                    eyes.face_pixels[5 * 8 + 1] = C_WHITE_TOP;
                    eyes.face_pixels[5 * 8 + 2] = C_PUPIL_TOP;
                    eyes.face_pixels[6 * 8 + 1] = C_WHITE_BOT;
                    eyes.face_pixels[6 * 8 + 2] = C_PUPIL_BOT;

                    // Left eye: sclera at col 5, pupil at col 6
                    eyes.face_pixels[5 * 8 + 5] = C_WHITE_TOP;
                    eyes.face_pixels[5 * 8 + 6] = C_PUPIL_TOP;
                    eyes.face_pixels[6 * 8 + 5] = C_WHITE_BOT;
                    eyes.face_pixels[6 * 8 + 6] = C_PUPIL_BOT;
                } else {
                    // Looking CENTER (default anime look)
                    eyes.face_pixels[5 * 8 + 1] = C_WHITE_TOP;
                    eyes.face_pixels[5 * 8 + 2] = C_PUPIL_TOP;
                    eyes.face_pixels[6 * 8 + 1] = C_WHITE_BOT;
                    eyes.face_pixels[6 * 8 + 2] = C_PUPIL_BOT;

                    eyes.face_pixels[5 * 8 + 5] = C_PUPIL_TOP;
                    eyes.face_pixels[5 * 8 + 6] = C_WHITE_TOP;
                    eyes.face_pixels[6 * 8 + 5] = C_PUPIL_BOT;
                    eyes.face_pixels[6 * 8 + 6] = C_WHITE_BOT;
                }
            }

            // BLUSHING EFFECT:
            // Apply rosy tint on cheeks at row 7 (y=15), cols 1..2 and cols 5..6
            if blush_intensity > 0.05 {
                // Sweet strawberry rose blush: RGB(242, 120, 142)
                let r_blush = 242u8;
                let g_blush = 120u8;
                let b_blush = 142u8;

                // Right cheek (cols 1, 2)
                let base_r1 = eyes.face_pixels[7 * 8 + 1];
                let base_r2 = eyes.face_pixels[7 * 8 + 2];
                eyes.face_pixels[7 * 8 + 1] =
                    blend_color(base_r1, r_blush, g_blush, b_blush, blush_intensity * 0.85);
                eyes.face_pixels[7 * 8 + 2] =
                    blend_color(base_r2, r_blush, g_blush, b_blush, blush_intensity);

                // Left cheek (cols 5, 6)
                let base_l1 = eyes.face_pixels[7 * 8 + 5];
                let base_l2 = eyes.face_pixels[7 * 8 + 6];
                eyes.face_pixels[7 * 8 + 5] =
                    blend_color(base_l1, r_blush, g_blush, b_blush, blush_intensity);
                eyes.face_pixels[7 * 8 + 6] =
                    blend_color(base_l2, r_blush, g_blush, b_blush, blush_intensity * 0.85);

                // If intense blush, softly tint bridge at row 6, cols 3..4
                if blush_intensity > 0.7 {
                    let base_b1 = eyes.face_pixels[6 * 8 + 3];
                    let base_b2 = eyes.face_pixels[6 * 8 + 4];
                    eyes.face_pixels[6 * 8 + 3] =
                        blend_color(base_b1, r_blush, g_blush, b_blush, 0.35);
                    eyes.face_pixels[6 * 8 + 4] =
                        blend_color(base_b2, r_blush, g_blush, b_blush, 0.35);
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

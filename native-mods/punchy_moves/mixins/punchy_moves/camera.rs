//! Punchy Moves' shoulder camera, ported 1:1 from the Java client's
//! `punchymoves.client.camera` package: the settings (and the config clamps),
//! the wall-collision recovery, the avatar's camera visibility, the centre
//! aim ray's unprojection, and the controller that layers the situation
//! camera profiles (fence, ladder, proximity, elytra, map view, ranged,
//! passenger) over the activation transition.
//!
//! Pure Rust and std-only, so the tests run without a Horizon build. The game
//! side is three small helpers the bundle injects: `list.rs` (the Options >
//! Mods row and the two keys), `settings.rs` and `click.rs` (the page), `key.rs`
//! (the key hooks) and `camera_hook.rs` (each frame, `native_camera_pose`).
//!
//! Angles are degrees and positions are world blocks. Yaw is Minecraft's: `0`
//! faces `+Z`, `90` faces `-X`; a positive pitch looks down. That gives the
//! forward vector `(-sin yaw, 0, cos yaw)` and the camera's right
//! `(cos yaw, 0, sin yaw)`.

use std::cell::RefCell;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Keys and stored setting names
// ---------------------------------------------------------------------------

/// The key that toggles the shoulder camera, and its default: unbound until
/// the player picks one on Options > Mods.
pub const TOGGLE_CAMERA: &str = "camera";
/// The Java keybind `key.punchy_moves.camera_toggle_v`, on V out of the box.
pub const DEFAULT_TOGGLE_CAMERA: &str = "key.keyboard.v";
/// The Java keybind `key.punchy_moves.camera_editor`, which opens the
/// camera's settings screen.
pub const TOGGLE_EDITOR: &str = "camera_editor";
pub const DEFAULT_TOGGLE_EDITOR: &str = "key.keyboard.f7";
/// The key that toggles the orbit (free) camera, unbound by default too.
pub const TOGGLE_ORBIT: &str = "orbit";
/// The Java keybind `key.punchy_moves.free_camera`, on B out of the box.
pub const DEFAULT_TOGGLE_ORBIT: &str = "key.keyboard.b";

/// The stored setting names, the Java config file's keys, one per field.
pub const ENABLED: &str = "enabled";
pub const HORIZONTAL: &str = "horizontal";
pub const VERTICAL: &str = "vertical";
pub const DEPTH: &str = "depth";
pub const FOV: &str = "fov";
pub const ROTATION_YAW: &str = "rotationYaw";
pub const ROTATION_PITCH: &str = "rotationPitch";
pub const FENCE_CAMERA: &str = "fenceCamera";
pub const LADDER_CAMERA: &str = "ladderCamera";
pub const PROXIMITY_CAMERA: &str = "proximityCamera";
pub const ELYTRA_CAMERA: &str = "elytraCamera";
pub const MAP_CAMERA: &str = "mapCamera";
pub const BOW_AND_CROSSBOW_CAMERA_FIX: &str = "bowAndCrossbowCameraFix";
pub const SPEAR_AND_TRIDENT_CAMERA_FIX: &str = "spearAndTridentCameraFix";
pub const LADDER_BODY_LOCK: &str = "ladderBodyLock";

/// How many buttons the settings page has.
pub const BUTTONS: usize = 15;
/// The button index of each setting, in the page's order (the order
/// [`ShoulderCameraSettings::click`] numbers them).
pub const ENABLED_INDEX: usize = 0;
pub const HORIZONTAL_INDEX: usize = 1;
pub const VERTICAL_INDEX: usize = 2;
pub const DEPTH_INDEX: usize = 3;
pub const FOV_INDEX: usize = 4;
pub const YAW_INDEX: usize = 5;
pub const PITCH_INDEX: usize = 6;
pub const FENCE_INDEX: usize = 7;
pub const LADDER_INDEX: usize = 8;
pub const PROXIMITY_INDEX: usize = 9;
pub const ELYTRA_INDEX: usize = 10;
pub const MAP_INDEX: usize = 11;
pub const BOW_AND_CROSSBOW_INDEX: usize = 12;
pub const SPEAR_AND_TRIDENT_INDEX: usize = 13;
pub const LADDER_BODY_LOCK_INDEX: usize = 14;

// ---------------------------------------------------------------------------
// Small math, the Java `Mth` calls the camera package uses
// ---------------------------------------------------------------------------

/// `Mth.clamp` for f32.
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// `Mth.clamp` for f64.
fn clampd(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

/// `Mth.lerp(delta, start, end)`.
fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

/// The smooth 0-to-1 curve of a blend in `0.0..=1.0`.
fn smoothstep(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `ShoulderCameraController.smoothStep(edge0, edge1, value)`.
fn smooth_step(edge0: f32, edge1: f32, value: f32) -> f32 {
    smoothstep((value - edge0) / (edge1 - edge0))
}

// ---------------------------------------------------------------------------
// Camera profiles
// ---------------------------------------------------------------------------

/// `AnimationLibrary.CameraProfile`: one situation's camera, carried by a
/// clip's `camera` block. Complete when every field is finite and the
/// response is positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraProfile {
    /// Blocks to the side.
    pub horizontal: f32,
    /// Blocks above the eye.
    pub vertical: f32,
    /// Blocks behind the eye.
    pub depth: f32,
    /// Field of view, in degrees.
    pub fov: f32,
    /// How fast the blend moves, per second.
    pub response: f32,
}

impl CameraProfile {
    /// `CameraProfile.isComplete`.
    pub fn is_complete(&self) -> bool {
        self.horizontal.is_finite()
            && self.vertical.is_finite()
            && self.depth.is_finite()
            && self.fov.is_finite()
            && self.response.is_finite()
            && self.response > 0.0
    }
}

/// `PlayerAnimationEngine.MapViewCamera`: the camera a held-up map drives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapViewCamera {
    /// Blocks to the side.
    pub horizontal: f32,
    /// Blocks above the eye.
    pub vertical: f32,
    /// Blocks behind the eye.
    pub depth: f32,
    /// Field of view, in degrees.
    pub fov: f32,
    /// How fast the blend moves, per second.
    pub response: f32,
}

impl MapViewCamera {
    /// The engine's `MapViewCamera.DEFAULT`.
    pub const DEFAULT: MapViewCamera = MapViewCamera {
        horizontal: 0.0,
        vertical: 0.68,
        depth: 1.0,
        fov: 55.0,
        response: 4.5,
    };
}

/// `PlayerAnimationEngine.PassengerView`: the camera while riding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassengerView {
    /// Blocks to the side.
    pub horizontal: f32,
    /// Blocks above the eye.
    pub vertical: f32,
    /// Blocks behind the eye.
    pub depth: f32,
    /// Field of view, in degrees.
    pub fov: f32,
    /// How fast the blend moves, per second.
    pub response: f32,
}

// ---------------------------------------------------------------------------
// ShoulderCameraSettings
// ---------------------------------------------------------------------------

/// The Java defaults.
pub const DEFAULT_HORIZONTAL: f32 = 1.17;
pub const DEFAULT_VERTICAL: f32 = -0.11;
pub const DEFAULT_DEPTH: f32 = 2.5;
pub const DEFAULT_FOV: f32 = 70.0;
pub const DEFAULT_ROTATION_YAW: f32 = 0.0;
pub const DEFAULT_ROTATION_PITCH: f32 = 0.0;

/// The shoulder camera's settings: where the camera sits behind and beside the
/// eye, how much it turns, and in which situations it may engage at all.
///
/// One click on the settings page moves each value to the next step of its
/// table, so the page needs no text entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraSettings {
    /// The shoulder camera may engage.
    pub enabled: bool,
    /// Blocks to the right of the eye (left when negative).
    pub horizontal: f32,
    /// Blocks above the eye.
    pub vertical: f32,
    /// Blocks behind the eye the camera sits.
    pub depth: f32,
    /// The field of view the camera draws with, in degrees.
    pub fov: f32,
    /// Degrees the camera turns right of where the player looks.
    pub rotation_yaw: f32,
    /// Degrees the camera turns down of where the player looks.
    pub rotation_pitch: f32,
    /// Engage while standing on a fence, wall or other narrow-topped block.
    pub fence_camera: bool,
    /// Engage while holding a ladder or vine.
    pub ladder_camera: bool,
    /// Engage while a block is right behind the eye.
    pub proximity_camera: bool,
    /// Engage while gliding on an elytra.
    pub elytra_camera: bool,
    /// Engage while a map is held up.
    pub map_camera: bool,
    /// Draw the aim ray from the camera while aiming a bow or crossbow.
    pub bow_and_crossbow_camera_fix: bool,
    /// Draw the aim ray while aiming a spear or trident.
    pub spear_and_trident_camera_fix: bool,
    /// Keep the body facing the ladder while climbing.
    pub ladder_body_lock: bool,
}

/// The steps one click moves each setting to. A value between steps counts as
/// the first.
const HORIZONTAL_STEPS: [f32; 5] = [0.6, 0.9, 1.17, 1.5, 1.8];
const VERTICAL_STEPS: [f32; 5] = [-0.11, 0.0, 0.25, 0.6, 1.0];
const DEPTH_STEPS: [f32; 5] = [1.5, 2.0, 2.5, 3.5, 5.0];
const FOV_STEPS: [f32; 5] = [50.0, 60.0, 70.0, 85.0, 100.0];
const YAW_STEPS: [f32; 5] = [-20.0, -10.0, 0.0, 10.0, 20.0];
const PITCH_STEPS: [f32; 5] = [-20.0, -10.0, 0.0, 10.0, 20.0];

impl Default for ShoulderCameraSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            horizontal: DEFAULT_HORIZONTAL,
            vertical: DEFAULT_VERTICAL,
            depth: DEFAULT_DEPTH,
            fov: DEFAULT_FOV,
            rotation_yaw: DEFAULT_ROTATION_YAW,
            rotation_pitch: DEFAULT_ROTATION_PITCH,
            fence_camera: true,
            ladder_camera: true,
            proximity_camera: true,
            elytra_camera: true,
            map_camera: true,
            bow_and_crossbow_camera_fix: true,
            spear_and_trident_camera_fix: true,
            ladder_body_lock: true,
        }
    }
}

impl ShoulderCameraSettings {
    /// The defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// The Java setters' clamps.
    pub fn set_horizontal(&mut self, value: f32) {
        self.horizontal = clamp(value, -2.0, 2.0);
    }

    /// The Java setters' clamps.
    pub fn set_vertical(&mut self, value: f32) {
        self.vertical = clamp(value, -1.5, 2.0);
    }

    /// The Java setters' clamps.
    pub fn set_depth(&mut self, value: f32) {
        self.depth = clamp(value, 1.0, 8.0);
    }

    /// The Java setters' clamps.
    pub fn set_fov(&mut self, value: f32) {
        self.fov = clamp(value, 30.0, 110.0);
    }

    /// The Java setters' clamps (`rotationDegrees`: plus or minus 30).
    pub fn set_rotation_yaw(&mut self, value: f32) {
        self.rotation_yaw = clamp(value, -30.0, 30.0);
    }

    /// The Java setters' clamps (`rotationDegrees`: plus or minus 30).
    pub fn set_rotation_pitch(&mut self, value: f32) {
        self.rotation_pitch = clamp(value, -30.0, 30.0);
    }

    /// Reads the settings from stored values (`get` reads one key); a missing
    /// or unreadable value keeps its default, and every number passes through
    /// the Java setter's clamp.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut out = Self::default();
        let flag = |key: &str, fallback: bool| get(key).map_or(fallback, |v| v.trim() != "false");
        let number = |key: &str, clamp_fn: fn(f32) -> f32| {
            get(key)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(clamp_fn)
        };
        out.enabled = flag(ENABLED, true);
        if let Some(v) = number(HORIZONTAL, |v| clamp(v, -2.0, 2.0)) {
            out.horizontal = v;
        }
        if let Some(v) = number(VERTICAL, |v| clamp(v, -1.5, 2.0)) {
            out.vertical = v;
        }
        if let Some(v) = number(DEPTH, |v| clamp(v, 1.0, 8.0)) {
            out.depth = v;
        }
        if let Some(v) = number(FOV, |v| clamp(v, 30.0, 110.0)) {
            out.fov = v;
        }
        if let Some(v) = number(ROTATION_YAW, |v| clamp(v, -30.0, 30.0)) {
            out.rotation_yaw = v;
        }
        if let Some(v) = number(ROTATION_PITCH, |v| clamp(v, -30.0, 30.0)) {
            out.rotation_pitch = v;
        }
        out.fence_camera = flag(FENCE_CAMERA, true);
        out.ladder_camera = flag(LADDER_CAMERA, true);
        out.proximity_camera = flag(PROXIMITY_CAMERA, true);
        out.elytra_camera = flag(ELYTRA_CAMERA, true);
        out.map_camera = flag(MAP_CAMERA, true);
        out.bow_and_crossbow_camera_fix = flag(BOW_AND_CROSSBOW_CAMERA_FIX, true);
        out.spear_and_trident_camera_fix = flag(SPEAR_AND_TRIDENT_CAMERA_FIX, true);
        out.ladder_body_lock = flag(LADDER_BODY_LOCK, true);
        out
    }

    /// The settings page's buttons, in the order [`ShoulderCameraSettings::click`]
    /// numbers them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Shoulder camera: {}", on(self.enabled)),
            format!("Sideways: {:.2} blocks", self.horizontal),
            format!("Height: {:.2} blocks", self.vertical),
            format!("Back: {:.1} blocks", self.depth),
            format!("Field of view: {:.0} degrees", self.fov),
            format!("Turn right: {:.0} degrees", self.rotation_yaw),
            format!("Turn down: {:.0} degrees", self.rotation_pitch),
            format!("On fences: {}", on(self.fence_camera)),
            format!("On ladders: {}", on(self.ladder_camera)),
            format!("Beside walls: {}", on(self.proximity_camera)),
            format!("While gliding: {}", on(self.elytra_camera)),
            format!("With a map: {}", on(self.map_camera)),
            format!(
                "Bow and crossbow aim fix: {}",
                on(self.bow_and_crossbow_camera_fix)
            ),
            format!(
                "Spear and trident aim fix: {}",
                on(self.spear_and_trident_camera_fix)
            ),
            format!("Ladder body lock: {}", on(self.ladder_body_lock)),
        ]
    }

    /// A click on button `i` of [`ShoulderCameraSettings::labels`]; an index
    /// past the page does nothing.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => self.set_horizontal(cycle(&HORIZONTAL_STEPS, self.horizontal)),
            2 => self.set_vertical(cycle(&VERTICAL_STEPS, self.vertical)),
            3 => self.set_depth(cycle(&DEPTH_STEPS, self.depth)),
            4 => self.set_fov(cycle(&FOV_STEPS, self.fov)),
            5 => self.set_rotation_yaw(cycle(&YAW_STEPS, self.rotation_yaw)),
            6 => self.set_rotation_pitch(cycle(&PITCH_STEPS, self.rotation_pitch)),
            7 => self.fence_camera = !self.fence_camera,
            8 => self.ladder_camera = !self.ladder_camera,
            9 => self.proximity_camera = !self.proximity_camera,
            10 => self.elytra_camera = !self.elytra_camera,
            11 => self.map_camera = !self.map_camera,
            12 => self.bow_and_crossbow_camera_fix = !self.bow_and_crossbow_camera_fix,
            13 => self.spear_and_trident_camera_fix = !self.spear_and_trident_camera_fix,
            14 => self.ladder_body_lock = !self.ladder_body_lock,
            _ => {}
        }
    }

    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); BUTTONS] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            (ENABLED, flag(self.enabled)),
            (HORIZONTAL, format!("{:.2}", self.horizontal)),
            (VERTICAL, format!("{:.2}", self.vertical)),
            (DEPTH, format!("{:.2}", self.depth)),
            (FOV, format!("{:.1}", self.fov)),
            (ROTATION_YAW, format!("{:.1}", self.rotation_yaw)),
            (ROTATION_PITCH, format!("{:.1}", self.rotation_pitch)),
            (FENCE_CAMERA, flag(self.fence_camera)),
            (LADDER_CAMERA, flag(self.ladder_camera)),
            (PROXIMITY_CAMERA, flag(self.proximity_camera)),
            (ELYTRA_CAMERA, flag(self.elytra_camera)),
            (MAP_CAMERA, flag(self.map_camera)),
            (
                BOW_AND_CROSSBOW_CAMERA_FIX,
                flag(self.bow_and_crossbow_camera_fix),
            ),
            (
                SPEAR_AND_TRIDENT_CAMERA_FIX,
                flag(self.spear_and_trident_camera_fix),
            ),
            (LADDER_BODY_LOCK, flag(self.ladder_body_lock)),
        ]
    }

    /// Flips [`ShoulderCameraSettings::enabled`] (the camera key).
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

/// The next value of a step table; a value between steps counts as the first.
fn cycle(steps: &[f32], value: f32) -> f32 {
    let at = steps
        .iter()
        .position(|step| (step - value).abs() < 1e-4)
        .unwrap_or(0);
    steps[(at + 1) % steps.len()]
}

// ---------------------------------------------------------------------------
// CameraCollisionRecovery
// ---------------------------------------------------------------------------

/// Keeps the camera from crossing a wall without popping when the wall is
/// gone: it follows the fraction along the arm at once when the arm shortens,
/// and eases back out exponentially (`RECOVERY_RESPONSE` per second) when it
/// lengthens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraCollisionRecovery {
    /// How far along the arm the camera may sit, `0.0..=1.0`.
    fraction: f64,
}

/// `CameraCollisionRecovery.RECOVERY_RESPONSE`: the exponential recovery's
/// rate, per second.
pub const RECOVERY_RESPONSE: f32 = 7.0;

impl CameraCollisionRecovery {
    /// A camera at full reach.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fraction along the arm the camera sits at, `0.0..=1.0`.
    pub fn fraction(&self) -> f32 {
        self.fraction as f32
    }

    /// One frame with `blocked` the longest unblocked fraction the game found
    /// (`1.0` when nothing blocks) over `dt` seconds; returns the new fraction.
    ///
    /// A shorter arm is taken at once, so the camera never sits inside the
    /// wall for a frame; a longer one is eased out exponentially. A broken
    /// target reads as `0.0`.
    pub fn update(&mut self, blocked: f64, dt: f32) -> f64 {
        let target = if blocked.is_finite() {
            clampd(blocked, 0.0, 1.0)
        } else {
            0.0
        };
        if target <= self.fraction {
            self.fraction = target;
            return self.fraction;
        }
        let dt = clamp(dt, 0.0, 0.1) as f64;
        let step = 1.0 - (-RECOVERY_RESPONSE as f64 * dt).exp();
        self.fraction += (target - self.fraction) * step;
        if target - self.fraction < 1.0e-4 {
            self.fraction = target;
        }
        self.fraction
    }

    /// Back to full reach.
    pub fn reset(&mut self) {
        self.fraction = 1.0;
    }
}

impl Default for CameraCollisionRecovery {
    fn default() -> Self {
        Self { fraction: 1.0 }
    }
}

// ---------------------------------------------------------------------------
// AvatarCameraVisibility
// ---------------------------------------------------------------------------

/// `AvatarCameraVisibility`: how drawn the local avatar is with the shoulder
/// camera up — skipped outright when the camera is inside the body, faded by
/// the angle to the camera and the distance behind the player otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvatarCameraVisibility {
    /// `0.15..=1.0` while drawn.
    pub alpha: f32,
    /// The avatar is not submitted at all this frame.
    pub skip_avatar: bool,
}

/// The fully drawn answer.
const AVATAR_VISIBLE: AvatarCameraVisibility = AvatarCameraVisibility {
    alpha: 1.0,
    skip_avatar: false,
};

/// `AvatarCameraVisibility.sample`.
///
/// `moved` and `to_avatar` are the two position deltas from the camera
/// (`a - camera` and `b - camera`), `dir_a` and `dir_b` the two directions
/// the visibility is measured against, and the sizes are the player's bounding
/// box width and height and eye height, in blocks.
#[allow(clippy::too_many_arguments)] // the Java sample takes the same eight
pub fn avatar_camera_visibility(
    moved: [f64; 3],
    to_avatar: [f64; 3],
    dir_a: [f64; 3],
    dir_b: [f64; 3],
    bb_width: f64,
    bb_height: f64,
    eye_height: f64,
    activating: bool,
) -> AvatarCameraVisibility {
    if !activating && distance_squared(moved, to_avatar) <= 1.0e-8 {
        return AVATAR_VISIBLE;
    }
    let width = bb_width.max(0.001);
    let len = length(to_avatar);
    let near = width * 0.75;
    if len < near {
        return AvatarCameraVisibility {
            alpha: 1.0,
            skip_avatar: true,
        };
    }
    let far = width * 2.5;
    if len >= far {
        return AVATAR_VISIBLE;
    }
    let across = dot(to_avatar, dir_a).abs();
    let along = dot(to_avatar, dir_b);
    let half = width * 0.5;
    let eye = eye_height.max(0.001);
    let above_eye = (bb_height - eye).max(0.001);
    if across >= half || along <= -eye || along >= above_eye {
        return AVATAR_VISIBLE;
    }
    let side = clampd(across / half, 0.0, 1.0);
    let stacked = if along >= 0.0 {
        clampd(along / above_eye, 0.0, 1.0)
    } else {
        clampd(-along / eye, 0.0, 1.0)
    };
    let angled = clamp((side * side + stacked * stacked).sqrt() as f32, 0.15, 1.0);
    let t = clampd((len - near) / (far - near), 0.0, 1.0);
    let eased = (t * t * (3.0 - 2.0 * t)) as f32;
    AvatarCameraVisibility {
        alpha: lerp(eased, angled, 1.0),
        skip_avatar: false,
    }
}

/// The squared distance between two points.
fn distance_squared(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    dot(d, d)
}

/// The dot product of two vectors.
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The length of a vector.
fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

// ---------------------------------------------------------------------------
// CenterAimRay
// ---------------------------------------------------------------------------

/// A 4×4 matrix in column-major order, the order Joml's `Matrix4f` fills a
/// `FloatBuffer` with: elements `4*c..4*c+4` are column `c`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub(crate) columns: [[f32; 4]; 4],
}

impl Mat4 {
    /// Reads a column-major 16-float array.
    pub fn from_column_major(values: &[f32; 16]) -> Self {
        let mut columns = [[0.0f32; 4]; 4];
        for (c, column) in columns.iter_mut().enumerate() {
            *column = [
                values[c * 4],
                values[c * 4 + 1],
                values[c * 4 + 2],
                values[c * 4 + 3],
            ];
        }
        Self { columns }
    }

    /// The matrix times another: `self * other` (apply `other` first).
    pub fn mul(&self, other: &Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for (c, column) in out.iter_mut().enumerate() {
            for (r, cell) in column.iter_mut().enumerate() {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    sum += self.columns[k][r] * other.columns[c][k];
                }
                *cell = sum;
            }
        }
        Mat4 { columns: out }
    }

    /// The matrix times a column vector.
    pub fn transform(&self, v: [f32; 4]) -> [f32; 4] {
        let mut out = [0.0f32; 4];
        for (r, cell) in out.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (c, value) in v.iter().enumerate() {
                sum += self.columns[c][r] * value;
            }
            *cell = sum;
        }
        out
    }

    /// The determinant.
    pub fn determinant(&self) -> f32 {
        let m = self.columns;
        let mut det = 0.0f32;
        for c in 0..4 {
            // Laplace expansion along the first row, via 3x3 minors.
            let minor = |skip: usize| {
                let mut sub = [[0.0f32; 3]; 3];
                let mut sc = 0usize;
                for (cc, column) in m.iter().enumerate() {
                    if cc == skip {
                        continue;
                    }
                    for r in 0..3 {
                        sub[r][sc] = column[r + 1];
                    }
                    sc += 1;
                }
                sub[0][0] * (sub[1][1] * sub[2][2] - sub[1][2] * sub[2][1])
                    - sub[0][1] * (sub[1][0] * sub[2][2] - sub[1][2] * sub[2][0])
                    + sub[0][2] * (sub[1][0] * sub[2][1] - sub[1][1] * sub[2][0])
            };
            let sign = if c % 2 == 0 { 1.0 } else { -1.0 };
            det += sign * m[c][0] * minor(c);
        }
        det
    }

    /// Whether every element is finite.
    pub fn is_finite(&self) -> bool {
        self.columns.iter().flatten().all(|c| c.is_finite())
    }

    /// The translation column.
    pub fn translation(&self) -> [f32; 3] {
        [self.columns[3][0], self.columns[3][1], self.columns[3][2]]
    }

    /// The scale factors per axis, from the column lengths.
    pub fn scale_factors(&self) -> [f32; 3] {
        [
            (self.columns[0][0] * self.columns[0][0]
                + self.columns[0][1] * self.columns[0][1]
                + self.columns[0][2] * self.columns[0][2])
                .sqrt(),
            (self.columns[1][0] * self.columns[1][0]
                + self.columns[1][1] * self.columns[1][1]
                + self.columns[1][2] * self.columns[1][2])
                .sqrt(),
            (self.columns[2][0] * self.columns[2][0]
                + self.columns[2][1] * self.columns[2][1]
                + self.columns[2][2] * self.columns[2][2])
                .sqrt(),
        ]
    }

    /// `getUnnormalizedRotation`: the rotation the upper-left 3x3 carries,
    /// as a quaternion.
    pub fn unnormalized_rotation(&self) -> crate::engine::Quat {
        // Shear-free rotation extraction from the matrix, via the standard
        // ZYX decomposition scaled by the column lengths.
        let scale = self.scale_factors();
        let mut m = [[0.0f32; 3]; 3];
        for (r, row) in m.iter_mut().enumerate() {
            for (c, value) in row.iter_mut().enumerate() {
                *value = self.columns[c][r] / scale[c];
            }
        }
        let x = (-m[1][2]).atan2(m[2][2]);
        let y = m[0][2].clamp(-1.0, 1.0).asin();
        let z = (-m[0][1]).atan2(m[0][0]);
        crate::engine::Quat::from_zyx_degrees(crate::library::Vec3::new(
            x.to_degrees(),
            y.to_degrees(),
            z.to_degrees(),
        ))
    }

    /// The inverse, by Gauss-Jordan with partial pivoting; `None` when
    /// singular.
    pub fn invert(&self) -> Option<Mat4> {
        let mut a = [[0.0f64; 8]; 4];
        for (c, column) in self.columns.iter().enumerate() {
            for (r, value) in column.iter().enumerate() {
                a[r][c] = *value as f64;
            }
            a[c][4 + c] = 1.0;
        }
        for col in 0..4 {
            let pivot = (col..4)
                .max_by(|x, y| a[*x][col].abs().total_cmp(&a[*y][col].abs()))
                .filter(|row| a[*row][col].abs() > 1.0e-12)?;
            a.swap(col, pivot);
            let pivot_row = a[col];
            let pivot_value = pivot_row[col];
            for (r, row) in a.iter_mut().enumerate() {
                if r == col {
                    continue;
                }
                let factor = row[col] / pivot_value;
                if factor != 0.0 {
                    for (c, cell) in row.iter_mut().enumerate() {
                        *cell -= factor * pivot_row[c];
                    }
                }
            }
            for cell in a[col].iter_mut() {
                *cell /= pivot_value;
            }
        }
        let mut columns = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for r in 0..4 {
                columns[c][r] = a[r][4 + c] as f32;
            }
        }
        Some(Mat4 { columns })
    }
}

/// The ray from the centre of the picture, out of the camera: what the
/// crosshair points along, whatever the camera does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CenterAimRay {
    /// Where the ray starts, in world blocks.
    pub origin: [f64; 3],
    /// Its unit direction.
    pub direction: [f64; 3],
}

impl CenterAimRay {
    /// The ray of a camera at `origin` looking along `yaw` and `pitch`.
    pub fn from_look(origin: [f64; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            origin,
            direction: look_direction(yaw, pitch),
        }
    }

    /// The ray of a camera pose.
    pub fn from_pose(pose: &CameraPose) -> Self {
        Self::from_look(pose.position, pose.yaw, pose.pitch)
    }

    /// The point `distance` along the ray.
    pub fn at(&self, distance: f64) -> [f64; 3] {
        [
            self.origin[0] + self.direction[0] * distance,
            self.origin[1] + self.direction[1] * distance,
            self.origin[2] + self.direction[2] * distance,
        ]
    }

    /// `CenterAimRay.unproject`: the ray the middle of the picture looks
    /// along, from the view matrix and the projection matrix (both read as
    /// column-major 16-float arrays). The math runs in `f32`, like Joml's.
    ///
    /// Returns `None` when either matrix is singular or the result is not a
    /// usable finite ray.
    pub fn unproject(view: &[f32; 16], projection: &[f32; 16]) -> Option<Self> {
        let combined = Mat4::from_column_major(projection)
            .mul(&Mat4::from_column_major(view))
            .invert()?;
        let direction_point = combined.transform([0.0, 0.0, 1.0, 0.0]);
        let mid_point = combined.transform([0.0, 0.0, 0.5, 1.0]);
        let finite4 = |v: [f32; 4]| v.iter().all(|c| c.is_finite());
        if !finite4(direction_point) || !finite4(mid_point) {
            return None;
        }
        if direction_point[3].abs() < 1.0e-7 || mid_point[3].abs() < 1.0e-7 {
            return None;
        }
        let origin = [
            (direction_point[0] / direction_point[3]) as f64,
            (direction_point[1] / direction_point[3]) as f64,
            (direction_point[2] / direction_point[3]) as f64,
        ];
        let through = [
            (mid_point[0] / mid_point[3]) as f64,
            (mid_point[1] / mid_point[3]) as f64,
            (mid_point[2] / mid_point[3]) as f64,
        ];
        let direction = [
            through[0] - origin[0],
            through[1] - origin[1],
            through[2] - origin[2],
        ];
        if !direction.iter().all(|c| c.is_finite()) || !origin.iter().all(|c| c.is_finite()) {
            return None;
        }
        let squared = dot(direction, direction);
        if squared < 1.0e-12 {
            return None;
        }
        let scale = squared.sqrt();
        Some(Self {
            origin,
            direction: [
                direction[0] / scale,
                direction[1] / scale,
                direction[2] / scale,
            ],
        })
    }
}

/// The unit direction `yaw` and `pitch` look along, yaw `0` facing `+Z` and a
/// positive pitch looking down.
pub fn look_direction(yaw: f32, pitch: f32) -> [f64; 3] {
    let yaw = (yaw as f64).to_radians();
    let pitch = (pitch as f64).to_radians();
    let (sin_yaw, cos_yaw) = (yaw.sin(), yaw.cos());
    let (sin_pitch, cos_pitch) = (pitch.sin(), pitch.cos());
    [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch]
}

// ---------------------------------------------------------------------------
// ShoulderCameraController
// ---------------------------------------------------------------------------

/// The camera this frame is drawn from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    /// Where the camera sits, in world blocks.
    pub position: [f64; 3],
    /// Its yaw in degrees.
    pub yaw: f32,
    /// Its pitch in degrees.
    pub pitch: f32,
    /// How far the shoulder camera has engaged, `0.0..=1.0`.
    pub weight: f32,
}

impl CameraPose {
    /// The centre-screen aim ray of this pose.
    pub fn aim_ray(&self) -> CenterAimRay {
        CenterAimRay::from_look(self.position, self.yaw, self.pitch)
    }
}

/// `CAMERA_ENTER_SECONDS`: how long the activation blend takes to engage.
pub const CAMERA_ENTER_SECONDS: f32 = 0.9;
/// `CAMERA_EXIT_SECONDS`: how long it takes to disengage.
pub const CAMERA_EXIT_SECONDS: f32 = 0.72;
/// `FIRST_PERSON_RENDER_GRACE_NANOS`: how long the avatar stays hidden after
/// the camera hands back to first person.
pub const FIRST_PERSON_RENDER_GRACE: Duration = Duration::from_nanos(200_000_000);
/// The response a profile without one falls back to.
pub const DEFAULT_PROFILE_RESPONSE: f32 = 4.0;
/// The depth below which the transition hides the local player.
pub const TRANSITION_HIDE_DEPTH: f32 = 0.58;
/// How far a free-camera turn moves per mouse pixel, in degrees.
pub const FREE_CAMERA_TURN: f32 = 0.15;

/// Blends the shoulder camera in and out and layers the situation camera
/// profiles over it, the Java `ShoulderCameraController`.
///
/// The Java class keeps its state in statics; here it is an instance the game
/// glue owns through [`controller`], so tests can build their own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraController {
    /// Whether the vanilla camera is in third person back right now; the
    /// glue sets this each frame, `isActive` needs it.
    pub third_person: bool,
    activated: bool,
    requested: bool,
    activation_blend: f32,
    transition_horizontal: f32,
    transition_vertical: f32,
    transition_depth: f32,
    transition_fov: f32,
    fence_blend: f32,
    fence_latched: bool,
    proximity_blend: f32,
    proximity_latched: bool,
    proximity_mining_hold: f32,
    elytra_blend: f32,
    ladder_blend: f32,
    map_view_blend: f32,
    ranged_blend: f32,
    passenger_blend: f32,
    passenger_view: Option<PassengerView>,
    fence_profile: Option<CameraProfile>,
    proximity_profile: Option<CameraProfile>,
    elytra_profile: Option<CameraProfile>,
    ladder_profile: Option<CameraProfile>,
    ranged_profile: Option<CameraProfile>,
    map_view: MapViewCamera,
    hide_local_player_until: Option<Instant>,
    avatar_alpha: f32,
    avatar_skip: bool,
    spyglass_takeover: bool,
    free_camera: bool,
    free_camera_yaw: f32,
    free_camera_pitch: f32,
    vanilla_perspective_override: bool,
    collision: CameraCollisionRecovery,
}

impl Default for ShoulderCameraController {
    fn default() -> Self {
        Self {
            third_person: true,
            activated: false,
            requested: false,
            activation_blend: 0.0,
            transition_horizontal: 0.0,
            transition_vertical: 0.0,
            transition_depth: 0.0,
            transition_fov: 0.0,
            fence_blend: 0.0,
            fence_latched: false,
            proximity_blend: 0.0,
            proximity_latched: false,
            proximity_mining_hold: 0.0,
            elytra_blend: 0.0,
            ladder_blend: 0.0,
            map_view_blend: 0.0,
            ranged_blend: 0.0,
            passenger_blend: 0.0,
            passenger_view: None,
            fence_profile: None,
            proximity_profile: None,
            elytra_profile: None,
            ladder_profile: None,
            ranged_profile: None,
            map_view: MapViewCamera::DEFAULT,
            hide_local_player_until: None,
            avatar_alpha: 1.0,
            avatar_skip: false,
            spyglass_takeover: false,
            free_camera: false,
            free_camera_yaw: 0.0,
            free_camera_pitch: 0.0,
            vanilla_perspective_override: false,
            collision: CameraCollisionRecovery::default(),
        }
    }
}

/// What [`ShoulderCameraController::toggle`] and the free camera's toggle
/// tell the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMessage {
    /// The shoulder camera turned on.
    CameraEnabled,
    /// The shoulder camera turned off.
    CameraDisabled,
    /// The free camera needs the shoulder camera first.
    FreeCameraRequiresCustom,
    /// The free camera turned on.
    FreeCameraEnabled,
    /// The free camera turned off.
    FreeCameraDisabled,
}

impl CameraMessage {
    /// The translation key the game glue shows.
    pub fn key(self) -> &'static str {
        match self {
            CameraMessage::CameraEnabled => "punchy_moves.camera.enabled",
            CameraMessage::CameraDisabled => "punchy_moves.camera.disabled",
            CameraMessage::FreeCameraRequiresCustom => "punchy_moves.free_camera.requires_custom",
            CameraMessage::FreeCameraEnabled => "punchy_moves.free_camera.enabled",
            CameraMessage::FreeCameraDisabled => "punchy_moves.free_camera.disabled",
        }
    }
}

impl ShoulderCameraController {
    /// A controller resting on vanilla's camera.
    pub fn new() -> Self {
        Self::default()
    }

    /// `isActive`: engaged and the vanilla camera is in third person back.
    pub fn is_active(&self) -> bool {
        self.activated && self.third_person
    }

    /// `hasCustomCamera`.
    pub fn has_custom_camera(&self) -> bool {
        self.activated && self.third_person
    }

    /// `isActivationTransitionRunning`.
    pub fn is_activation_transition_running(&self) -> bool {
        self.activated && (!self.requested || self.activation_blend < 1.0)
    }

    /// How far the activation blend has gone, `0.0..=1.0`.
    pub fn activation_blend(&self) -> f32 {
        self.activation_blend
    }

    /// Whether the camera is asked for.
    pub fn requested(&self) -> bool {
        self.requested
    }

    /// `updateActivationTransition`: eases the activation blend, resumes a
    /// saved camera, and drops the camera when vanilla left third person.
    pub fn update_activation_transition(&mut self, dt: f32, settings: &ShoulderCameraSettings) {
        if self.activated && !self.third_person {
            self.on_vanilla_perspective_change();
        }
        self.resume_saved_camera(settings);
        if self.activated {
            let seconds = if self.requested {
                CAMERA_ENTER_SECONDS
            } else {
                CAMERA_EXIT_SECONDS
            };
            let direction = if self.requested { 1.0 } else { -1.0 };
            self.activation_blend = clamp(
                self.activation_blend + direction * clamp(dt, 0.0, 0.1) / seconds,
                0.0,
                1.0,
            );
        }
    }

    /// `updateSpyglassCameraTransition`: the spyglass takes the camera over
    /// and hands it back.
    pub fn update_spyglass_camera_transition(
        &mut self,
        taking: bool,
        settings: &ShoulderCameraSettings,
    ) {
        if taking {
            if !self.spyglass_takeover && self.third_person && self.activated && self.requested {
                self.spyglass_takeover = true;
                self.stop_free_camera(true);
                self.capture_transition_target(true, settings);
                self.requested = false;
            }
        } else if self.spyglass_takeover {
            self.spyglass_takeover = false;
            self.activated = true;
            self.requested = true;
            self.capture_transition_target(false, settings);
        }
    }

    /// `finishActivationFrame`: once the exit blend has settled, hand back to
    /// first person and hide the player for the render grace.
    pub fn finish_activation_frame(&mut self, now: Instant) {
        if self.activated && !self.requested && self.activation_blend <= 0.0 {
            self.hide_local_player_until = Some(now + FIRST_PERSON_RENDER_GRACE);
            self.activated = false;
            self.reset_dynamic_transitions();
        }
    }

    /// `updateFenceTransition`: `disengage` and `engage` are the two latch
    /// inputs the game glue reads off the block under the player.
    pub fn update_fence_transition(
        &mut self,
        disengage: bool,
        engage: bool,
        dt: f32,
        profile: Option<CameraProfile>,
    ) {
        if profile.is_some_and(|p| p.is_complete()) {
            self.fence_profile = profile;
        }
        let complete = self.fence_profile.is_some_and(|p| p.is_complete());
        if !complete {
            self.fence_latched = false;
        } else if engage {
            self.fence_latched = true;
        } else if disengage {
            self.fence_latched = false;
        }
        let target = if self.fence_latched { 1.0 } else { 0.0 };
        let response = 1.0 - (-self.response(self.fence_profile) * clamp(dt, 0.0, 0.1)).exp();
        self.fence_blend = lerp(response, self.fence_blend, target);
        if (self.fence_blend - target).abs() < 1.0e-4 {
            self.fence_blend = target;
        }
    }

    /// `updateProximityTransition`: `distance` is how far the block behind
    /// the eye is, `mining` whether the player is breaking a block, and
    /// `blocked` whether something else holds the camera off.
    #[allow(clippy::too_many_arguments)]
    pub fn update_proximity_transition(
        &mut self,
        distance: f64,
        mining: bool,
        blocked: bool,
        dt: f32,
        profile: Option<CameraProfile>,
        proximity_blocks: f32,
        mining_blocks: f32,
        hold_seconds: f32,
        engage_response: f32,
        release_response: f32,
    ) {
        if profile.is_some_and(|p| p.is_complete()) {
            self.proximity_profile = profile;
        }
        let complete = self.proximity_profile.is_some_and(|p| p.is_complete());
        if complete {
            let near = proximity_blocks.max(0.0);
            let far = near.max(mining_blocks);
            if mining && distance.is_finite() && distance <= far as f64 {
                self.proximity_mining_hold = hold_seconds.max(0.0);
            } else {
                self.proximity_mining_hold = (self.proximity_mining_hold - dt.max(0.0)).max(0.0);
            }
            let close = !blocked && distance.is_finite() && distance <= near as f64;
            self.proximity_latched = close || self.proximity_mining_hold > 1.0e-4;
        } else {
            self.proximity_latched = false;
            self.proximity_mining_hold = 0.0;
        }
        let target = if self.proximity_latched { 1.0 } else { 0.0 };
        let response = if self.proximity_latched {
            engage_response.max(0.1)
        } else {
            release_response.max(0.1)
        };
        let step = 1.0 - (-response * clamp(dt, 0.0, 0.1)).exp();
        self.proximity_blend = lerp(step, self.proximity_blend, target);
        if (self.proximity_blend - target).abs() < 1.0e-4 {
            self.proximity_blend = target;
        }
    }

    /// `updateElytraTransition`.
    pub fn update_elytra_transition(
        &mut self,
        gliding: bool,
        dt: f32,
        profile: Option<CameraProfile>,
    ) {
        if profile.is_some_and(|p| p.is_complete()) {
            self.elytra_profile = profile;
        }
        let engaged = gliding && self.elytra_profile.is_some_and(|p| p.is_complete());
        let target = if engaged { 1.0 } else { 0.0 };
        let response = 1.0 - (-self.response(self.elytra_profile) * clamp(dt, 0.0, 0.1)).exp();
        self.elytra_blend = lerp(response, self.elytra_blend, target);
        if (self.elytra_blend - target).abs() < 1.0e-4 {
            self.elytra_blend = target;
        }
    }

    /// `updateLadderTransition`.
    pub fn update_ladder_transition(
        &mut self,
        climbing: bool,
        dt: f32,
        profile: Option<CameraProfile>,
    ) {
        if profile.is_some_and(|p| p.is_complete()) {
            self.ladder_profile = profile;
        }
        let engaged = climbing && self.ladder_profile.is_some_and(|p| p.is_complete());
        let target = if engaged { 1.0 } else { 0.0 };
        let response = 1.0 - (-self.response(self.ladder_profile) * clamp(dt, 0.0, 0.1)).exp();
        self.ladder_blend = lerp(response, self.ladder_blend, target);
        if (self.ladder_blend - target).abs() < 1.0e-4 {
            self.ladder_blend = target;
        }
    }

    /// `updateMapViewTransition`.
    pub fn update_map_view_transition(&mut self, held: bool, dt: f32) {
        let target = if held { 1.0 } else { 0.0 };
        let response = 1.0 - (-self.map_view.response.max(0.1) * clamp(dt, 0.0, 0.1)).exp();
        self.map_view_blend = lerp(response, self.map_view_blend, target);
        if (self.map_view_blend - target).abs() < 1.0e-4 {
            self.map_view_blend = target;
        }
    }

    /// `updateRangedCameraTransition`.
    pub fn update_ranged_camera_transition(
        &mut self,
        aiming: bool,
        dt: f32,
        profile: Option<CameraProfile>,
    ) {
        if profile.is_some_and(|p| p.is_complete()) {
            self.ranged_profile = profile;
        }
        let target = if aiming && self.ranged_profile.is_some_and(|p| p.is_complete()) {
            1.0
        } else {
            0.0
        };
        let response = 1.0 - (-self.response(self.ranged_profile) * clamp(dt, 0.0, 0.1)).exp();
        self.ranged_blend = lerp(response, self.ranged_blend, target);
        if (self.ranged_blend - target).abs() < 1.0e-4 {
            self.ranged_blend = target;
        }
    }

    /// `updatePassengerTransition`.
    pub fn update_passenger_transition(&mut self, view: Option<PassengerView>, dt: f32) {
        if view.is_some() {
            self.passenger_view = view;
        }
        let target = if view.is_none() { 0.0 } else { 1.0 };
        let response = match view {
            Some(passenger) => passenger.response,
            None => self
                .passenger_view
                .map_or(4.0, |passenger| passenger.response),
        };
        let step = 1.0 - (-response.max(0.1) * clamp(dt, 0.0, 0.1)).exp();
        self.passenger_blend = lerp(step, self.passenger_blend, target);
        if (self.passenger_blend - target).abs() < 1.0e-4 {
            self.passenger_blend = target;
            if target == 0.0 {
                self.passenger_view = None;
            }
        }
    }

    /// `resetDynamicTransitions`.
    pub fn reset_dynamic_transitions(&mut self) {
        self.fence_blend = 0.0;
        self.fence_latched = false;
        self.proximity_blend = 0.0;
        self.proximity_latched = false;
        self.proximity_mining_hold = 0.0;
        self.elytra_blend = 0.0;
        self.ladder_blend = 0.0;
        self.map_view_blend = 0.0;
        self.ranged_blend = 0.0;
        self.passenger_blend = 0.0;
        self.passenger_view = None;
        self.fence_profile = None;
        self.proximity_profile = None;
        self.elytra_profile = None;
        self.ladder_profile = None;
        self.ranged_profile = None;
        self.avatar_alpha = 1.0;
        self.avatar_skip = false;
    }

    /// `updateLocalAvatarCameraVisibility`: reworks the avatar's alpha from
    /// the camera position, two player positions and two directions.
    #[allow(clippy::too_many_arguments)]
    pub fn update_local_avatar_camera_visibility(
        &mut self,
        active: bool,
        bb_width: f64,
        bb_height: f64,
        eye_height: f64,
        camera: [f64; 3],
        pos_a: [f64; 3],
        pos_b: [f64; 3],
        dir_a: [f64; 3],
        dir_b: [f64; 3],
    ) {
        self.avatar_alpha = 1.0;
        self.avatar_skip = false;
        if active && self.is_active() {
            let visibility = avatar_camera_visibility(
                [
                    pos_a[0] - camera[0],
                    pos_a[1] - camera[1],
                    pos_a[2] - camera[2],
                ],
                [
                    pos_b[0] - camera[0],
                    pos_b[1] - camera[1],
                    pos_b[2] - camera[2],
                ],
                dir_a,
                dir_b,
                bb_width,
                bb_height,
                eye_height,
                self.is_activation_transition_running(),
            );
            self.avatar_alpha = visibility.alpha;
            self.avatar_skip = visibility.skip_avatar;
        }
    }

    /// `localAvatarAlpha`.
    pub fn local_avatar_alpha(&self) -> f32 {
        clamp(self.avatar_alpha, 0.0, 1.0)
    }

    /// `shouldSkipLocalAvatarForCameraCollision`.
    pub fn should_skip_local_avatar_for_camera_collision(&self) -> bool {
        self.is_active() && self.avatar_skip
    }

    /// `effectiveHorizontal`.
    pub fn effective_horizontal(&self, settings: &ShoulderCameraSettings) -> f32 {
        let contextual = self.contextual_horizontal(self.smooth_proximity_blend(), settings);
        if self.activated && self.activation_blend < 1.0 {
            self.transition_horizontal * smooth_step(0.18, 1.0, self.activation_blend)
        } else {
            contextual
        }
    }

    /// `effectiveVertical`.
    pub fn effective_vertical(&self, settings: &ShoulderCameraSettings) -> f32 {
        let contextual = self.contextual_vertical(self.smooth_proximity_blend(), settings);
        if self.activated && self.activation_blend < 1.0 {
            self.transition_vertical * smooth_step(0.12, 1.0, self.activation_blend)
        } else {
            contextual
        }
    }

    /// `effectiveDepth`.
    pub fn effective_depth(&self, settings: &ShoulderCameraSettings) -> f32 {
        let contextual = self.contextual_depth(self.smooth_proximity_blend(), settings);
        if self.activated && self.activation_blend < 1.0 {
            self.transition_depth * smooth_step(0.0, 1.0, self.activation_blend)
        } else {
            contextual
        }
    }

    /// `effectiveFov`: `vanilla_fov` is the options' field of view.
    pub fn effective_fov(&self, settings: &ShoulderCameraSettings, vanilla_fov: f32) -> f32 {
        let contextual = self.contextual_fov(self.smooth_proximity_blend(), settings);
        if self.activated && self.activation_blend < 1.0 {
            lerp(
                smooth_step(0.3, 1.0, self.activation_blend),
                vanilla_fov,
                self.transition_fov,
            )
        } else {
            contextual
        }
    }

    /// `shouldHideLocalPlayerDuringTransition`.
    pub fn should_hide_local_player_during_transition(
        &self,
        settings: &ShoulderCameraSettings,
        now: Instant,
    ) -> bool {
        self.is_active()
            && (self
                .hide_local_player_until
                .is_some_and(|until| now < until)
                || (self.is_activation_transition_running()
                    && self.effective_depth(settings) < TRANSITION_HIDE_DEPTH))
    }

    /// `proximityNeutralHorizontal`: the horizontal offset with the proximity
    /// blend forced to neutral.
    pub fn proximity_neutral_horizontal(&self, settings: &ShoulderCameraSettings) -> f32 {
        self.contextual_horizontal(0.0, settings)
    }

    /// `proximityNeutralVertical`.
    pub fn proximity_neutral_vertical(&self, settings: &ShoulderCameraSettings) -> f32 {
        self.contextual_vertical(0.0, settings)
    }

    /// `proximityNeutralDepth`.
    pub fn proximity_neutral_depth(&self, settings: &ShoulderCameraSettings) -> f32 {
        self.contextual_depth(0.0, settings)
    }

    fn contextual_horizontal(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
        let base = lerp(
            self.smooth_fence_blend(),
            settings.horizontal,
            self.profile_horizontal(self.fence_profile, settings),
        );
        let with_proximity = lerp(
            proximity,
            base,
            self.profile_horizontal(self.proximity_profile, settings),
        );
        let with_ladder = lerp(
            self.smooth_ladder_blend(),
            with_proximity,
            self.profile_horizontal(self.ladder_profile, settings),
        );
        let with_elytra = lerp(
            self.smooth_elytra_blend(),
            with_ladder,
            self.profile_horizontal(self.elytra_profile, settings),
        );
        let with_passenger = match self.passenger_view {
            Some(passenger) => lerp(
                self.smooth_passenger_blend(),
                with_elytra,
                passenger.horizontal,
            ),
            None => with_elytra,
        };
        let with_map = lerp(
            self.smooth_map_view_blend(),
            with_passenger,
            self.map_view.horizontal,
        );
        lerp(
            self.smooth_ranged_blend(),
            with_map,
            self.profile_horizontal(self.ranged_profile, settings),
        )
    }

    fn contextual_vertical(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
        let base = lerp(
            self.smooth_fence_blend(),
            settings.vertical,
            self.profile_vertical(self.fence_profile, settings),
        );
        let with_proximity = lerp(
            proximity,
            base,
            self.profile_vertical(self.proximity_profile, settings),
        );
        let with_ladder = lerp(
            self.smooth_ladder_blend(),
            with_proximity,
            self.profile_vertical(self.ladder_profile, settings),
        );
        let with_elytra = lerp(
            self.smooth_elytra_blend(),
            with_ladder,
            self.profile_vertical(self.elytra_profile, settings),
        );
        let with_passenger = match self.passenger_view {
            Some(passenger) => lerp(
                self.smooth_passenger_blend(),
                with_elytra,
                passenger.vertical,
            ),
            None => with_elytra,
        };
        let with_map = lerp(
            self.smooth_map_view_blend(),
            with_passenger,
            self.map_view.vertical,
        );
        lerp(
            self.smooth_ranged_blend(),
            with_map,
            self.profile_vertical(self.ranged_profile, settings),
        )
    }

    fn contextual_depth(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
        let base = lerp(
            self.smooth_fence_blend(),
            settings.depth,
            self.profile_depth(self.fence_profile, settings),
        );
        let with_proximity = lerp(
            proximity,
            base,
            self.profile_depth(self.proximity_profile, settings),
        );
        let with_ladder = lerp(
            self.smooth_ladder_blend(),
            with_proximity,
            self.profile_depth(self.ladder_profile, settings),
        );
        let with_elytra = lerp(
            self.smooth_elytra_blend(),
            with_ladder,
            self.profile_depth(self.elytra_profile, settings),
        );
        let with_passenger = match self.passenger_view {
            Some(passenger) => lerp(self.smooth_passenger_blend(), with_elytra, passenger.depth),
            None => with_elytra,
        };
        let with_map = lerp(
            self.smooth_map_view_blend(),
            with_passenger,
            self.map_view.depth,
        );
        lerp(
            self.smooth_ranged_blend(),
            with_map,
            self.profile_depth(self.ranged_profile, settings),
        )
    }

    fn contextual_fov(&self, proximity: f32, settings: &ShoulderCameraSettings) -> f32 {
        let base = lerp(
            self.smooth_fence_blend(),
            settings.fov,
            self.profile_fov(self.fence_profile, settings),
        );
        let with_proximity = lerp(
            proximity,
            base,
            self.profile_fov(self.proximity_profile, settings),
        );
        let with_ladder = lerp(
            self.smooth_ladder_blend(),
            with_proximity,
            self.profile_fov(self.ladder_profile, settings),
        );
        let with_elytra = lerp(
            self.smooth_elytra_blend(),
            with_ladder,
            self.profile_fov(self.elytra_profile, settings),
        );
        let with_passenger = match self.passenger_view {
            Some(passenger) => lerp(self.smooth_passenger_blend(), with_elytra, passenger.fov),
            None => with_elytra,
        };
        let with_map = lerp(
            self.smooth_map_view_blend(),
            with_passenger,
            self.map_view.fov,
        );
        lerp(
            self.smooth_ranged_blend(),
            with_map,
            self.profile_fov(self.ranged_profile, settings),
        )
    }

    fn smooth_fence_blend(&self) -> f32 {
        smoothstep(self.fence_blend)
    }

    fn smooth_proximity_blend(&self) -> f32 {
        smoothstep(self.proximity_blend)
    }

    fn smooth_elytra_blend(&self) -> f32 {
        smoothstep(self.elytra_blend)
    }

    fn smooth_ladder_blend(&self) -> f32 {
        smoothstep(self.ladder_blend)
    }

    fn smooth_map_view_blend(&self) -> f32 {
        smoothstep(self.map_view_blend)
    }

    fn smooth_ranged_blend(&self) -> f32 {
        smoothstep(self.ranged_blend)
    }

    fn smooth_passenger_blend(&self) -> f32 {
        smoothstep(self.passenger_blend)
    }

    fn profile_horizontal(
        &self,
        profile: Option<CameraProfile>,
        settings: &ShoulderCameraSettings,
    ) -> f32 {
        profile
            .filter(CameraProfile::is_complete)
            .map_or(settings.horizontal, |p| p.horizontal)
    }

    fn profile_vertical(
        &self,
        profile: Option<CameraProfile>,
        settings: &ShoulderCameraSettings,
    ) -> f32 {
        profile
            .filter(CameraProfile::is_complete)
            .map_or(settings.vertical, |p| p.vertical)
    }

    fn profile_depth(
        &self,
        profile: Option<CameraProfile>,
        settings: &ShoulderCameraSettings,
    ) -> f32 {
        profile
            .filter(CameraProfile::is_complete)
            .map_or(settings.depth, |p| p.depth)
    }

    fn profile_fov(
        &self,
        profile: Option<CameraProfile>,
        settings: &ShoulderCameraSettings,
    ) -> f32 {
        profile
            .filter(CameraProfile::is_complete)
            .map_or(settings.fov, |p| p.fov)
    }

    fn response(&self, profile: Option<CameraProfile>) -> f32 {
        profile
            .filter(CameraProfile::is_complete)
            .map_or(DEFAULT_PROFILE_RESPONSE, |p| p.response)
    }

    /// `toggle`: flips the camera and reports the message to show. The glue
    /// persists `settings` and applies the returned free-camera restore.
    pub fn toggle(
        &mut self,
        settings: &mut ShoulderCameraSettings,
    ) -> (CameraMessage, Option<(f32, f32)>) {
        self.vanilla_perspective_override = false;
        self.hide_local_player_until = None;
        let engaging = !self.requested;
        let mut restore = None;
        if !engaging {
            restore = self.stop_free_camera(true);
        }
        if engaging {
            if !self.activated {
                self.activation_blend = 0.0;
            }
            self.activated = true;
            self.capture_transition_target(false, settings);
        } else {
            self.capture_transition_target(true, settings);
        }
        self.requested = engaging;
        settings.enabled = engaging;
        (
            if engaging {
                CameraMessage::CameraEnabled
            } else {
                CameraMessage::CameraDisabled
            },
            restore,
        )
    }

    /// `onVanillaPerspectiveChange`.
    pub fn on_vanilla_perspective_change(&mut self) {
        self.vanilla_perspective_override = true;
        self.hide_local_player_until = None;
        let _ = self.stop_free_camera(true);
        self.activated = false;
        self.requested = false;
        self.activation_blend = 0.0;
        self.spyglass_takeover = false;
        self.reset_dynamic_transitions();
    }

    /// `setEnabledFromEditor`.
    pub fn set_enabled_from_editor(
        &mut self,
        enabled: bool,
        settings: &mut ShoulderCameraSettings,
    ) -> Option<(f32, f32)> {
        self.vanilla_perspective_override = false;
        self.hide_local_player_until = None;
        let mut restore = None;
        if !enabled {
            restore = self.stop_free_camera(true);
        }
        if enabled {
            if !self.activated {
                self.activation_blend = 0.0;
            }
            self.activated = true;
            self.capture_transition_target(false, settings);
        } else if self.activated {
            self.capture_transition_target(true, settings);
        }
        self.requested = enabled;
        settings.enabled = enabled;
        restore
    }

    /// `captureTransitionTarget`: while disengaging, the camera eases towards
    /// where it is now; while engaging, towards the settings.
    fn capture_transition_target(&mut self, exiting: bool, settings: &ShoulderCameraSettings) {
        self.transition_horizontal = if exiting {
            self.contextual_horizontal(self.smooth_proximity_blend(), settings)
        } else {
            settings.horizontal
        };
        self.transition_vertical = if exiting {
            self.contextual_vertical(self.smooth_proximity_blend(), settings)
        } else {
            settings.vertical
        };
        self.transition_depth = if exiting {
            self.contextual_depth(self.smooth_proximity_blend(), settings)
        } else {
            settings.depth
        };
        self.transition_fov = if exiting {
            self.contextual_fov(self.smooth_proximity_blend(), settings)
        } else {
            settings.fov
        };
    }

    /// `toggleFreeCamera`: `player_yaw` and `player_pitch` are the player's
    /// rotation to remember; the restore pair goes back to the player.
    pub fn toggle_free_camera(
        &mut self,
        player_yaw: f32,
        player_pitch: f32,
    ) -> (CameraMessage, Option<(f32, f32)>) {
        if !self.is_active() {
            return (CameraMessage::FreeCameraRequiresCustom, None);
        }
        let mut restore = None;
        if self.free_camera {
            restore = self.stop_free_camera(true);
        } else {
            self.free_camera = true;
            self.free_camera_yaw = player_yaw;
            self.free_camera_pitch = player_pitch;
        }
        (
            if self.free_camera {
                CameraMessage::FreeCameraEnabled
            } else {
                CameraMessage::FreeCameraDisabled
            },
            restore,
        )
    }

    /// `turnFreeCamera`.
    pub fn turn_free_camera(&mut self, dx: f64, dy: f64) -> bool {
        if !self.free_camera {
            return false;
        }
        if !self.is_active() {
            self.free_camera = false;
            return false;
        }
        self.free_camera_yaw += dx as f32 * FREE_CAMERA_TURN;
        self.free_camera_pitch = clamp(
            self.free_camera_pitch + dy as f32 * FREE_CAMERA_TURN,
            -90.0,
            90.0,
        );
        true
    }

    /// `isFreeCameraActive`.
    pub fn is_free_camera_active(&self) -> bool {
        self.free_camera && self.is_active()
    }

    /// The free camera's remembered yaw.
    pub fn free_camera_yaw(&self) -> f32 {
        self.free_camera_yaw
    }

    /// The free camera's remembered pitch.
    pub fn free_camera_pitch(&self) -> f32 {
        self.free_camera_pitch
    }

    /// `stopFreeCamera`: returns the rotation to put back on the player.
    pub fn stop_free_camera(&mut self, restore: bool) -> Option<(f32, f32)> {
        if self.free_camera {
            let pair = (self.free_camera_yaw, self.free_camera_pitch);
            if restore {
                self.free_camera = false;
                return Some(pair);
            }
            self.free_camera = false;
        }
        None
    }

    /// `resumeSavedCamera`: a third-person camera left over from before the
    /// mod loaded re-engages the shoulder camera.
    pub fn resume_saved_camera(&mut self, settings: &ShoulderCameraSettings) {
        if !self.vanilla_perspective_override
            && !self.activated
            && !self.spyglass_takeover
            && settings.enabled
            && self.third_person
        {
            self.activation_blend = 0.0;
            self.activated = true;
            self.requested = true;
            self.capture_transition_target(false, settings);
        }
    }

    /// The camera this frame, from the eye `eye` the vanilla camera sits at,
    /// the player's `yaw` and `pitch`, and the settings: the effective
    /// offsets, scaled by the activation blend and pulled in by the wall
    /// recovery.
    pub fn compute_pose(
        &self,
        eye: [f64; 3],
        yaw: f32,
        pitch: f32,
        settings: &ShoulderCameraSettings,
    ) -> CameraPose {
        let weight = clamp(self.activation_blend, 0.0, 1.0);
        let horizontal = self.effective_horizontal(settings) as f64 * weight as f64;
        let vertical = self.effective_vertical(settings) as f64 * weight as f64;
        let depth = self.effective_depth(settings) as f64
            * weight as f64
            * self.collision.fraction() as f64;

        let (sin_yaw, cos_yaw) = (
            (yaw as f64).to_radians().sin(),
            (yaw as f64).to_radians().cos(),
        );
        // Forward at yaw 0 is +Z, right is +X; the camera sits back along
        // forward and out along right.
        let forward = [-sin_yaw, 0.0, cos_yaw];
        let right = [cos_yaw, 0.0, sin_yaw];
        let position = [
            eye[0] + right[0] * horizontal - forward[0] * depth,
            eye[1] + vertical,
            eye[2] + right[2] * horizontal - forward[2] * depth,
        ];
        let pose_yaw = yaw + settings.rotation_yaw * weight;
        let pose_pitch = (pitch + settings.rotation_pitch * weight).clamp(-90.0, 90.0);
        CameraPose {
            position,
            yaw: pose_yaw,
            pitch: pose_pitch,
            weight,
        }
    }

    /// The wall recovery.
    pub fn collision(&self) -> &CameraCollisionRecovery {
        &self.collision
    }

    /// The wall recovery, to change it.
    pub fn collision_mut(&mut self) -> &mut CameraCollisionRecovery {
        &mut self.collision
    }

    /// The map view camera the blends read.
    pub fn map_view(&self) -> MapViewCamera {
        self.map_view
    }

    /// Replaces the map view camera (the engine loaded a new one).
    pub fn set_map_view_camera(&mut self, map_view: MapViewCamera) {
        self.map_view = map_view;
    }
}

// ---------------------------------------------------------------------------
// The frame the hooks call
// ---------------------------------------------------------------------------

#[derive(Default)]
struct State {
    /// Read on the first frame, then kept.
    settings: Option<ShoulderCameraSettings>,
    /// The controller's state, while a camera is on.
    controller: Option<ShoulderCameraController>,
    /// When `frame_dt` last ran, for the frame's delta.
    last_frame: Option<Instant>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> ShoulderCameraSettings {
    STATE.with_borrow_mut(|s| {
        *s.settings
            .get_or_insert_with(|| ShoulderCameraSettings::read(get))
    })
}

/// Replaces the settings (the settings page or a key changed them).
pub fn set_settings(settings: ShoulderCameraSettings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

/// Runs `f` on the controller, creating it on the first call.
pub fn controller<T>(f: impl FnOnce(&mut ShoulderCameraController) -> T) -> T {
    STATE.with_borrow_mut(|s| {
        f(s.controller
            .get_or_insert_with(ShoulderCameraController::new))
    })
}

/// Forgets the controller: the next frame starts from vanilla's camera again.
pub fn reset() {
    STATE.with_borrow_mut(|s| s.controller = None);
}

/// Seconds since the last call, for the blend's frame step: the first call and
/// a stalled frame both read `1/20`, and no step exceeds the transitions'
/// `0.1` clamp.
pub fn frame_dt() -> f32 {
    STATE.with_borrow_mut(|s| {
        let now = Instant::now();
        let dt = s
            .last_frame
            .map_or(1.0 / 20.0, |last| now.duration_since(last).as_secs_f32());
        s.last_frame = Some(now);
        dt.clamp(0.0, 0.1)
    })
}

/// Advances the camera by `dt` seconds with `blocked` the unblocked fraction
/// the game found, and returns the pose to draw from, or `None` to keep
/// vanilla's.
pub fn frame_pose(
    eye: [f64; 3],
    yaw: f32,
    pitch: f32,
    dt: f32,
    blocked: f32,
    settings: &ShoulderCameraSettings,
) -> Option<CameraPose> {
    if !settings.enabled {
        reset();
        return None;
    }
    STATE.with_borrow_mut(|state| {
        let controller = state
            .controller
            .get_or_insert_with(ShoulderCameraController::new);
        controller.resume_saved_camera(settings);
        controller.update_activation_transition(dt, settings);
        controller.collision_mut().update(blocked as f64, dt);
        if controller.activation_blend() <= 0.0 {
            None
        } else {
            Some(controller.compute_pose(eye, yaw, pitch, settings))
        }
    })
}

/// The camera key: flips the controller and the settings, and reports the
/// message to show and the free-camera rotation to put back on the player.
pub fn toggle_camera(
    get: impl Fn(&str) -> Option<String>,
) -> (ShoulderCameraSettings, CameraMessage, Option<(f32, f32)>) {
    STATE.with_borrow_mut(|s| {
        let settings = s
            .settings
            .get_or_insert_with(|| ShoulderCameraSettings::read(get));
        let controller = s
            .controller
            .get_or_insert_with(ShoulderCameraController::new);
        let (message, restore) = controller.toggle(settings);
        (*settings, message, restore)
    })
}

/// The orbit key: toggles the free camera; the restore pair goes back on the
/// player.
pub fn toggle_orbit(
    get: impl Fn(&str) -> Option<String>,
    player_yaw: f32,
    player_pitch: f32,
) -> (CameraMessage, Option<(f32, f32)>) {
    STATE.with_borrow_mut(|s| {
        s.settings
            .get_or_insert_with(|| ShoulderCameraSettings::read(get));
        let controller = s
            .controller
            .get_or_insert_with(ShoulderCameraController::new);
        controller.toggle_free_camera(player_yaw, player_pitch)
    })
}

/// A free-camera mouse turn; `false` when the free camera is not running.
pub fn turn_orbit(dx: f64, dy: f64) -> bool {
    controller(|c| c.turn_free_camera(dx, dy))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_values<'a>(values: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn defaults_are_usable_and_complete() {
        let settings = ShoulderCameraSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.horizontal, 1.17);
        assert_eq!(settings.vertical, -0.11);
        assert_eq!(settings.depth, 2.5);
        assert_eq!(settings.fov, 70.0);
        assert_eq!(settings.rotation_yaw, 0.0);
        assert_eq!(settings.rotation_pitch, 0.0);
        assert!(settings.fence_camera);
        assert!(settings.ladder_camera);
        assert!(settings.proximity_camera);
        assert!(settings.elytra_camera);
        assert!(settings.map_camera);
        assert!(settings.bow_and_crossbow_camera_fix);
        assert!(settings.spear_and_trident_camera_fix);
        assert!(settings.ladder_body_lock);
    }

    #[test]
    fn the_setters_clamp_like_the_java_config() {
        let mut settings = ShoulderCameraSettings::default();
        settings.set_horizontal(5.0);
        assert_eq!(settings.horizontal, 2.0);
        settings.set_vertical(-4.0);
        assert_eq!(settings.vertical, -1.5);
        settings.set_depth(0.2);
        assert_eq!(settings.depth, 1.0);
        settings.set_fov(200.0);
        assert_eq!(settings.fov, 110.0);
        settings.set_rotation_yaw(45.0);
        assert_eq!(settings.rotation_yaw, 30.0);
        settings.set_rotation_pitch(-45.0);
        assert_eq!(settings.rotation_pitch, -30.0);
    }

    #[test]
    fn the_button_indices_name_the_settings_they_change() {
        assert_eq!(ENABLED_INDEX, 0);
        assert_eq!(HORIZONTAL_INDEX, 1);
        assert_eq!(VERTICAL_INDEX, 2);
        assert_eq!(DEPTH_INDEX, 3);
        assert_eq!(FOV_INDEX, 4);
        assert_eq!(YAW_INDEX, 5);
        assert_eq!(PITCH_INDEX, 6);
        assert_eq!(FENCE_INDEX, 7);
        assert_eq!(LADDER_INDEX, 8);
        assert_eq!(PROXIMITY_INDEX, 9);
        assert_eq!(ELYTRA_INDEX, 10);
        assert_eq!(MAP_INDEX, 11);
        assert_eq!(BOW_AND_CROSSBOW_INDEX, 12);
        assert_eq!(SPEAR_AND_TRIDENT_INDEX, 13);
        assert_eq!(LADDER_BODY_LOCK_INDEX, 14);
        assert_eq!(BUTTONS, 15);
    }

    #[test]
    fn read_keeps_defaults_when_a_key_is_missing_and_clamps_numbers() {
        let settings = ShoulderCameraSettings::read(stored_values(&[
            (HORIZONTAL, "9"),
            (DEPTH, "0.5"),
            (FENCE_CAMERA, "false"),
            (LADDER_BODY_LOCK, "false"),
        ]));
        assert_eq!(settings.horizontal, 2.0, "9 clamps to 2");
        assert_eq!(settings.depth, 1.0, "0.5 clamps up to 1");
        assert!(!settings.fence_camera);
        assert!(!settings.ladder_body_lock);
        assert_eq!(settings.vertical, DEFAULT_VERTICAL, "missing keeps default");
        assert!(settings.elytra_camera, "missing keeps the default true");
    }

    #[test]
    fn click_cycles_every_button_and_wraps() {
        let mut settings = ShoulderCameraSettings::default();
        settings.click(HORIZONTAL_INDEX);
        assert_eq!(settings.horizontal, 1.5, "1.17 is a step, so the next one");
        settings.click(FOV_INDEX);
        assert_eq!(settings.fov, 85.0, "70 is a step, so the next one");
        settings.click(FOV_INDEX);
        assert_eq!(settings.fov, 100.0);
        settings.click(FOV_INDEX);
        assert_eq!(settings.fov, 50.0, "wrapped past the end");
        settings.click(LADDER_BODY_LOCK_INDEX);
        assert!(!settings.ladder_body_lock);
        settings.click(BUTTONS + 3);
        assert!(!settings.ladder_body_lock, "past the page does nothing");
    }

    #[test]
    fn settings_round_trip_through_stored() {
        let mut settings = ShoulderCameraSettings::default();
        settings.click(ENABLED_INDEX);
        settings.set_horizontal(0.75);
        settings.set_rotation_yaw(12.0);
        let stored = settings.stored();
        assert_eq!(stored.len(), BUTTONS);
        let pairs: Vec<(&str, &str)> = stored.iter().map(|(k, v)| (&**k, &**v)).collect();
        let reread = ShoulderCameraSettings::read(stored_values(&pairs));
        assert_eq!(reread, settings);
        assert_eq!(stored[5].0, ROTATION_YAW);
        assert_eq!(stored[12].0, BOW_AND_CROSSBOW_CAMERA_FIX);
    }

    #[test]
    fn collision_pulls_in_at_once_and_recovers_exponentially() {
        let mut recovery = CameraCollisionRecovery::new();
        assert_eq!(recovery.fraction(), 1.0);
        // A shorter arm is taken at once.
        assert_eq!(recovery.update(0.25, 0.05), 0.25);
        // A longer arm eases out: one 0.1 s step closes 1 - e^-0.7 of the gap.
        let step = 1.0 - (-7.0f64 * 0.1).exp();
        let grown = recovery.update(1.0, 0.1);
        let expected = 0.25 + (1.0 - 0.25) * step;
        assert!((grown - expected).abs() < 1e-4, "{grown} vs {expected}");
        // A broken target reads as 0.0 and pulls the camera all the way in.
        recovery.reset();
        assert_eq!(recovery.update(f64::NAN, 0.05), 0.0);
    }

    #[test]
    fn the_avatar_is_skipped_up_close_and_faded_inside_the_band() {
        let width = 0.6;
        let height = 1.8;
        let eye = 1.62;
        // Inside 0.75 widths of the camera: skipped outright.
        let close = avatar_camera_visibility(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -0.2],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            width,
            height,
            eye,
            false,
        );
        assert!(close.skip_avatar);
        // Far beyond 2.5 widths: fully drawn.
        let far = avatar_camera_visibility(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -5.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            width,
            height,
            eye,
            false,
        );
        assert!(!far.skip_avatar);
        assert_eq!(far.alpha, 1.0);
        // In between, straight down the camera's axis: the distance fade
        // only, from the 0.15 angle floor towards 1.
        let mid = avatar_camera_visibility(
            [0.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            width,
            height,
            eye,
            false,
        );
        assert!(!mid.skip_avatar);
        assert!((mid.alpha - 1.0).abs() > 1.0e-3);
        assert!(mid.alpha > 0.15);
        // Just outside the skip radius, dead centre: the 0.15 angle floor.
        let aside = avatar_camera_visibility(
            [0.0, 0.0, 0.0],
            [0.001, 0.0, -0.46],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            width,
            height,
            eye,
            false,
        );
        assert!((aside.alpha - 0.15).abs() < 0.01, "{}", aside.alpha);
    }

    #[test]
    fn unproject_aims_through_the_middle_of_the_picture() {
        // A view that translates the world by (1, 2, 3) and a projection
        // whose clip w is `1 - z`: the ray through NDC (0, 0) starts at
        // (-1, -2, -2) and runs straight down -Z.
        let view: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            1.0, 2.0, 3.0, 1.0, // translation column
        ];
        let projection: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, -1.0, // clip w = 1 - z
            0.0, 0.0, 0.0, 1.0,
        ];
        let ray = CenterAimRay::unproject(&view, &projection).expect("a usable ray");
        for (got, want) in ray.origin.iter().zip([-1.0, -2.0, -2.0]) {
            assert!((got - want).abs() < 1e-5, "{:?} vs {:?}", ray.origin, want);
        }
        assert!((ray.direction[0]).abs() < 1e-6);
        assert!((ray.direction[1]).abs() < 1e-6);
        assert!((ray.direction[2] + 1.0).abs() < 1e-6);
        let through = ray.at(3.0);
        assert!((through[2] + 5.0).abs() < 1e-5);
        // A singular matrix has no ray.
        let singular: [f32; 16] = [0.0; 16];
        assert!(CenterAimRay::unproject(&singular, &projection).is_none());
    }

    #[test]
    fn the_activation_blend_rises_and_falls_at_its_rates() {
        let mut settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        assert!(!controller.is_active());
        let (message, _) = controller.toggle(&mut settings);
        assert_eq!(message, CameraMessage::CameraEnabled);
        assert!(settings.enabled);
        assert!(controller.is_active());
        // 0.45 s of a 0.9 s engagement is half way.
        for _ in 0..5 {
            controller.update_activation_transition(0.09, &settings);
        }
        assert!((controller.activation_blend() - 0.5).abs() < 1e-4);
        // Back out at the slower-looking but shorter exit rate.
        let (message, _) = controller.toggle(&mut settings);
        assert_eq!(message, CameraMessage::CameraDisabled);
        assert!(!settings.enabled);
        for _ in 0..4 {
            controller.update_activation_transition(0.09, &settings);
        }
        assert!(
            (controller.activation_blend()).abs() < 1e-3,
            "0.36 s of the 0.72 s exit empties the 0.5 blend"
        );
        // Mid-exit with the depth still low, the player is hidden.
        assert!(controller.should_hide_local_player_during_transition(&settings, Instant::now()));
        // Settled, the frame hands back to first person.
        for _ in 0..20 {
            controller.update_activation_transition(0.1, &settings);
        }
        controller.finish_activation_frame(Instant::now());
        assert!(!controller.is_active());
        assert!(!controller.should_hide_local_player_during_transition(&settings, Instant::now()));
    }

    #[test]
    fn a_profile_blends_the_situation_camera_over_the_settings() {
        let settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        controller.activated = true;
        controller.requested = true;
        controller.activation_blend = 1.0;
        let profile = CameraProfile {
            horizontal: 2.0,
            vertical: 1.0,
            depth: 4.0,
            fov: 90.0,
            response: 10.0,
        };
        // The fence latch needs a complete profile and the setting on.
        for _ in 0..30 {
            controller.update_fence_transition(false, true, 0.1, Some(profile));
        }
        assert!((controller.fence_blend - 1.0).abs() < 1e-3);
        // A complete profile replaces the held one; an incomplete one does not.
        let broken = CameraProfile {
            response: f32::NAN,
            ..profile
        };
        controller.update_fence_transition(true, false, 0.1, Some(broken));
        assert_eq!(
            controller.profile_horizontal(controller.fence_profile, &settings),
            2.0
        );
        // Settled, the effective offset is the profile's.
        for _ in 0..30 {
            controller.update_fence_transition(false, true, 0.1, None);
        }
        assert!((controller.effective_horizontal(&settings) - 2.0).abs() < 1e-3);
        assert!((controller.effective_depth(&settings) - 4.0).abs() < 1e-3);
        assert!((controller.effective_fov(&settings, 70.0) - 90.0).abs() < 1e-3);
    }

    #[test]
    fn the_proximity_camera_latches_and_holds_while_mining() {
        let mut controller = ShoulderCameraController::new();
        controller.activated = true;
        controller.requested = true;
        controller.activation_blend = 1.0;
        let profile = CameraProfile {
            horizontal: 0.0,
            vertical: 0.5,
            depth: 1.0,
            fov: 70.0,
            response: 8.0,
        };
        // Close to a wall: latched.
        controller.update_proximity_transition(
            0.4,
            false,
            false,
            0.1,
            Some(profile),
            0.5,
            1.0,
            0.3,
            6.0,
            3.0,
        );
        assert!(controller.proximity_latched);
        // Walk away: it releases and eases back.
        controller
            .update_proximity_transition(3.0, false, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
        assert!(!controller.proximity_latched);
        // Mining within the mining distance starts the hold.
        controller
            .update_proximity_transition(0.9, true, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
        assert!(controller.proximity_latched);
        // The hold survives walking away for its seconds, then expires.
        controller
            .update_proximity_transition(3.0, false, false, 0.1, None, 0.5, 1.0, 0.3, 6.0, 3.0);
        assert!(controller.proximity_latched, "hold still running");
        controller
            .update_proximity_transition(3.0, false, false, 0.2, None, 0.5, 1.0, 0.3, 6.0, 3.0);
        assert!(!controller.proximity_latched, "hold expired");
    }

    #[test]
    fn the_free_camera_turns_and_restores() {
        let mut controller = ShoulderCameraController::new();
        let (message, _) = controller.toggle_free_camera(20.0, 5.0);
        assert_eq!(message, CameraMessage::FreeCameraRequiresCustom);
        controller.activated = true;
        controller.requested = true;
        let (message, restore) = controller.toggle_free_camera(20.0, 5.0);
        assert_eq!(message, CameraMessage::FreeCameraEnabled);
        assert_eq!(restore, None);
        assert!(controller.is_free_camera_active());
        // 10 pixels right and 1000 up: yaw moves 1.5, pitch clamps at 90.
        assert!(controller.turn_free_camera(10.0, 1000.0));
        assert!((controller.free_camera_yaw() - 21.5).abs() < 1e-4);
        assert_eq!(controller.free_camera_pitch(), 90.0);
        let (message, restore) = controller.toggle_free_camera(0.0, 0.0);
        assert_eq!(message, CameraMessage::FreeCameraDisabled);
        assert_eq!(restore, Some((21.5, 90.0)));
        assert!(!controller.is_free_camera_active());
    }

    #[test]
    fn a_disengaged_camera_leaves_the_eye_alone() {
        let settings = ShoulderCameraSettings::default();
        let controller = ShoulderCameraController::new();
        let pose = controller.compute_pose([1.0, 2.0, 3.0], 0.0, 0.0, &settings);
        assert_eq!(pose.position, [1.0, 2.0, 3.0]);
        assert_eq!(pose.weight, 0.0);
    }

    #[test]
    fn an_engaged_camera_offsets_the_eye_by_the_settings() {
        let mut settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        controller.toggle(&mut settings);
        controller.activation_blend = 1.0;
        controller.resume_saved_camera(&settings);
        let pose = controller.compute_pose([1.0, 2.0, 3.0], 0.0, 0.0, &settings);
        // Facing +Z, the camera sits 1.17 right, 0.11 down... no: vertical
        // -0.11 lowers it, and 2.5 back along -Z.
        assert!((pose.position[0] - (1.0 + 1.17)).abs() < 1e-6);
        assert!((pose.position[1] - (2.0 - 0.11)).abs() < 1e-6);
        assert!((pose.position[2] - (3.0 - 2.5)).abs() < 1e-6);
        assert_eq!(pose.weight, 1.0);
    }

    #[test]
    fn the_aim_ray_leaves_the_camera_along_its_look() {
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            yaw: 90.0,
            pitch: 0.0,
            weight: 1.0,
        };
        let ray = pose.aim_ray();
        // Yaw 90 faces -X.
        assert!((ray.direction[0] + 1.0).abs() < 1e-9);
        assert!((ray.direction[2]).abs() < 1e-9);
        let at = ray.at(4.0);
        assert!((at[0] + 4.0).abs() < 1e-9);
        assert!((at[1] - 64.0).abs() < 1e-9);
    }

    #[test]
    fn the_frame_keeps_no_camera_while_it_is_off() {
        let settings = ShoulderCameraSettings {
            enabled: false,
            ..ShoulderCameraSettings::default()
        };
        STATE.with_borrow_mut(|s| *s = State::default());
        assert!(frame_pose([0.0; 3], 0.0, 0.0, 0.05, 1.0, &settings).is_none());
        STATE.with_borrow_mut(|s| *s = State::default());
    }

    #[test]
    fn frame_dt_is_bounded() {
        STATE.with_borrow_mut(|s| *s = State::default());
        let first = frame_dt();
        assert!(
            (first - 1.0 / 20.0).abs() < 1e-6,
            "the first frame reads 1/20"
        );
        let second = frame_dt();
        assert!(second <= 0.1, "no step exceeds the transitions' clamp");
        STATE.with_borrow_mut(|s| *s = State::default());
    }

    #[test]
    fn the_spyglass_takes_the_camera_and_hands_it_back() {
        let mut settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        controller.toggle(&mut settings);
        controller.update_spyglass_camera_transition(true, &settings);
        assert!(controller.spyglass_takeover);
        assert!(!controller.requested);
        controller.update_spyglass_camera_transition(false, &settings);
        assert!(!controller.spyglass_takeover);
        assert!(controller.requested);
        assert!(controller.activated);
    }

    #[test]
    fn a_vanilla_perspective_change_drops_everything() {
        let mut settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        controller.toggle(&mut settings);
        controller.update_fence_transition(
            false,
            true,
            0.1,
            Some(CameraProfile {
                horizontal: 2.0,
                vertical: 1.0,
                depth: 4.0,
                fov: 90.0,
                response: 10.0,
            }),
        );
        controller.on_vanilla_perspective_change();
        assert!(!controller.activated);
        assert!(!controller.requested);
        assert_eq!(controller.activation_blend(), 0.0);
        assert_eq!(controller.fence_blend, 0.0);
        assert!(controller.fence_profile.is_none());
    }

    #[test]
    fn a_saved_third_person_camera_re_engages() {
        let settings = ShoulderCameraSettings::default();
        let mut controller = ShoulderCameraController::new();
        controller.resume_saved_camera(&settings);
        assert!(controller.activated);
        assert!(controller.requested);
        // The settings off, it stays off.
        let mut off = settings;
        off.enabled = false;
        let mut controller = ShoulderCameraController::new();
        controller.resume_saved_camera(&off);
        assert!(!controller.activated);
    }
}

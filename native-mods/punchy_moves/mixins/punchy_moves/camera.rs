//! Punchy Moves' shoulder camera: the settings page, the collision recovery,
//! the blend that eases the camera out and back in, the side swap between the
//! right and left shoulder, and the centre-screen aim ray.
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
use std::time::Instant;

// ---------------------------------------------------------------------------
// Keys and stored setting names
// ---------------------------------------------------------------------------

/// The key that toggles the shoulder camera, and its default: unbound until
/// the player picks one on Options > Mods.
pub const TOGGLE_CAMERA: &str = "camera";
pub const DEFAULT_TOGGLE_CAMERA: &str = "key.keyboard.unknown";
/// The key that toggles the orbit (free) camera, unbound by default too.
pub const TOGGLE_ORBIT: &str = "orbit";
pub const DEFAULT_TOGGLE_ORBIT: &str = "key.keyboard.unknown";

/// The stored setting names, one per field, in the settings page's order.
pub const ENABLED: &str = "enabled";
pub const HORIZONTAL: &str = "horizontal";
pub const VERTICAL: &str = "vertical";
pub const DEPTH: &str = "depth";
pub const FOV: &str = "fov";
pub const ROTATION_YAW: &str = "rotation_yaw";
pub const ROTATION_PITCH: &str = "rotation_pitch";
pub const FENCE_CAMERA: &str = "fence_camera";
pub const LADDER_CAMERA: &str = "ladder_camera";
pub const PROXIMITY_CAMERA: &str = "proximity_camera";
pub const ELYTRA_CAMERA: &str = "elytra_camera";
pub const MAP_CAMERA: &str = "map_camera";
pub const RANGED_CAMERA_FIX: &str = "ranged_camera_fix";
pub const FREE_CAMERA: &str = "free_camera";

/// How many buttons the settings page has.
pub const BUTTONS: usize = 14;
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
pub const RANGED_INDEX: usize = 12;
pub const FREE_INDEX: usize = 13;

// ---------------------------------------------------------------------------
// ShoulderCameraSettings
// ---------------------------------------------------------------------------

/// The shoulder camera's settings: where the camera sits behind and beside the
/// eye, how much it turns, and in which situations it may engage at all.
///
/// One click on the settings page moves each value to the next step of its
/// table, so the page needs no text entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraSettings {
    /// The shoulder camera may engage.
    pub enabled: bool,
    /// Blocks to the right of the eye (left when the left shoulder is used).
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
    /// Draw the aim ray from the camera, so ranged shots cross the crosshair.
    pub ranged_camera_fix: bool,
    /// Let the mouse orbit the camera without turning the player.
    pub free_camera: bool,
}

/// The steps one click moves each setting to. A value between steps counts as
/// the first.
const HORIZONTAL_STEPS: [f32; 5] = [0.25, 0.45, 0.60, 0.75, 0.95];
const VERTICAL_STEPS: [f32; 4] = [0.0, 0.05, 0.15, 0.30];
const DEPTH_STEPS: [f32; 5] = [1.6, 2.2, 2.8, 3.4, 4.2];
const FOV_STEPS: [f32; 4] = [60.0, 70.0, 80.0, 90.0];
const YAW_STEPS: [f32; 5] = [0.0, 10.0, 20.0, -10.0, -20.0];
const PITCH_STEPS: [f32; 5] = [0.0, 5.0, 10.0, -5.0, -10.0];

impl Default for ShoulderCameraSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            horizontal: 0.60,
            vertical: 0.05,
            depth: 2.8,
            fov: 70.0,
            rotation_yaw: 0.0,
            rotation_pitch: 0.0,
            fence_camera: false,
            ladder_camera: true,
            proximity_camera: true,
            elytra_camera: false,
            map_camera: false,
            ranged_camera_fix: true,
            free_camera: false,
        }
    }
}

/// When the camera is asked to engage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situation {
    /// Ordinary play: nothing special around the player.
    Normal,
    /// Standing on a fence or another narrow-topped block.
    Fence,
    /// Holding a ladder, vine or scaffolding.
    Ladder,
    /// A block sits right behind the eye.
    Wall,
    /// Gliding on an elytra.
    Elytra,
    /// A map is held up.
    Map,
}

impl ShoulderCameraSettings {
    /// The defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the settings from stored values (`get` reads one key); a missing
    /// or unreadable value keeps its default.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Self::default();
        let flag = |key: &str, fallback: bool| get(key).map_or(fallback, |v| v.trim() != "false");
        let number = |key: &str, fallback: f32, min: f32, max: f32| {
            get(key)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(min, max))
                .unwrap_or(fallback)
        };
        Self {
            enabled: flag(ENABLED, true),
            horizontal: number(HORIZONTAL, defaults.horizontal, 0.0, 3.0),
            vertical: number(VERTICAL, defaults.vertical, -1.0, 2.0),
            depth: number(DEPTH, defaults.depth, 0.0, 8.0),
            fov: number(FOV, defaults.fov, 30.0, 130.0),
            rotation_yaw: number(ROTATION_YAW, defaults.rotation_yaw, -180.0, 180.0),
            rotation_pitch: number(ROTATION_PITCH, defaults.rotation_pitch, -90.0, 90.0),
            fence_camera: flag(FENCE_CAMERA, defaults.fence_camera),
            ladder_camera: flag(LADDER_CAMERA, defaults.ladder_camera),
            proximity_camera: flag(PROXIMITY_CAMERA, defaults.proximity_camera),
            elytra_camera: flag(ELYTRA_CAMERA, defaults.elytra_camera),
            map_camera: flag(MAP_CAMERA, defaults.map_camera),
            ranged_camera_fix: flag(RANGED_CAMERA_FIX, defaults.ranged_camera_fix),
            free_camera: flag(FREE_CAMERA, defaults.free_camera),
        }
    }

    /// Whether the camera may engage in `situation`.
    pub fn allows(&self, situation: Situation) -> bool {
        if !self.enabled {
            return false;
        }
        match situation {
            Situation::Normal => true,
            Situation::Fence => self.fence_camera,
            Situation::Ladder => self.ladder_camera,
            Situation::Wall => self.proximity_camera,
            Situation::Elytra => self.elytra_camera,
            Situation::Map => self.map_camera,
        }
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
            format!("Ranged aim fix: {}", on(self.ranged_camera_fix)),
            format!("Orbit camera: {}", on(self.free_camera)),
        ]
    }

    /// A click on button `i` of [`ShoulderCameraSettings::labels`]; an index
    /// past the page does nothing.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => self.horizontal = cycle(&HORIZONTAL_STEPS, self.horizontal),
            2 => self.vertical = cycle(&VERTICAL_STEPS, self.vertical),
            3 => self.depth = cycle(&DEPTH_STEPS, self.depth),
            4 => self.fov = cycle(&FOV_STEPS, self.fov),
            5 => self.rotation_yaw = cycle(&YAW_STEPS, self.rotation_yaw),
            6 => self.rotation_pitch = cycle(&PITCH_STEPS, self.rotation_pitch),
            7 => self.fence_camera = !self.fence_camera,
            8 => self.ladder_camera = !self.ladder_camera,
            9 => self.proximity_camera = !self.proximity_camera,
            10 => self.elytra_camera = !self.elytra_camera,
            11 => self.map_camera = !self.map_camera,
            12 => self.ranged_camera_fix = !self.ranged_camera_fix,
            13 => self.free_camera = !self.free_camera,
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
            (RANGED_CAMERA_FIX, flag(self.ranged_camera_fix)),
            (FREE_CAMERA, flag(self.free_camera)),
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
/// and eases back out when it lengthens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraCollisionRecovery {
    /// How far along the arm the camera may sit, `0.0..=1.0`.
    fraction: f32,
    /// Fraction regained per second while nothing blocks.
    recovery_rate: f32,
}

impl CameraCollisionRecovery {
    /// The recovery speed a fresh camera had: more than a quarter turn a
    /// second, quick enough to feel immediate, slow enough not to snap.
    pub const DEFAULT_RATE: f32 = 3.5;

    /// A camera at full reach.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fraction along the arm the camera sits at, `0.0..=1.0`.
    pub fn fraction(&self) -> f32 {
        self.fraction
    }

    /// Sets the fraction, clamped; used by tests and by a teleport.
    pub fn set_fraction(&mut self, fraction: f32) -> f32 {
        self.fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.fraction
    }

    /// The recovery speed in fraction per second.
    pub fn recovery_rate(&self) -> f32 {
        self.recovery_rate
    }

    /// Sets the recovery speed; a negative or non-finite value becomes `0.0`.
    pub fn set_recovery_rate(&mut self, rate: f32) {
        self.recovery_rate = if rate.is_finite() { rate.max(0.0) } else { 0.0 };
    }

    /// One frame with `blocked` the longest unblocked fraction the game found
    /// (`1.0` when nothing blocks) over `dt` seconds; returns the new fraction.
    ///
    /// A shorter arm is taken at once, so the camera never sits inside the
    /// wall for a frame; a longer one is eased out.
    pub fn update(&mut self, blocked: f32, dt: f32) -> f32 {
        let blocked = if blocked.is_finite() {
            blocked.clamp(0.0, 1.0)
        } else {
            1.0
        };
        if blocked < self.fraction {
            self.fraction = blocked;
            return self.fraction;
        }
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let step = self.recovery_rate * dt;
        let gap = blocked - self.fraction;
        self.fraction = if gap <= step {
            blocked
        } else {
            self.fraction + step
        };
        self.fraction
    }

    /// Back to full reach.
    pub fn reset(&mut self) {
        self.fraction = 1.0;
    }
}

impl Default for CameraCollisionRecovery {
    fn default() -> Self {
        Self {
            fraction: 1.0,
            recovery_rate: Self::DEFAULT_RATE,
        }
    }
}

// ---------------------------------------------------------------------------
// ShoulderCameraController
// ---------------------------------------------------------------------------

/// Which shoulder the camera sits over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Over the right shoulder (the camera is offset to the right).
    Right,
    /// Over the left shoulder.
    Left,
}

impl Profile {
    /// The horizontal offset's sign for this shoulder: `+1` right, `-1` left.
    pub fn side(self) -> f32 {
        match self {
            Profile::Right => 1.0,
            Profile::Left => -1.0,
        }
    }

    /// The other shoulder.
    pub fn opposite(self) -> Profile {
        match self {
            Profile::Right => Profile::Left,
            Profile::Left => Profile::Right,
        }
    }
}

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

/// Below this blend the local avatar is hidden outright.
pub const AVATAR_HIDE_BELOW: f32 = 0.35;
/// At this blend the local avatar is fully drawn.
pub const AVATAR_VISIBLE_AT: f32 = 0.70;
/// How fast the blend moves per second, in either direction.
const BLEND_RATE: f32 = 6.0;
/// How fast a shoulder swap completes, per second.
const PROFILE_RATE: f32 = 5.0;

/// Blends the shoulder camera in and out and swaps the shoulder smoothly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShoulderCameraController {
    /// Whether the shoulder camera is asked for.
    wanted: bool,
    /// How far it has engaged, `0.0..=1.0`.
    blend: f32,
    /// The shoulder being moved to.
    profile: Profile,
    /// The shoulder being moved from; equal to `profile` when settled.
    from_profile: Profile,
    /// How far the swap has gone, `0.0..=1.0`; `1.0` when settled.
    transition: f32,
    /// The wall recovery.
    collision: CameraCollisionRecovery,
}

impl Default for ShoulderCameraController {
    fn default() -> Self {
        Self {
            wanted: false,
            blend: 0.0,
            profile: Profile::Right,
            from_profile: Profile::Right,
            transition: 1.0,
            collision: CameraCollisionRecovery::default(),
        }
    }
}

impl ShoulderCameraController {
    /// A controller resting on vanilla's third-person camera.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the shoulder camera is asked for.
    pub fn wanted(&self) -> bool {
        self.wanted
    }

    /// How far the camera has engaged, `0.0..=1.0`.
    pub fn blend(&self) -> f32 {
        self.blend
    }

    /// Asks the camera in.
    pub fn enter(&mut self) {
        self.wanted = true;
    }

    /// Asks the camera out.
    pub fn exit(&mut self) {
        self.wanted = false;
    }

    /// Flips in and out; returns the new request.
    pub fn toggle(&mut self) -> bool {
        self.wanted = !self.wanted;
        self.wanted
    }

    /// The shoulder the camera is moving to.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// How far a shoulder swap has gone, `0.0..=1.0`; `1.0` when settled.
    pub fn transition(&self) -> f32 {
        self.transition
    }

    /// Whether a shoulder swap is still in progress.
    pub fn swapping(&self) -> bool {
        self.transition < 1.0
    }

    /// Starts a swap to the other shoulder.
    pub fn swap_shoulder(&mut self) -> Profile {
        if self.transition >= 1.0 {
            self.from_profile = self.profile;
            self.profile = self.profile.opposite();
            self.transition = 0.0;
        } else {
            // Mid-swap: the shoulder being passed through becomes the start,
            // so a second press never jumps across the head.
            self.from_profile = self.profile;
            self.profile = self.profile.opposite();
            self.transition = 1.0 - self.transition;
        }
        self.profile
    }

    /// The wall recovery.
    pub fn collision(&self) -> &CameraCollisionRecovery {
        &self.collision
    }

    /// The wall recovery, to change it.
    pub fn collision_mut(&mut self) -> &mut CameraCollisionRecovery {
        &mut self.collision
    }

    /// The horizontal offset's sign this frame: `+1` on the right shoulder,
    /// `-1` on the left, and in between during a swap.
    pub fn side(&self) -> f32 {
        if self.transition >= 1.0 {
            return self.profile.side();
        }
        let from = self.from_profile.side();
        let to = self.profile.side();
        let eased = smoothstep(self.transition.clamp(0.0, 1.0));
        from + (to - from) * eased
    }

    /// One frame over `dt` seconds with `blocked` the unblocked fraction the
    /// game found; advances the blend, a shoulder swap and the wall recovery,
    /// and returns the new blend.
    pub fn update(&mut self, dt: f32, blocked: f32) -> f32 {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let target = if self.wanted { 1.0 } else { 0.0 };
        self.blend = approach(self.blend, target, BLEND_RATE * dt);
        if self.transition < 1.0 {
            self.transition = (self.transition + PROFILE_RATE * dt).min(1.0);
        }
        self.collision.update(blocked, dt);
        self.blend
    }

    /// How drawn the local avatar is, `0.0..=1.0`: hidden while the camera is
    /// close to the eye, fully drawn once the camera has engaged.
    pub fn avatar_alpha(&self) -> f32 {
        let span = AVATAR_VISIBLE_AT - AVATAR_HIDE_BELOW;
        ((self.blend - AVATAR_HIDE_BELOW) / span).clamp(0.0, 1.0)
    }

    /// Whether the local avatar is drawn at all.
    pub fn avatar_visible(&self) -> bool {
        self.blend >= AVATAR_HIDE_BELOW
    }

    /// The camera this frame, from the eye `eye` the vanilla camera sits at,
    /// the player's `yaw` and `pitch`, and the settings.
    pub fn compute_pose(
        &self,
        eye: [f64; 3],
        yaw: f32,
        pitch: f32,
        settings: &ShoulderCameraSettings,
    ) -> CameraPose {
        let weight = self.blend.clamp(0.0, 1.0);
        let side = self.side() as f64;
        let horizontal = settings.horizontal as f64 * side * weight as f64;
        let vertical = settings.vertical as f64 * weight as f64;
        let depth = settings.depth as f64 * weight as f64 * self.collision.fraction() as f64;

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
        let pose_yaw = yaw + settings.rotation_yaw * self.side() * weight;
        let pose_pitch = (pitch + settings.rotation_pitch * weight).clamp(-90.0, 90.0);
        CameraPose {
            position,
            yaw: pose_yaw,
            pitch: pose_pitch,
            weight,
        }
    }
}

/// `a` moved towards `b` by at most `step`.
fn approach(a: f32, b: f32, step: f32) -> f32 {
    if !a.is_finite() {
        return b;
    }
    let step = if step.is_finite() { step.max(0.0) } else { 0.0 };
    let gap = b - a;
    if gap.abs() <= step {
        b
    } else if gap > 0.0 {
        a + step
    } else {
        a - step
    }
}

/// The smooth 0-to-1 curve of a shoulder swap.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// CenterAimRay
// ---------------------------------------------------------------------------

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
// The frame the hooks call
// ---------------------------------------------------------------------------

#[derive(Default)]
struct State {
    /// Read on the first frame, then kept.
    settings: Option<ShoulderCameraSettings>,
    /// The blend and the shoulder, while a camera is on.
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

/// Forgets the blend: the next frame starts from vanilla's camera again.
pub fn reset() {
    STATE.with_borrow_mut(|s| s.controller = None);
}

/// Seconds since the last call, for the blend's frame step: the first call and
/// a stalled frame both read `1/20`, so the camera never jumps.
pub fn frame_dt() -> f32 {
    STATE.with_borrow_mut(|s| {
        let now = Instant::now();
        let dt = s
            .last_frame
            .map_or(1.0 / 20.0, |last| now.duration_since(last).as_secs_f32());
        s.last_frame = Some(now);
        dt.clamp(0.0, 1.0 / 20.0)
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
        controller.enter();
        controller.update(dt, blocked);
        if controller.blend() <= 0.0 {
            None
        } else {
            Some(controller.compute_pose(eye, yaw, pitch, settings))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let values: Vec<(String, String)> = values
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        move |key| {
            values
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn defaults_are_usable_and_complete() {
        let settings = ShoulderCameraSettings::default();
        assert!(settings.enabled);
        assert!(settings.horizontal > 0.0);
        assert!(settings.vertical >= 0.0);
        assert!(settings.depth > 0.0);
        assert!((30.0..=130.0).contains(&settings.fov));
        assert!(settings.proximity_camera);
        assert!(settings.ranged_camera_fix);
        assert!(!settings.free_camera);
        // The page lists one button per setting.
        assert_eq!(settings.labels().len(), BUTTONS);
        assert_eq!(settings.stored().len(), BUTTONS);
    }

    #[test]
    fn the_button_indices_name_the_settings_they_change() {
        // Each index moves exactly the field it is named for.
        let mut settings = ShoulderCameraSettings::default();
        settings.click(ENABLED_INDEX);
        assert!(!settings.enabled);
        settings.click(FENCE_INDEX);
        assert!(settings.fence_camera);
        settings.click(LADDER_INDEX);
        assert!(!settings.ladder_camera);
        settings.click(PROXIMITY_INDEX);
        assert!(!settings.proximity_camera);
        settings.click(ELYTRA_INDEX);
        assert!(settings.elytra_camera);
        settings.click(MAP_INDEX);
        assert!(settings.map_camera);
        settings.click(RANGED_INDEX);
        assert!(!settings.ranged_camera_fix);
        settings.click(FREE_INDEX);
        assert!(settings.free_camera);

        // The numeric ones move to a new value of their own field.
        for (index, value) in [
            (HORIZONTAL_INDEX, settings.horizontal),
            (VERTICAL_INDEX, settings.vertical),
            (DEPTH_INDEX, settings.depth),
            (FOV_INDEX, settings.fov),
            (YAW_INDEX, settings.rotation_yaw),
            (PITCH_INDEX, settings.rotation_pitch),
        ] {
            let mut stepped = ShoulderCameraSettings::default();
            stepped.click(index);
            let field = match index {
                HORIZONTAL_INDEX => stepped.horizontal,
                VERTICAL_INDEX => stepped.vertical,
                DEPTH_INDEX => stepped.depth,
                FOV_INDEX => stepped.fov,
                YAW_INDEX => stepped.rotation_yaw,
                _ => stepped.rotation_pitch,
            };
            assert_ne!(field, value, "index {} changed nothing", index);
        }

        // Every index is on the page, and each is used once.
        let indices = [
            ENABLED_INDEX,
            HORIZONTAL_INDEX,
            VERTICAL_INDEX,
            DEPTH_INDEX,
            FOV_INDEX,
            YAW_INDEX,
            PITCH_INDEX,
            FENCE_INDEX,
            LADDER_INDEX,
            PROXIMITY_INDEX,
            ELYTRA_INDEX,
            MAP_INDEX,
            RANGED_INDEX,
            FREE_INDEX,
        ];
        let mut seen = [false; BUTTONS];
        for index in indices {
            assert!(index < BUTTONS);
            assert!(!seen[index], "index {} repeats", index);
            seen[index] = true;
        }
        assert!(seen.iter().all(|used| *used));
    }

    #[test]
    fn read_keeps_defaults_when_a_key_is_missing() {
        let defaults = ShoulderCameraSettings::default();
        let read = ShoulderCameraSettings::read(|_| None);
        assert_eq!(read, defaults);

        // A saved value wins; an unreadable or out-of-range one falls back or
        // clamps.
        let read = ShoulderCameraSettings::read(stored(&[
            ("enabled", "false"),
            ("horizontal", "0.95"),
            ("depth", "99"),
            ("fov", "banana"),
            ("free_camera", "true"),
            ("fence_camera", "true"),
        ]));
        assert!(!read.enabled);
        assert_eq!(read.horizontal, 0.95);
        assert_eq!(read.depth, 8.0);
        assert_eq!(read.fov, defaults.fov);
        assert!(read.free_camera);
        assert!(read.fence_camera);
        assert_eq!(read.ladder_camera, defaults.ladder_camera);

        // A NaN never survives.
        let read = ShoulderCameraSettings::read(stored(&[("vertical", "NaN")]));
        assert_eq!(read.vertical, defaults.vertical);
    }

    #[test]
    fn click_cycles_every_button_and_wraps() {
        let mut settings = ShoulderCameraSettings::default();
        let start = settings;

        // Every button click changes something.
        for button in 0..BUTTONS {
            let before = settings;
            settings.click(button);
            assert_ne!(settings, before, "button {} changed nothing", button);
        }

        // A click past the page does nothing.
        let before = settings;
        settings.click(BUTTONS);
        assert_eq!(settings, before);

        // Toing and froing through a cycle returns to where it started.
        let mut cycled = start;
        let steps = HORIZONTAL_STEPS.len();
        for _ in 0..steps {
            cycled.click(1);
        }
        assert_eq!(cycled.horizontal, start.horizontal);

        // Numbers move to the next step, not off the table.
        let mut stepped = start;
        stepped.click(4);
        assert!(FOV_STEPS.contains(&stepped.fov));
        assert_ne!(stepped.fov, start.fov);

        // A value between steps counts as the first.
        let mut off_table = start;
        off_table.fov = 63.0;
        off_table.click(4);
        assert_eq!(off_table.fov, FOV_STEPS[1]);
    }

    #[test]
    fn toggles_and_allows_follow_the_situation() {
        let mut settings = ShoulderCameraSettings::default();
        assert!(settings.allows(Situation::Normal));
        assert!(!settings.allows(Situation::Fence));
        assert!(settings.allows(Situation::Ladder));
        assert!(settings.allows(Situation::Wall));
        assert!(!settings.allows(Situation::Elytra));
        assert!(!settings.allows(Situation::Map));

        settings.click(7);
        settings.click(10);
        assert!(settings.allows(Situation::Fence));
        assert!(settings.allows(Situation::Elytra));

        // Turned off, nothing engages anywhere.
        settings.toggle();
        assert!(!settings.enabled);
        for situation in [
            Situation::Normal,
            Situation::Fence,
            Situation::Ladder,
            Situation::Wall,
            Situation::Elytra,
            Situation::Map,
        ] {
            assert!(!settings.allows(situation), "{:?} engaged", situation);
        }
    }

    #[test]
    fn settings_round_trip_through_stored() {
        let settings = ShoulderCameraSettings {
            fence_camera: true,
            map_camera: true,
            horizontal: 0.75,
            fov: 90.0,
            rotation_pitch: -5.0,
            ..ShoulderCameraSettings::default()
        };

        let pairs: Vec<(&str, String)> = settings.stored().to_vec();
        let values: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        let read = ShoulderCameraSettings::read(stored(&values));
        assert_eq!(read, settings);
    }

    #[test]
    fn collision_pulls_in_at_once_and_recovers_smoothly() {
        let mut recovery = CameraCollisionRecovery::new();
        assert_eq!(recovery.fraction(), 1.0);

        // A wall: the camera is at the wall on the same frame, never behind it.
        assert_eq!(recovery.update(0.4, 1.0 / 20.0), 0.4);
        // A closer wall pulls in again at once.
        assert_eq!(recovery.update(0.25, 1.0 / 20.0), 0.25);

        // Nothing blocking: it eases back out at the recovery rate.
        let rate = recovery.recovery_rate();
        let step = recovery.update(1.0, 0.1);
        assert!((step - (0.25 + rate * 0.1)).abs() < 1e-6);

        // Far enough away it stops exactly at the unblocked fraction.
        assert_eq!(recovery.update(1.0, 10.0), 1.0);

        // Clamping and bad input.
        recovery.set_fraction(-1.0);
        assert_eq!(recovery.fraction(), 0.0);
        recovery.set_fraction(f32::NAN);
        assert_eq!(recovery.fraction(), 1.0);
        recovery.set_recovery_rate(-3.0);
        assert_eq!(recovery.recovery_rate(), 0.0);
        recovery.set_fraction(0.5);
        assert_eq!(recovery.update(f32::NAN, 0.1), 0.5);
        recovery.reset();
        assert_eq!(recovery.fraction(), 1.0);
    }

    #[test]
    fn blend_rises_and_falls_at_its_rate() {
        let mut controller = ShoulderCameraController::new();
        assert_eq!(controller.blend(), 0.0);
        assert!(!controller.wanted());

        controller.enter();
        assert_eq!(controller.update(0.05, 1.0), 0.3);
        assert_eq!(controller.update(0.05, 1.0), 0.6);
        // It never passes 1.
        assert_eq!(controller.update(1.0, 1.0), 1.0);

        controller.exit();
        assert_eq!(controller.update(0.05, 1.0), 0.7);
        assert_eq!(controller.update(1.0, 1.0), 0.0);
        // And never passes 0.
        assert_eq!(controller.update(1.0, 1.0), 0.0);

        // A toggle flips the request and reports it.
        controller.enter();
        assert!(!controller.toggle());
        assert!(controller.toggle());
        assert!(controller.wanted());

        // A NaN frame leaves the blend alone.
        controller.exit();
        let before = controller.blend();
        assert_eq!(controller.update(f32::NAN, 1.0), before);
    }

    #[test]
    fn the_avatar_is_hidden_until_the_camera_has_engaged() {
        let mut controller = ShoulderCameraController::new();
        assert_eq!(controller.avatar_alpha(), 0.0);
        assert!(!controller.avatar_visible());

        controller.enter();
        controller.update(0.05, 1.0); // 0.3
        assert_eq!(controller.avatar_alpha(), 0.0);
        assert!(!controller.avatar_visible());

        controller.update(0.05, 1.0); // 0.6
        let alpha = controller.avatar_alpha();
        assert!(alpha > 0.0 && alpha < 1.0);
        assert!(controller.avatar_visible());

        controller.update(1.0, 1.0);
        assert_eq!(controller.avatar_alpha(), 1.0);
        assert!(controller.avatar_visible());
    }

    #[test]
    fn a_shoulder_swap_crosses_the_middle_and_settles() {
        let mut controller = ShoulderCameraController::new();
        assert_eq!(controller.profile(), Profile::Right);
        assert_eq!(controller.side(), 1.0);
        assert!(!controller.swapping());

        assert_eq!(controller.swap_shoulder(), Profile::Left);
        assert!(controller.swapping());
        assert_eq!(controller.transition(), 0.0);

        // Halfway through, the camera is over the middle of the head.
        controller.update(0.1, 1.0);
        assert!((controller.transition() - 0.5).abs() < 1e-6);
        assert!(controller.side().abs() < 1e-6);

        controller.update(0.1, 1.0);
        assert_eq!(controller.transition(), 1.0);
        assert_eq!(controller.side(), -1.0);
        assert!(!controller.swapping());

        // Pressing again mid-swap turns round from where it is.
        controller.swap_shoulder(); // back to the right
        controller.update(0.1, 1.0); // half way
        let side = controller.side();
        controller.swap_shoulder();
        assert_eq!(controller.profile(), Profile::Left);
        assert_eq!(controller.transition(), 0.5);
        assert!((controller.side() - side).abs() < 1e-6);
    }

    fn settled() -> ShoulderCameraController {
        let mut controller = ShoulderCameraController::new();
        controller.enter();
        controller.update(1.0, 1.0);
        controller.collision_mut().set_fraction(1.0);
        controller
    }

    #[test]
    fn compute_pose_offsets_the_eye_by_the_settings() {
        let settings = ShoulderCameraSettings {
            horizontal: 0.6,
            vertical: 0.1,
            depth: 2.0,
            rotation_yaw: 0.0,
            rotation_pitch: 0.0,
            ..ShoulderCameraSettings::default()
        };
        let controller = settled();
        let eye = [10.0, 64.0, -3.0];

        let pose = controller.compute_pose(eye, 0.0, 0.0, &settings);
        assert_eq!(pose.weight, 1.0);
        // Right of the eye, above it, and back along +Z's opposite.
        assert!((pose.position[0] - 10.6).abs() < 1e-6);
        assert!((pose.position[1] - 64.1).abs() < 1e-6);
        assert!((pose.position[2] - -5.0).abs() < 1e-6);
        assert_eq!(pose.yaw, 0.0);
        assert_eq!(pose.pitch, 0.0);

        // Facing west (yaw 90): right is +Z, forward is -X.
        let pose = controller.compute_pose(eye, 90.0, 0.0, &settings);
        assert!((pose.position[0] - 12.0).abs() < 1e-6);
        assert!((pose.position[2] - -2.4).abs() < 1e-6);

        // The left shoulder mirrors the sideways offset.
        let mut left = controller;
        left.swap_shoulder();
        left.update(1.0, 1.0);
        let pose = left.compute_pose(eye, 0.0, 0.0, &settings);
        assert!((pose.position[0] - 9.4).abs() < 1e-6);
    }

    #[test]
    fn compute_pose_turns_the_camera_and_clamps_the_pitch() {
        let settings = ShoulderCameraSettings {
            horizontal: 0.6,
            vertical: 0.0,
            depth: 2.0,
            rotation_yaw: 15.0,
            rotation_pitch: 10.0,
            ..ShoulderCameraSettings::default()
        };
        let controller = settled();
        let pose = controller.compute_pose([0.0, 0.0, 0.0], 0.0, 0.0, &settings);
        assert_eq!(pose.yaw, 15.0);
        assert_eq!(pose.pitch, 10.0);

        // Looking steeply down, the pitch never leaves vanilla's range.
        let pose = controller.compute_pose([0.0, 0.0, 0.0], 0.0, 88.0, &settings);
        assert_eq!(pose.pitch, 90.0);
    }

    #[test]
    fn compute_pose_follows_the_wall_recovery() {
        let settings = ShoulderCameraSettings {
            horizontal: 0.0,
            vertical: 0.0,
            depth: 2.0,
            ..ShoulderCameraSettings::default()
        };
        let mut controller = settled();
        controller.collision_mut().set_fraction(0.25);
        let pose = controller.compute_pose([0.0, 0.0, 0.0], 0.0, 0.0, &settings);
        // A quarter of the two-block depth.
        assert!((pose.position[2] - -0.5).abs() < 1e-6);
    }

    #[test]
    fn a_disengaged_camera_leaves_the_eye_alone() {
        let settings = ShoulderCameraSettings::default();
        let controller = ShoulderCameraController::new();
        let eye = [1.0, 2.0, 3.0];
        let pose = controller.compute_pose(eye, 40.0, -20.0, &settings);
        assert_eq!(pose.position, eye);
        assert_eq!(pose.yaw, 40.0);
        assert_eq!(pose.pitch, -20.0);
        assert_eq!(pose.weight, 0.0);
    }

    #[test]
    fn the_aim_ray_leaves_the_camera_along_its_look() {
        // Yaw 0 faces +Z; a positive pitch looks down.
        let ray = CenterAimRay::from_look([0.0, 64.0, 0.0], 0.0, 0.0);
        assert!((ray.direction[0]).abs() < 1e-12);
        assert!((ray.direction[1]).abs() < 1e-12);
        assert!((ray.direction[2] - 1.0).abs() < 1e-12);
        assert_eq!(ray.at(3.0), [0.0, 64.0, 3.0]);

        // Yaw 90 faces -X.
        let ray = CenterAimRay::from_look([0.0, 0.0, 0.0], 90.0, 0.0);
        assert!((ray.direction[0] + 1.0).abs() < 1e-12);

        // A 45-degree downward pitch drops as far as it goes forward.
        let ray = CenterAimRay::from_look([0.0, 0.0, 0.0], 0.0, 45.0);
        assert!((ray.direction[1] + ray.direction[2]).abs() < 1e-12);

        // It is a unit vector at any angles.
        for (yaw, pitch) in [(0.0, 0.0), (37.0, -52.0), (-120.0, 89.0)] {
            let direction = look_direction(yaw, pitch);
            let length = (direction[0] * direction[0]
                + direction[1] * direction[1]
                + direction[2] * direction[2])
                .sqrt();
            assert!((length - 1.0).abs() < 1e-9);
        }

        // A pose's ray starts where the camera is.
        let pose = CameraPose {
            position: [4.0, 70.0, -2.0],
            yaw: 0.0,
            pitch: 0.0,
            weight: 1.0,
        };
        let ray = pose.aim_ray();
        assert_eq!(ray.origin, pose.position);
        assert_eq!(CenterAimRay::from_pose(&pose), ray);
    }

    #[test]
    fn the_frame_keeps_no_camera_while_it_is_off() {
        reset();
        let off = ShoulderCameraSettings {
            enabled: false,
            ..ShoulderCameraSettings::default()
        };
        assert_eq!(frame_pose([0.0; 3], 0.0, 0.0, 0.1, 1.0, &off), None);

        // Turned on, the camera eases in and starts answering.
        let on = ShoulderCameraSettings::default();
        assert_eq!(
            frame_pose([0.0; 3], 0.0, 0.0, 0.0, 1.0, &on).map(|p| p.weight),
            None
        );
        let pose = frame_pose([0.0; 3], 0.0, 0.0, 0.1, 1.0, &on).expect("engaged");
        assert!(pose.weight > 0.0);
        assert!(pose.position[2] < 0.0);

        // A later frame keeps the blend, so the camera does not start over.
        let later = frame_pose([0.0; 3], 0.0, 0.0, 0.1, 1.0, &on).expect("engaged");
        assert!(later.weight > pose.weight);

        // Switched off, it forgets the blend and vanilla's camera returns.
        assert_eq!(frame_pose([0.0; 3], 0.0, 0.0, 0.1, 1.0, &off), None);
        reset();
    }

    #[test]
    fn frame_dt_is_bounded() {
        // The first frame, and any stalled one, step by at most a twentieth of
        // a second, so the camera never jumps.
        reset();
        let dt = frame_dt();
        assert!(dt > 0.0 && dt <= 1.0 / 20.0);
        let dt = frame_dt();
        assert!((0.0..=1.0 / 20.0).contains(&dt));
    }
}

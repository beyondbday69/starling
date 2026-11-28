//! Punchy Moves' player animation engine: the humanoid pose it builds, the
//! controllers that feed it, and the per-player state the game's pose hooks
//! read.
//!
//! Pure Rust and std-only, so the tests run without a Horizon build. Nothing
//! here knows about the game: a pose is seven [`PartTransform`]s, and the
//! hooks in the bundle translate them into the game's own part poses
//! (`h_pose.rs`), its whole-model matrix (`h_body.rs`) and the state the
//! models are posed with (`h_pose_start.rs`).
//!
//! Units: rotations are radians in every part (what a model part takes),
//! positions are blocks, and angles the settings or signals speak in are
//! degrees and converted once here.

use crate::library::{Clip, LocomotionInfluence, OverlayMode, Pose, ResolvedOverlay, Vec3};
use std::collections::HashMap;
use std::time::Instant;

use crate::faces::{FaceQuads, FaceRig};
use crate::signals::{BasePoseRetention, MotionSignals};

/// The hand action an emote plays: our emote glue's marker, not a Java type.
/// The Java emote system plays a named clip; the game glue translates this
/// into its own hand swing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemAction {
    /// The hand swings.
    #[default]
    Swing,
    /// The hand holds something up.
    Use,
    /// The hand eats.
    Eat,
    /// The hand charges a draw.
    Charge,
}

impl ItemAction {
    /// The action the signals imply, for the emote the player just played.
    pub fn from_signals(signals: &MotionSignals) -> Self {
        if signals.charging_crossbow || signals.charging_trident {
            ItemAction::Charge
        } else if signals.mining {
            ItemAction::Swing
        } else {
            ItemAction::Use
        }
    }
}

/// The numbers the engine's own controllers run at. The pack's tuning lives
/// in [`crate::signals::tuning`]; these are the engine's defaults.
#[derive(Debug, Clone)]
pub struct EngineTuning {
    /// Blocks of travel per full gait cycle.
    pub stride_distance: f32,
    /// Swing scale of the gait at walking speed.
    pub gait_amplitude: f32,
    /// Weight change per second when easing between poses.
    pub fade_rate: f32,
    /// The turn-in-place settings the controllers read.
    pub turn_settings: TurnSettings,
}

impl Default for EngineTuning {
    fn default() -> Self {
        Self {
            stride_distance: 2.0,
            gait_amplitude: 1.0,
            fade_rate: 6.0,
            turn_settings: TurnSettings::default(),
        }
    }
}

impl EngineTuning {
    /// The engine's defaults.
    pub fn new() -> Self {
        Self::default()
    }
}

/// One input axis clamped into `-1.0..=1.0`; NaN becomes `0.0`.
fn clamp_axis(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-1.0, 1.0)
    }
}

/// The length of the movement input vector, capped at `1.0`.
fn input_length(signals: &MotionSignals) -> f32 {
    let forward = clamp_axis(signals.walk_forward);
    let left = clamp_axis(signals.walk_left);
    (forward * forward + left * left).sqrt().min(1.0)
}

/// Whether the player is in water on or under the surface.
fn swimming(signals: &MotionSignals) -> bool {
    signals.surface_swimming || signals.submerged_swimming
}

/// `current` moved towards `target` by at most `rate * dt`.
fn fade(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
    let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
    let rate = if rate.is_finite() { rate.max(0.0) } else { 0.0 };
    approach(current, target, rate * dt)
}

// ---------------------------------------------------------------------------
// HumanoidPose
// ---------------------------------------------------------------------------

/// The seven parts of the humanoid player model, in draw order.
pub const PART_NAMES: [&str; 7] = [
    "head",
    "body",
    "right_arm",
    "left_arm",
    "right_leg",
    "left_leg",
    "cape",
];

/// One part's transform: a rotation about each model axis and an offset, all
/// added on top of whatever the body already has. A positive `x_rot` swings a
/// limb forward, a positive `y_rot` turns it left, a positive `z_rot` rolls it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PartTransform {
    pub x_rot: f32,
    pub y_rot: f32,
    pub z_rot: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// The scale multipliers the part carries (1.0 keeps the vanilla size).
    pub x_scale: f32,
    pub y_scale: f32,
    pub z_scale: f32,
}

impl PartTransform {
    /// A part that adds nothing.
    pub const IDENTITY: PartTransform = PartTransform {
        x_rot: 0.0,
        y_rot: 0.0,
        z_rot: 0.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        x_scale: 1.0,
        y_scale: 1.0,
        z_scale: 1.0,
    };

    /// This transform blended towards `other` by `t`, clamped to `0.0..=1.0`.
    pub fn lerp(self, other: PartTransform, t: f32) -> PartTransform {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mix = |a: f32, b: f32| a + (b - a) * t;
        PartTransform {
            x_rot: mix(self.x_rot, other.x_rot),
            y_rot: mix(self.y_rot, other.y_rot),
            z_rot: mix(self.z_rot, other.z_rot),
            x: mix(self.x, other.x),
            y: mix(self.y, other.y),
            z: mix(self.z, other.z),
            x_scale: mix(self.x_scale, other.x_scale),
            y_scale: mix(self.y_scale, other.y_scale),
            z_scale: mix(self.z_scale, other.z_scale),
        }
    }

    /// This transform scaled by `k`.
    pub fn scaled(self, k: f32) -> PartTransform {
        PartTransform {
            x_rot: self.x_rot * k,
            y_rot: self.y_rot * k,
            z_rot: self.z_rot * k,
            x: self.x * k,
            y: self.y * k,
            z: self.z * k,
            x_scale: self.x_scale,
            y_scale: self.y_scale,
            z_scale: self.z_scale,
        }
    }

    /// Whether the transform changes nothing at all.
    pub fn is_identity(self) -> bool {
        self == PartTransform::IDENTITY
    }
}

/// A whole humanoid pose: the seven parts plus how much of the animation
/// shows (`blend`) and how much of each bone's base pose is kept
/// (`retention`, see [`BasePoseRetention`]).
#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidPose {
    pub head: PartTransform,
    pub body: PartTransform,
    pub right_arm: PartTransform,
    pub left_arm: PartTransform,
    pub right_leg: PartTransform,
    pub left_leg: PartTransform,
    pub cape: PartTransform,
    /// How much of this pose shows, `0.0..=1.0`.
    pub blend: f32,
    /// How much of the base pose each bone keeps, `0.0..=1.0` per bone.
    pub retention: BasePoseRetention,
}

impl Default for HumanoidPose {
    fn default() -> Self {
        Self {
            head: PartTransform::IDENTITY,
            body: PartTransform::IDENTITY,
            right_arm: PartTransform::IDENTITY,
            left_arm: PartTransform::IDENTITY,
            right_leg: PartTransform::IDENTITY,
            left_leg: PartTransform::IDENTITY,
            cape: PartTransform::IDENTITY,
            blend: 1.0,
            retention: BasePoseRetention::default(),
        }
    }
}

impl HumanoidPose {
    /// Every part adding nothing, at full blend with no retention.
    pub fn identity() -> Self {
        Self::default()
    }

    /// The parts by name, in [`PART_NAMES`] order.
    pub fn parts(&self) -> [(&'static str, PartTransform); 7] {
        [
            ("head", self.head),
            ("body", self.body),
            ("right_arm", self.right_arm),
            ("left_arm", self.left_arm),
            ("right_leg", self.right_leg),
            ("left_leg", self.left_leg),
            ("cape", self.cape),
        ]
    }

    /// One part's transform by name, or `None` when the name is not a part.
    pub fn part(&self, name: &str) -> Option<PartTransform> {
        self.parts()
            .into_iter()
            .find(|(part, _)| *part == name)
            .map(|(_, transform)| transform)
    }

    /// One part's transform by name, to change.
    pub fn part_mut(&mut self, name: &str) -> Option<&mut PartTransform> {
        match name {
            "head" => Some(&mut self.head),
            "body" => Some(&mut self.body),
            "right_arm" => Some(&mut self.right_arm),
            "left_arm" => Some(&mut self.left_arm),
            "right_leg" => Some(&mut self.right_leg),
            "left_leg" => Some(&mut self.left_leg),
            "cape" => Some(&mut self.cape),
            _ => None,
        }
    }

    /// This pose blended towards `next` by `t`: how a partial tick samples
    /// the pose between the last two updates.
    pub fn lerp(&self, next: &HumanoidPose, t: f32) -> HumanoidPose {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        HumanoidPose {
            head: self.head.lerp(next.head, t),
            body: self.body.lerp(next.body, t),
            right_arm: self.right_arm.lerp(next.right_arm, t),
            left_arm: self.left_arm.lerp(next.left_arm, t),
            right_leg: self.right_leg.lerp(next.right_leg, t),
            left_leg: self.left_leg.lerp(next.left_leg, t),
            cape: self.cape.lerp(next.cape, t),
            blend: self.blend + (next.blend - self.blend) * t,
            retention: next.retention.clone(),
        }
    }

    /// This pose laid over `base`: each part moves from the base towards this
    /// pose by `blend * (1 - retention[part])`, so a bone the retention keeps
    /// stays where the base put it.
    pub fn over(&self, base: &HumanoidPose) -> HumanoidPose {
        let blend = if self.blend.is_finite() {
            self.blend.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut out = base.clone();
        for (name, animated) in self.parts() {
            let kept = self.retention.value(name, false);
            let kept = if kept.is_nan() {
                0.0
            } else {
                kept.clamp(0.0, 1.0)
            };
            let amount = blend * (1.0 - kept);
            if let Some(current) = out.part_mut(name) {
                *current = current.lerp(animated, amount);
            }
        }
        out.blend = self.blend;
        out.retention = self.retention.clone();
        out
    }
}

// ---------------------------------------------------------------------------
// LocomotionController
// ---------------------------------------------------------------------------

/// A full walk's speed in blocks a second: what a movement input of `1.0`
/// carries the player along at.
pub const WALK_SPEED: f32 = 4.317;

/// How far a limb swings at full stride, in radians.
pub const LIMB_SWING: f32 = 0.85;

/// The gait: advances the walk cycle with the distance travelled, eases the
/// stride in and out, and counts how long the player has been running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocomotionController {
    /// The walk cycle's phase, a fraction in `0.0..1.0`.
    phase: f32,
    /// The stride amount, `0.0..=1.0`, eased.
    stride: f32,
    /// The fall weight, `0.0..=1.0`, eased.
    lift: f32,
    /// Seconds the player has been running.
    run_seconds: f32,
}

/// One update's gait.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LocomotionFrame {
    /// The walk cycle's phase, degrees in `0.0..360.0`.
    pub phase: f32,
    /// How much the legs swing, `0.0..=1.0`.
    pub stride: f32,
    /// The fall or landing weight, `0.0..=1.0`.
    pub lift: f32,
    /// The movement input's length, `0.0..=1.0`.
    pub speed: f32,
    /// Seconds of running so far.
    pub run_seconds: f32,
}

impl Default for LocomotionController {
    fn default() -> Self {
        Self {
            phase: 0.0,
            stride: 0.0,
            lift: 0.0,
            run_seconds: 0.0,
        }
    }
}

impl LocomotionController {
    pub fn new() -> Self {
        Self::default()
    }

    /// The gait phase, a fraction in `0.0..1.0`.
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// The eased stride amount.
    pub fn stride(&self) -> f32 {
        self.stride
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame from the movement signals.
    pub fn update(
        &mut self,
        signals: &MotionSignals,
        dt: f32,
        tuning: &EngineTuning,
    ) -> LocomotionFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let speed = if signals.moving {
            input_length(signals)
        } else {
            0.0
        };
        // The cycle advances with the blocks travelled, so it never outruns
        // the feet however the frame rate moves. The stride is the pack's
        // ask, clamped the way `MovementTuning.strideDistance` clamps it.
        let stride = crate::signals::tuning::stride_distance(tuning.stride_distance, 0.5, 8.0);
        let distance = speed * WALK_SPEED * dt;
        if stride > 0.0 && distance.is_finite() {
            self.phase = gait_wrap(self.phase + distance / stride);
        }
        let target_stride = if signals.moving { speed.max(0.35) } else { 0.0 };
        self.stride = fade(self.stride, target_stride, dt, tuning.fade_rate);
        self.lift = fade(
            self.lift,
            signals.fall_weight as f32 / crate::signals::HEAVY as f32,
            dt,
            tuning.fade_rate,
        );
        if signals.running && signals.moving {
            self.run_seconds += dt;
        } else {
            self.run_seconds = 0.0;
        }
        LocomotionFrame {
            phase: self.phase * 360.0,
            stride: self.stride,
            lift: self.lift,
            speed,
            run_seconds: self.run_seconds,
        }
    }
}

// ---------------------------------------------------------------------------
// TurnInPlaceController
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Quaternions, the Joml calls the pose math needs
// ---------------------------------------------------------------------------

/// A rotation quaternion, `(w, x, y, z)`, mirroring the Joml calls the Java
/// makes on pose rotations: `rotationZYX(z, y, x)` to build and
/// `getEulerAnglesZYX` to read back, in degrees at the edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quat {
    /// The identity rotation.
    pub const IDENTITY: Quat = Quat {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// `new Quaternionf().rotationZYX(z, y, x)` from a pose rotation in
    /// degrees: about Z first, then Y, then X.
    pub fn from_zyx_degrees(rotation: Vec3) -> Self {
        let (zx, yx, xx) = (
            (rotation.z as f64).to_radians() * 0.5,
            (rotation.y as f64).to_radians() * 0.5,
            (rotation.x as f64).to_radians() * 0.5,
        );
        let (sz, cz) = (zx.sin() as f32, zx.cos() as f32);
        let (sy, cy) = (yx.sin() as f32, yx.cos() as f32);
        let (sx, cx) = (xx.sin() as f32, xx.cos() as f32);
        // q = qX * qY * qZ
        Self {
            w: cx * cy * cz - sx * sy * sz,
            x: sx * cy * cz + cx * sy * sz,
            y: cx * sy * cz - sx * cy * sz,
            z: cx * cy * sz + sx * sy * cz,
        }
    }

    /// `getEulerAnglesZYX`, back in degrees: `(x, y, z)` rotations.
    pub fn to_euler_zyx_degrees(self) -> Vec3 {
        let m = self.to_matrix3();
        // R = Rx(a) Ry(b) Rz(c): b from m02, a from -m12/m22, c from -m01/m00.
        let x = (-m[1][2]).atan2(m[2][2]);
        let y = m[0][2].clamp(-1.0, 1.0).asin();
        let z = (-m[0][1]).atan2(m[0][0]);
        Vec3::new(
            (x as f64 * 180.0 / std::f64::consts::PI) as f32,
            (y as f64 * 180.0 / std::f64::consts::PI) as f32,
            (z as f64 * 180.0 / std::f64::consts::PI) as f32,
        )
    }

    /// The rotation matrix, row-major `[[m00, m01, m02], ...]`.
    pub fn to_matrix3(self) -> [[f32; 3]; 3] {
        let (w, x, y, z) = (self.w, self.x, self.y, self.z);
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }

    /// `slerp`: the shortest path towards `other` by `t`.
    pub fn slerp(self, other: Quat, t: f32) -> Quat {
        if t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return other;
        }
        let mut dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;
        let (ox, oy, oz, ow) = if dot < 0.0 {
            dot = -dot;
            (-other.x, -other.y, -other.z, -other.w)
        } else {
            (other.x, other.y, other.z, other.w)
        };
        if dot > 1.0 - 1.0e-6 {
            // Nearly parallel: lerp and normalize.
            let out = Self {
                w: self.w + (ow - self.w) * t,
                x: self.x + (ox - self.x) * t,
                y: self.y + (oy - self.y) * t,
                z: self.z + (oz - self.z) * t,
            };
            return out.normalized();
        }
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;
        Self {
            w: self.w * a + ow * b,
            x: self.x * a + ox * b,
            y: self.y * a + oy * b,
            z: self.z * a + oz * b,
        }
    }

    /// `normalize`.
    pub fn normalized(self) -> Quat {
        let squared = self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z;
        if squared < 1.0e-12 {
            return Quat::IDENTITY;
        }
        let scale = 1.0 / squared.sqrt();
        Self {
            w: self.w * scale,
            x: self.x * scale,
            y: self.y * scale,
            z: self.z * scale,
        }
    }

    /// The Hamilton product, `self * other`.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Quat) -> Quat {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// `invert`.
    pub fn invert(self) -> Quat {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Rotates a vector.
    pub fn transform(self, v: Vec3) -> Vec3 {
        let qv = Quat {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };
        let out = self.mul(qv).mul(self.invert());
        Vec3::new(out.x, out.y, out.z)
    }
}

/// `ClipSequencePlayer.quaternion`: a pose rotation in degrees as a
/// quaternion.
fn quaternion(rotation: Vec3) -> Quat {
    Quat::from_zyx_degrees(rotation)
}

// ---------------------------------------------------------------------------
// MovingPoseBlend
// ---------------------------------------------------------------------------

/// `MovingPoseBlend`: a chain of sources blended into each other, the newest
/// easing in over the running one and dropping it once settled.
struct MovingPoseBlend<S: MovingSource> {
    root: MovingNode<S>,
}

impl<S: MovingSource> MovingPoseBlend<S> {
    /// A blend resting on the identity source.
    fn new(identity: impl Fn() -> S + 'static) -> Self {
        Self {
            root: MovingNode::Leaf {
                source: MovingSourceNode::Identity(std::rc::Rc::new(identity)),
            },
        }
    }

    /// `retarget`: the new source eases in over `seconds`.
    fn retarget(&mut self, source: S, seconds: f32) {
        self.root.commit();
        let leaf = MovingNode::Leaf {
            source: MovingSourceNode::Live(source),
        };
        self.root = if seconds > 0.0 {
            MovingNode::Blend {
                from: Box::new(std::mem::replace(&mut self.root, leaf.clone())),
                to: Box::new(leaf),
                duration: seconds.max(0.0),
                previous_elapsed: 0.0,
                elapsed: 0.0,
            }
        } else {
            leaf
        };
    }

    fn tick(&mut self, dt: f32) {
        let root = std::mem::replace(
            &mut self.root,
            MovingNode::Leaf {
                source: MovingSourceNode::Identity(std::rc::Rc::new(|| {
                    unreachable!("replaced on the next line")
                })),
            },
        );
        self.root = root.tick(dt.max(0.0));
    }

    fn sample(&self, bone: &str, partial: f32) -> S::Output {
        self.root.sample(bone, partial.clamp(0.0, 1.0))
    }

    fn transitioning(&self) -> bool {
        matches!(self.root, MovingNode::Blend { .. })
    }

    fn clear(&mut self, identity: impl Fn() -> S + 'static) {
        self.root = MovingNode::Leaf {
            source: MovingSourceNode::Identity(std::rc::Rc::new(identity)),
        };
    }
}

/// What a moving source produces and how it steps.
trait MovingSource: Clone {
    type Output: BlendMixable;

    fn sample(&self, bone: &str, partial: f32) -> Self::Output;
    fn tick(&mut self, dt: f32) {
        let _ = dt;
    }
    fn commit(&mut self) {}
    fn set_times(&mut self, _previous_time: f32, _time: f32, _rate: f32) {}
    fn sync_playback(
        &mut self,
        _previous_time: f32,
        _time: f32,
        _previous_overlay: f32,
        _overlay: f32,
        _rate: f32,
    ) {
    }
}

#[derive(Clone)]
enum MovingSourceNode<S: MovingSource> {
    /// The resting source: the identity.
    Identity(std::rc::Rc<dyn Fn() -> S>),
    /// A live playback.
    Live(S),
}

impl<S: MovingSource> MovingSourceNode<S> {
    fn sample(&self, bone: &str, partial: f32) -> S::Output {
        match self {
            MovingSourceNode::Identity(make) => make().sample(bone, partial),
            MovingSourceNode::Live(source) => source.sample(bone, partial),
        }
    }

    fn tick(&mut self, dt: f32) {
        if let MovingSourceNode::Live(source) = self {
            source.tick(dt);
        }
    }

    #[allow(
        dead_code,
        reason = "the leaf half of the node API, reached through MovingNode"
    )]
    fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
        if let MovingSourceNode::Live(source) = self {
            source.set_times(previous_time, time, rate);
        }
    }

    #[allow(
        dead_code,
        reason = "the leaf half of the node API, reached through MovingNode"
    )]
    fn sync_playback(
        &mut self,
        previous_time: f32,
        time: f32,
        previous_overlay: f32,
        overlay: f32,
        rate: f32,
    ) {
        if let MovingSourceNode::Live(source) = self {
            source.sync_playback(previous_time, time, previous_overlay, overlay, rate);
        }
    }
}

enum MovingNode<S: MovingSource> {
    Leaf {
        source: MovingSourceNode<S>,
    },
    Blend {
        from: Box<MovingNode<S>>,
        to: Box<MovingNode<S>>,
        duration: f32,
        previous_elapsed: f32,
        elapsed: f32,
    },
}

impl<S: MovingSource> Clone for MovingNode<S> {
    fn clone(&self) -> Self {
        match self {
            MovingNode::Leaf { source } => MovingNode::Leaf {
                source: source.clone(),
            },
            MovingNode::Blend {
                from,
                to,
                duration,
                previous_elapsed,
                elapsed,
            } => MovingNode::Blend {
                from: from.clone(),
                to: to.clone(),
                duration: *duration,
                previous_elapsed: *previous_elapsed,
                elapsed: *elapsed,
            },
        }
    }
}

impl<S: MovingSource> MovingNode<S> {
    fn sample(&self, bone: &str, partial: f32) -> S::Output {
        match self {
            MovingNode::Leaf { source } => source.sample(bone, partial),
            MovingNode::Blend {
                from,
                to,
                duration,
                previous_elapsed,
                elapsed,
            } => {
                let t = ((previous_elapsed + (elapsed - previous_elapsed) * partial)
                    / duration.max(1.0e-6))
                .clamp(0.0, 1.0);
                from.sample(bone, partial)
                    .mixed_with(&to.sample(bone, partial), t * t * (3.0 - 2.0 * t))
            }
        }
    }

    fn tick(self, dt: f32) -> Self {
        match self {
            MovingNode::Leaf { mut source } => {
                source.tick(dt);
                MovingNode::Leaf { source }
            }
            MovingNode::Blend {
                mut from,
                mut to,
                duration,
                previous_elapsed: _,
                mut elapsed,
            } => {
                from = Box::new(from.tick(dt));
                to = Box::new(to.tick(dt));
                let previous_elapsed = elapsed;
                elapsed += dt;
                if previous_elapsed >= duration {
                    *to
                } else {
                    MovingNode::Blend {
                        from,
                        to,
                        duration,
                        previous_elapsed,
                        elapsed,
                    }
                }
            }
        }
    }

    fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
        if let MovingNode::Leaf {
            source: MovingSourceNode::Live(source),
        } = self
        {
            source.set_times(previous_time, time, rate);
        }
    }

    fn sync_playback(
        &mut self,
        previous_time: f32,
        time: f32,
        previous_overlay: f32,
        overlay: f32,
        rate: f32,
    ) {
        if let MovingNode::Leaf {
            source: MovingSourceNode::Live(source),
        } = self
        {
            source.sync_playback(previous_time, time, previous_overlay, overlay, rate);
        }
    }

    fn commit(&mut self) {
        match self {
            MovingNode::Leaf { source } => source.tick(0.0),
            MovingNode::Blend {
                from,
                to,
                previous_elapsed,
                elapsed,
                ..
            } => {
                *previous_elapsed = *elapsed;
                from.commit();
                to.commit();
            }
        }
    }
}

/// The mix a moving blend applies between two outputs.
trait BlendMixable {
    fn mixed_with(&self, other: &Self, t: f32) -> Self;
}

// ---------------------------------------------------------------------------
// ClipSequencePlayer
// ---------------------------------------------------------------------------

/// `ClipSequencePlayer.Sample`: one bone's pose from the playback, with its
/// weight and the base-pose retention it carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub pose: Pose,
    pub weight: f32,
    pub base_retain: f32,
    pub legacy_retain_weight: f32,
}

impl Sample {
    /// `Sample.empty`.
    pub fn empty() -> Self {
        Self {
            pose: Pose::identity(),
            weight: 0.0,
            base_retain: 0.0,
            legacy_retain_weight: 1.0,
        }
    }

    /// `withWeight`.
    pub fn with_weight(self, weight: f32) -> Self {
        Self { weight, ..self }
    }

    /// `retention`.
    pub fn retention(self, legacy: f32) -> f32 {
        self.base_retain + self.legacy_retain_weight * legacy
    }
}

impl BlendMixable for Sample {
    fn mixed_with(&self, other: &Self, t: f32) -> Self {
        mix_samples(*self, *other, t)
    }
}

impl BlendMixable for Pose {
    fn mixed_with(&self, other: &Self, t: f32) -> Self {
        mix(*self, *other, t)
    }
}

/// One playback running one clip with its overlays.
#[derive(Debug, Clone, PartialEq)]
struct Playback {
    clip: Option<Clip>,
    overlays: Vec<ResolvedOverlay>,
    looping: bool,
    held: std::collections::HashMap<String, Sample>,
    retention: BasePoseRetention,
    left: bool,
    base: Option<Clip>,
    time: f32,
    previous_time: f32,
    overlay_time: f32,
    previous_overlay_time: f32,
    rate: f32,
}

impl Playback {
    fn new(
        clip: Option<Clip>,
        overlays: Vec<ResolvedOverlay>,
        looping: bool,
        held: std::collections::HashMap<String, Sample>,
        retention: BasePoseRetention,
        left: bool,
        base: Option<Clip>,
    ) -> Self {
        Self {
            clip,
            overlays,
            looping,
            held,
            retention,
            left,
            base,
            time: 0.0,
            previous_time: 0.0,
            overlay_time: 0.0,
            previous_overlay_time: 0.0,
            rate: 1.0,
        }
    }
}

impl MovingSource for Playback {
    type Output = Sample;

    fn sample(&self, bone: &str, partial: f32) -> Sample {
        let retain = self.retention.value(bone, self.left);
        let sampled = clip_sequence_current(
            self.clip.as_ref(),
            &self.overlays,
            self.looping,
            &self.held,
            bone,
            self.previous_time + (self.time - self.previous_time) * partial,
            self.previous_overlay_time + (self.overlay_time - self.previous_overlay_time) * partial,
            self.base.as_ref(),
        );
        if self.retention.value(bone, self.left).is_finite() {
            Sample {
                base_retain: retain,
                ..sampled
            }
        } else {
            sampled
        }
    }

    fn tick(&mut self, dt: f32) {
        self.previous_time = self.time;
        self.previous_overlay_time = self.overlay_time;
        self.time += dt * self.rate;
        self.overlay_time += dt * self.rate;
    }
}

/// `ClipSequencePlayer`: one clip playing with its overlays, cross-blending
/// into each new `play`.
pub struct ClipSequencePlayer {
    clip: Option<Clip>,
    base: Option<Clip>,
    overlays: Vec<ResolvedOverlay>,
    tracks: std::collections::HashSet<String>,
    direct_tracks: std::collections::HashSet<String>,
    duration: f32,
    current: Option<Playback>,
    blend: MovingPoseBlend<Playback>,
}

impl Default for ClipSequencePlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipSequencePlayer {
    /// A stopped player.
    pub fn new() -> Self {
        Self {
            clip: None,
            base: None,
            overlays: Vec::new(),
            tracks: std::collections::HashSet::new(),
            direct_tracks: std::collections::HashSet::new(),
            duration: 0.16,
            current: None,
            blend: MovingPoseBlend::new(|| Playback {
                clip: None,
                overlays: Vec::new(),
                looping: false,
                held: std::collections::HashMap::new(),
                retention: BasePoseRetention::default(),
                left: false,
                base: None,
                time: 0.0,
                previous_time: 0.0,
                overlay_time: 0.0,
                previous_overlay_time: 0.0,
                rate: 1.0,
            }),
        }
    }

    /// `play`, the full form: `hold_start` snapshots the running pose, the
    /// retention and handedness ride along, and `base` adds a pose on top.
    #[allow(clippy::too_many_arguments)]
    pub fn play(
        &mut self,
        clip: Option<Clip>,
        overlays: Vec<ResolvedOverlay>,
        looping: bool,
        transition_seconds: f32,
        hold_start: bool,
        retention: BasePoseRetention,
        left: bool,
        base: Option<Clip>,
    ) {
        let mut held = std::collections::HashMap::new();
        if hold_start {
            for bone in &self.tracks {
                held.insert(bone.clone(), self.sample(bone, 1.0));
            }
        }
        let mut additive = Vec::new();
        let mut merged = clip;
        if let Some(base_clip) = &merged
            && !overlays.is_empty()
        {
            for overlay in &overlays {
                if overlay.mode == OverlayMode::Additive {
                    additive.push(overlay.clone());
                }
            }
            merged = Some(merge_exclusive_tracks(base_clip, &overlays));
        }
        self.overlays = additive;
        self.base = base.clone();
        self.duration = if transition_seconds.is_finite() {
            transition_seconds.max(0.0)
        } else {
            0.16
        };
        let playback = Playback::new(
            merged.clone(),
            self.overlays.clone(),
            looping,
            held,
            retention,
            left,
            base,
        );
        let retarget = if self.current.is_some() {
            self.duration
        } else {
            0.0
        };
        self.blend.retarget(playback.clone(), retarget);
        self.current = Some(playback);
        self.clip = merged;
        let clips: Vec<Clip> = self
            .clip
            .iter()
            .cloned()
            .chain(self.overlays.iter().map(|o| o.clip.clone()))
            .chain(self.base.iter().cloned())
            .collect();
        for clip in &clips {
            self.add_tracks(clip);
        }
    }

    fn add_tracks(&mut self, clip: &Clip) {
        for bone in clip.bones.keys() {
            self.direct_tracks.insert(bone.clone());
            self.tracks.insert(bone.clone());
        }
        if clip.bones.contains_key("waist") {
            for bone in ["body", "head", "left_arm", "right_arm"] {
                self.tracks.insert(bone.to_string());
            }
        }
    }

    /// `tick`.
    pub fn tick(&mut self, dt: f32) {
        self.blend.tick(dt);
        if let Some(current) = &mut self.current {
            current.tick(dt);
        }
        if !self.blend.transitioning() {
            self.tracks.clear();
            self.direct_tracks.clear();
            let held: Vec<String> = self
                .current
                .as_ref()
                .map_or(Vec::new(), |current| current.held.keys().cloned().collect());
            self.tracks.extend(held);
            let clips: Vec<Clip> = self
                .clip
                .iter()
                .cloned()
                .chain(self.overlays.iter().map(|o| o.clip.clone()))
                .chain(self.base.iter().cloned())
                .collect();
            for clip in &clips {
                self.add_tracks(clip);
            }
        }
    }

    /// `seek`.
    pub fn seek(&mut self, time: f32) {
        let length = self.clip.as_ref().map(|clip| clip.length);
        if let (Some(current), Some(length)) = (&mut self.current, length) {
            current.time = time.clamp(0.0, length);
            self.sync_current();
        }
    }

    /// Copies the current playback's clock into the blend's live leaf, so
    /// the sampled side sees seeks and rate changes at once.
    fn sync_current(&mut self) {
        if let Some(current) = &self.current {
            let (previous_time, time, previous_overlay, overlay, rate) = (
                current.previous_time,
                current.time,
                current.previous_overlay_time,
                current.overlay_time,
                current.rate,
            );
            self.blend
                .root
                .sync_playback(previous_time, time, previous_overlay, overlay, rate);
        }
    }

    /// `setPlaybackRate`.
    pub fn set_playback_rate(&mut self, rate: f32) {
        if let Some(current) = &mut self.current {
            current.rate = if rate.is_finite() { rate.max(0.0) } else { 1.0 };
            self.sync_current();
        }
    }

    /// `renderTime`.
    pub fn render_time(&self, partial: f32) -> f32 {
        self.current.as_ref().map_or(0.0, |current| {
            current.previous_time + (current.time - current.previous_time) * partial
        })
    }

    /// `holdLastFrame`.
    pub fn hold_last_frame(&mut self) {
        let length = self.clip.as_ref().map(|clip| clip.length);
        if let (Some(current), Some(length)) = (&mut self.current, length) {
            current.previous_time = length;
            current.time = length;
            self.sync_current();
        }
    }

    /// `finished`.
    pub fn finished(&self) -> bool {
        match (&self.clip, &self.current) {
            (None, _) => true,
            (Some(clip), Some(current)) => !current.looping && current.time >= clip.length,
            (Some(_), None) => true,
        }
    }

    /// `progress`.
    pub fn progress(&self) -> f32 {
        match (&self.clip, &self.current) {
            (None, _) => 1.0,
            (Some(clip), Some(current)) => current.time / clip.length.max(0.001),
            (Some(_), None) => 1.0,
        }
    }

    /// `active`.
    pub fn active(&self) -> bool {
        self.clip.is_some() || self.blend.transitioning()
    }

    /// The clip playing.
    pub fn clip(&self) -> Option<&Clip> {
        self.clip.as_ref()
    }

    /// `hasDirectTrack`.
    pub fn has_direct_track(&self, bone: &str) -> bool {
        self.direct_tracks.contains(bone)
    }

    /// `stop`.
    pub fn stop(&mut self, transition_seconds: f32) {
        if self.clip.is_some() {
            self.play(
                None,
                Vec::new(),
                false,
                transition_seconds,
                false,
                BasePoseRetention::default(),
                false,
                None,
            );
        }
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.clip = None;
        self.base = None;
        self.current = None;
        self.overlays = Vec::new();
        self.tracks.clear();
        self.direct_tracks.clear();
        self.blend.clear(|| Playback {
            clip: None,
            overlays: Vec::new(),
            looping: false,
            held: std::collections::HashMap::new(),
            retention: BasePoseRetention::default(),
            left: false,
            base: None,
            time: 0.0,
            previous_time: 0.0,
            overlay_time: 0.0,
            previous_overlay_time: 0.0,
            rate: 1.0,
        });
    }

    /// `sample`.
    pub fn sample(&self, bone: &str, partial: f32) -> Sample {
        self.blend.sample(bone, partial)
    }
}

/// `ClipSequencePlayer.current`: one bone's sample from a clip and its
/// overlays, falling back to the held snapshot.
#[allow(clippy::too_many_arguments)]
pub fn clip_sequence_current(
    clip: Option<&Clip>,
    overlays: &[ResolvedOverlay],
    looping: bool,
    held: &std::collections::HashMap<String, Sample>,
    bone: &str,
    time: f32,
    overlay_time: f32,
    base: Option<&Clip>,
) -> Sample {
    let Some(clip) = clip else {
        return Sample::empty();
    };
    let mut found = has_track(Some(clip), bone);
    let mut pose = if looping {
        clip.sample(bone, time)
    } else {
        clip.sample_once(bone, time)
    };
    for overlay in overlays {
        if overlay.applies_to(bone) && has_track(Some(&overlay.clip), bone) {
            let overlay_pose = overlay.clip.sample(bone, overlay_time);
            found = true;
            match overlay.mode {
                OverlayMode::Additive => {
                    pose = Pose {
                        position: pose.position + overlay_pose.position,
                        rotation: pose.rotation + overlay_pose.rotation,
                        scale: pose.scale + overlay_pose.scale - Vec3::new(1.0, 1.0, 1.0),
                    };
                }
                OverlayMode::Override | OverlayMode::Exclusive => {
                    pose = overlay_pose;
                }
            }
            if overlay.mode == OverlayMode::Exclusive {
                break;
            }
        }
    }
    if let Some(base) = base
        && has_track(Some(base), bone)
    {
        let base_pose = base.sample_once(bone, base.length);
        pose = Pose {
            position: base_pose.position + pose.position,
            rotation: base_pose.rotation + pose.rotation,
            scale: base_pose.scale + pose.scale - Vec3::new(1.0, 1.0, 1.0),
        };
        found = true;
    }
    if found {
        Sample {
            pose,
            weight: 1.0,
            base_retain: 0.0,
            legacy_retain_weight: 1.0,
        }
    } else {
        held.get(bone).copied().unwrap_or_else(Sample::empty)
    }
}

/// `ClipSequencePlayer.hasTrack`.
pub fn has_track(clip: Option<&Clip>, bone: &str) -> bool {
    let Some(clip) = clip else {
        return false;
    };
    clip.bones.contains_key(bone)
        || clip.bones.contains_key("waist")
            && matches!(bone, "body" | "head" | "left_arm" | "right_arm")
}

/// `ClipSequencePlayer.mergeExclusiveTracks`.
pub fn merge_exclusive_tracks(clip: &Clip, overlays: &[ResolvedOverlay]) -> Clip {
    let mut bones = clip.bones.clone();
    for overlay in overlays {
        if overlay.mode != OverlayMode::Additive {
            for (bone, track) in &overlay.clip.bones {
                if overlay.applies_to(bone) {
                    bones.insert(bone.clone(), track.clone());
                }
            }
        }
    }
    Clip::new(clip.name.clone(), clip.length, clip.loop_, bones)
}

/// `ClipSequencePlayer.mixSamples`.
pub fn mix_samples(a: Sample, b: Sample, t: f32) -> Sample {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let wa = a.weight * (1.0 - t);
    let wb = b.weight * t;
    let total = wa + wb;
    if total < 1.0e-5 {
        return Sample::empty();
    }
    let share = wb / total;
    Sample {
        pose: mix(a.pose, b.pose, share),
        weight: total,
        base_retain: a.base_retain + (b.base_retain - a.base_retain) * share,
        legacy_retain_weight: a.legacy_retain_weight
            + (b.legacy_retain_weight - a.legacy_retain_weight) * share,
    }
}

/// `ClipSequencePlayer.mix`.
pub fn mix(a: Pose, b: Pose, t: f32) -> Pose {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    Pose {
        position: a.position.lerp(b.position, t),
        rotation: mix_rotation(a.rotation, b.rotation, t),
        scale: a.scale.lerp(b.scale, t),
    }
}

/// `ClipSequencePlayer.mixRotation`: slerp through quaternions, read back as
/// degrees.
pub fn mix_rotation(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    quaternion(a)
        .slerp(quaternion(b), t)
        .normalized()
        .to_euler_zyx_degrees()
}

/// `ClipSequencePlayer.applyRotationOffset`.
pub fn apply_rotation_offset(base: Vec3, from: Vec3, to: Vec3, t: f32) -> Vec3 {
    let offset = quaternion(from).invert().mul(quaternion(to));
    quaternion(base)
        .mul(Quat::IDENTITY.slerp(offset, t))
        .normalized()
        .to_euler_zyx_degrees()
}

/// `ClipSequencePlayer.compose`: the sample over the base pose, keeping
/// `retain` of the base.
pub fn compose(base: Pose, sample: Sample, legacy_retain: f32) -> Pose {
    let weight = sample.weight.clamp(0.0, 1.0);
    if weight <= 0.0 {
        return base;
    }
    let retain = sample.retention(legacy_retain).clamp(0.0, 1.0);
    let composed = Pose {
        position: base.position * retain + sample.pose.position,
        rotation: retained_rotation(base.rotation, sample.pose.rotation, retain),
        scale: (base.scale - Vec3::new(1.0, 1.0, 1.0)) * retain + sample.pose.scale,
    };
    if weight >= 0.99999 {
        return composed;
    }
    Pose {
        position: base.position.lerp(composed.position, weight),
        rotation: quaternion(base.rotation)
            .slerp(quaternion(composed.rotation), weight)
            .normalized()
            .to_euler_zyx_degrees(),
        scale: base.scale.lerp(composed.scale, weight),
    }
}

/// `ClipSequencePlayer.retainedRotation`.
fn retained_rotation(base: Vec3, over: Vec3, retain: f32) -> Vec3 {
    if retain != 0.0 && base.length_squared() != 0.0 {
        quaternion(base * retain)
            .mul(quaternion(over))
            .normalized()
            .to_euler_zyx_degrees()
    } else {
        over
    }
}

// ---------------------------------------------------------------------------
// ClipPoseBlend
// ---------------------------------------------------------------------------

/// `ClipPoseBlend.Playback`: one clip running against the render clock.
#[derive(Debug, Clone, PartialEq)]
struct ClipPosePlayback {
    clip: Clip,
    carry_clip: Clip,
    overlays: Vec<ResolvedOverlay>,
    carry_overlays: Vec<ResolvedOverlay>,
    looping: bool,
    previous_time: f32,
    time: f32,
    rate: f32,
}

impl MovingSource for ClipPosePlayback {
    type Output = Pose;

    fn sample(&self, bone: &str, partial: f32) -> Pose {
        let mut bone = bone;
        let carry = bone.starts_with("carry:");
        if carry {
            bone = &bone["carry:".len()..];
        }
        let mut time = self.previous_time + (self.time - self.previous_time) * partial;
        if self.looping {
            let length = self.clip.length.max(1.0e-4);
            time = positive_modulo(time, length);
        }
        clip_sequence_current(
            if carry {
                Some(&self.carry_clip)
            } else {
                Some(&self.clip)
            },
            if carry {
                &self.carry_overlays
            } else {
                &self.overlays
            },
            self.looping,
            &std::collections::HashMap::new(),
            bone,
            time,
            time,
            None,
        )
        .pose
    }

    fn tick(&mut self, dt: f32) {
        self.commit();
        self.time += dt * self.rate;
    }

    fn commit(&mut self) {
        self.previous_time = self.time;
    }

    fn set_times(&mut self, previous_time: f32, time: f32, rate: f32) {
        self.previous_time = previous_time;
        self.time = time;
        self.rate = rate;
    }
}

/// `ClipPoseBlend`: one clip's pose, blended into each new `start`.
pub struct ClipPoseBlend {
    blend: MovingPoseBlend<ClipPosePlayback>,
}

impl Default for ClipPoseBlend {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipPoseBlend {
    /// A blend resting on identity.
    pub fn new() -> Self {
        Self {
            blend: MovingPoseBlend::new(|| ClipPosePlayback {
                clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
                carry_clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
                overlays: Vec::new(),
                carry_overlays: Vec::new(),
                looping: false,
                previous_time: 0.0,
                time: 0.0,
                rate: 1.0,
            }),
        }
    }

    /// `start`.
    pub fn start(
        &mut self,
        clip: Option<Clip>,
        time: f32,
        rate: f32,
        looping: bool,
        blend_seconds: f32,
        overlays: Vec<ResolvedOverlay>,
    ) {
        let Some(clip) = clip else {
            return;
        };
        let playback = ClipPosePlayback {
            carry_clip: clip.without_local_arms(),
            carry_overlays: overlays
                .iter()
                .map(|overlay| ResolvedOverlay {
                    clip: overlay.clip.without_local_arms(),
                    mode: overlay.mode,
                    bones: overlay.bones.clone(),
                })
                .collect(),
            clip,
            overlays: overlays
                .iter()
                .filter(|overlay| overlay.mode == OverlayMode::Additive)
                .cloned()
                .collect(),
            looping,
            previous_time: time,
            time,
            rate,
        };
        let retarget = if self.has_target() {
            blend_seconds
        } else {
            0.0
        };
        self.blend.retarget(playback, retarget);
    }

    fn has_target(&self) -> bool {
        !matches!(
            &self.blend.root,
            MovingNode::Leaf {
                source: MovingSourceNode::Identity(_),
            }
        )
    }

    /// `tick`.
    pub fn tick(&mut self, dt: f32, previous_time: f32, time: f32, rate: f32) {
        self.blend.tick(dt);
        self.blend.root.set_times(previous_time, time, rate);
    }

    /// `sample`.
    pub fn sample(&self, bone: &str, partial: f32) -> Pose {
        self.blend.sample(bone, partial)
    }

    /// `sampleCarry`.
    pub fn sample_carry(&self, bone: &str, partial: f32) -> Pose {
        self.blend.sample(&format!("carry:{}", bone), partial)
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.blend.clear(|| ClipPosePlayback {
            clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
            carry_clip: Clip::new(String::new(), 0.0, false, std::collections::HashMap::new()),
            overlays: Vec::new(),
            carry_overlays: Vec::new(),
            looping: false,
            previous_time: 0.0,
            time: 0.0,
            rate: 1.0,
        });
    }
}

// ---------------------------------------------------------------------------
// TurnInPlaceController (+ CameraTurnInPlace, KeyframedTurnInPlace)
// ---------------------------------------------------------------------------

/// `TurnInPlaceController.ENTRY_SECONDS`.
pub const TURN_ENTRY_SECONDS: f32 = 0.08;
/// `TurnInPlaceController.ALIGNMENT_EPSILON`.
pub const TURN_ALIGNMENT_EPSILON: f32 = 1.0;

/// Which turn-in-place playback a setting asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TurnMode {
    /// One clip scaled by how far the turn went.
    Scaled,
    /// Left and right step clips.
    #[default]
    Steps,
    /// The camera-following tracker.
    Camera,
    /// The keyframed two-turn tracker (`dynamic` and `procedural`).
    Dynamic,
}

/// `TurnInPlaceController.Settings`: what a pack configures, the Java
/// record's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnSettings {
    pub enabled: bool,
    pub threshold: f32,
    pub delay: f32,
    pub fade: f32,
    pub mode: TurnMode,
    pub reference: f32,
    pub double_turn: f32,
    pub template: Option<Clip>,
    pub left: Option<Clip>,
    pub right: Option<Clip>,
    pub left_add: Option<Clip>,
    pub right_add: Option<Clip>,
}

impl Default for TurnSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 45.0,
            delay: 2.0,
            fade: 0.12,
            mode: TurnMode::Steps,
            reference: 45.0,
            double_turn: 75.0,
            template: None,
            left: None,
            right: None,
            left_add: None,
            right_add: None,
        }
    }
}

impl TurnSettings {
    /// `Settings.from(library)`: the `turn_in_place` settings off the
    /// library, with the Java defaults and clamps.
    pub fn from_library(library: &crate::library::AnimationLibrary) -> Self {
        let mode = match library
            .bindings()
            .string_setting(None, None, "turn_in_place_mode", "steps")
            .as_str()
        {
            "scaled" => TurnMode::Scaled,
            "camera" => TurnMode::Camera,
            "dynamic" | "procedural" => TurnMode::Dynamic,
            _ => TurnMode::Steps,
        };
        let template = library.clip(&library.bindings().string_setting(
            None,
            None,
            "turn_in_place_clip",
            "",
        ));
        let left = library.clip(&library.bindings().string_setting(
            None,
            None,
            "turn_in_place_left",
            if mode == TurnMode::Dynamic {
                "step_left"
            } else {
                ""
            },
        ));
        let right = library.clip(&library.bindings().string_setting(
            None,
            None,
            "turn_in_place_right",
            if mode == TurnMode::Dynamic {
                "step_right"
            } else {
                ""
            },
        ));
        let valid = |clip: Option<&Clip>| {
            clip.is_some_and(|clip| {
                has_track(Some(clip), "left_leg")
                    && has_track(Some(clip), "right_leg")
                    && clip.length.is_finite()
                    && clip.length > 0.0
            })
        };
        let enabled = match mode {
            TurnMode::Scaled => valid(template),
            _ => valid(left) && valid(right),
        };
        let setting = |key: &str, default: f32, min: f32, max: f32| {
            let value = library.bindings().numeric_setting(
                None,
                None,
                &format!("turn_in_place_{}", key),
                default,
            );
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                default
            }
        };
        Self {
            enabled: enabled
                && library
                    .bindings()
                    .numeric_setting(None, None, "turn_in_place_enabled", 0.0)
                    > 0.5,
            threshold: if mode == TurnMode::Dynamic {
                setting("minimum_turn_degrees", 2.0, 2.0, 90.0)
            } else {
                setting("minimum_turn_degrees", 45.0, 2.0, 90.0)
            },
            delay: if mode == TurnMode::Dynamic {
                setting("turn_delay_seconds", 0.12, 0.0, 0.3)
            } else {
                setting(
                    "look_idle_seconds",
                    if mode == TurnMode::Camera { 0.0 } else { 2.0 },
                    0.0,
                    10.0,
                )
            },
            fade: setting("fade_out_seconds", 0.12, 0.05, 0.4),
            mode,
            reference: setting("reference_degrees", 45.0, 1.0, 180.0),
            double_turn: setting("double_turn_degrees", 75.0, 30.0, 180.0),
            template: template.cloned(),
            left: left.cloned(),
            right: right.cloned(),
            left_add: library
                .clip(
                    &library
                        .bindings()
                        .string_setting(None, None, "turn_in_place_left_add", ""),
                )
                .cloned(),
            right_add: library
                .clip(
                    &library
                        .bindings()
                        .string_setting(None, None, "turn_in_place_right_add", ""),
                )
                .cloned(),
        }
    }
}

/// `CameraTurnInPlace`: the camera-mode tracker.
#[derive(Debug, Clone, PartialEq)]
struct CameraTurnInPlace {
    state: CameraTurnState,
    previous_look: f32,
    target_yaw: f32,
    body_yaw: f32,
    tracking_start_yaw: f32,
    time: f32,
    strength: f32,
    retarget_time: f32,
    last_motion_time: f64,
    retargeting: bool,
    clip: Option<Clip>,
    addition: Option<Clip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CameraTurnState {
    #[default]
    Idle,
    Tracking,
    Animating,
}

/// `CameraTurnInPlace.INPUT_GAP_SECONDS`.
const CAMERA_INPUT_GAP: f32 = 0.05;
/// `CameraTurnInPlace.RETARGET_SECONDS`.
const CAMERA_RETARGET_SECONDS: f32 = 0.08;
/// `CameraTurnInPlace.ANGLE_EPSILON`.
const CAMERA_ANGLE_EPSILON: f32 = 0.001;

impl CameraTurnInPlace {
    fn reset(&mut self, body_yaw: f32, look: f32, time: f64) {
        self.state = CameraTurnState::Idle;
        self.previous_look = look;
        self.body_yaw = body_yaw;
        self.tracking_start_yaw = body_yaw;
        self.target_yaw = body_yaw + wrap_degrees360(look - body_yaw);
        self.last_motion_time = time;
        self.time = 0.0;
        self.strength = 0.0;
        self.retarget_time = 0.0;
        self.retargeting = false;
        self.clip = None;
        self.addition = None;
    }

    fn update(&mut self, look: f32, time: f64, dt: f32, settings: &TurnSettings) -> f32 {
        let delta = wrap_degrees360(look - self.previous_look);
        self.previous_look = look;
        self.target_yaw += delta;
        let moved = delta.abs() > 1.0e-6;
        if moved {
            self.last_motion_time = time;
        }
        if self.state == CameraTurnState::Animating {
            let length = self.clip.as_ref().map_or(0.0, |clip| clip.length);
            self.time = length.min(self.time + dt);
            if self.time < length {
                return self.body_yaw;
            }
            self.clip = None;
            self.strength = 0.0;
            self.state = CameraTurnState::Idle;
            if wrap_degrees360(self.target_yaw - self.body_yaw).abs() > CAMERA_ANGLE_EPSILON {
                self.begin_tracking(true);
            }
            return self.body_yaw;
        }
        if self.state == CameraTurnState::Idle {
            if !moved {
                return self.body_yaw;
            }
            self.begin_tracking(false);
        }
        if self.retargeting {
            self.retarget_time = CAMERA_RETARGET_SECONDS.min(self.retarget_time + dt);
            let t = self.retarget_time / CAMERA_RETARGET_SECONDS;
            let eased = t * t * (3.0 - 2.0 * t);
            self.body_yaw = self.tracking_start_yaw
                + wrap_degrees360(self.target_yaw - self.tracking_start_yaw) * eased;
            self.retargeting = self.retarget_time < CAMERA_RETARGET_SECONDS;
        } else {
            self.body_yaw = self.target_yaw;
        }
        let swung = self.body_yaw - self.tracking_start_yaw;
        self.strength = (swung / settings.reference).clamp(0.0, 1.0);
        self.clip = if swung < 0.0 {
            settings.left.clone()
        } else {
            settings.right.clone()
        };
        self.addition = if swung < 0.0 {
            settings.left_add.clone()
        } else {
            settings.right_add.clone()
        };
        self.time = 0.0;
        if !self.retargeting
            && !moved
            && (time - self.last_motion_time) as f32 >= CAMERA_INPUT_GAP - 1.0e-6
        {
            if self.strength > 0.0 {
                self.state = CameraTurnState::Animating;
            } else {
                self.state = CameraTurnState::Idle;
                self.clip = None;
            }
        }
        self.body_yaw
    }

    fn begin_tracking(&mut self, retarget: bool) {
        self.state = CameraTurnState::Tracking;
        self.tracking_start_yaw = self.body_yaw;
        self.target_yaw = self.body_yaw + wrap_degrees360(self.target_yaw - self.body_yaw);
        self.time = 0.0;
        self.strength = 0.0;
        self.retarget_time = 0.0;
        self.retargeting = retarget;
    }

    fn sample(&self, bone: &str) -> Pose {
        if let (Some(clip), false) = (&self.clip, self.strength == 0.0) {
            let pose = sample_authored(clip, self.addition.as_ref(), bone, self.time);
            return Pose {
                position: pose.position * self.strength,
                rotation: pose.rotation * self.strength,
                scale: Vec3::new(1.0, 1.0, 1.0).lerp(pose.scale, self.strength),
            };
        }
        Pose::identity()
    }

    fn active(&self) -> bool {
        self.clip.is_some() && self.strength > 0.0
    }
}

/// `KeyframedTurnInPlace`: the two-turn tracker.
#[derive(Debug, Clone, PartialEq)]
struct KeyframedTurnInPlace {
    from: HashMap<String, Pose>,
    now: f32,
    gesture_time: f32,
    last_motion: f32,
    reverse_distance: f32,
    restart_since: f32,
    large_since: f32,
    body_yaw: f32,
    previous_look: f32,
    look_target: f32,
    origin_yaw: f32,
    gesture_direction: f32,
    start_yaw: f32,
    target_yaw: f32,
    time: f32,
    strength: f32,
    playback_rate: f32,
    body_duration: f32,
    entry_duration: f32,
    turns: u8,
    gesture: bool,
    stopped: bool,
    two_turns: bool,
    restart_pending: bool,
    clip: Option<Clip>,
    addition: Option<Clip>,
}

/// The keyframed tracker's constants.
const KEYFRAMED_QUIET_SECONDS: f32 = 0.12;
const KEYFRAMED_FIRST_BODY_SECONDS: f32 = 0.36;
const KEYFRAMED_SECOND_BODY_SECONDS: f32 = 0.3;
const KEYFRAMED_FIRST_ENTRY_SECONDS: f32 = 0.1;
const KEYFRAMED_SECOND_ENTRY_SECONDS: f32 = 0.06;
const KEYFRAMED_SECOND_PLAYBACK_RATE: f32 = 1.4;
const KEYFRAMED_REVERSAL_DEGREES: f32 = 3.0;
const KEYFRAMED_FIRST_TURN_SHARE: f32 = 0.48;
const KEYFRAMED_MAX_BLEND: f64 = 1.333_333_333_33;
const KEYFRAMED_BLEND_RESPONSE_SECONDS: f32 = 0.04;
const KEYFRAMED_MAX_COLLECTION_SECONDS: f32 = 0.3;
const KEYFRAMED_BONES: [&str; 6] = [
    "left_leg",
    "right_leg",
    "left_arm",
    "right_arm",
    "body",
    "head",
];

impl Default for KeyframedTurnInPlace {
    fn default() -> Self {
        let mut out = Self {
            from: HashMap::new(),
            now: 0.0,
            gesture_time: 0.0,
            last_motion: 0.0,
            reverse_distance: 0.0,
            restart_since: 0.0,
            large_since: f32::NAN,
            body_yaw: 0.0,
            previous_look: 0.0,
            look_target: 0.0,
            origin_yaw: 0.0,
            gesture_direction: 0.0,
            start_yaw: 0.0,
            target_yaw: 0.0,
            time: 0.0,
            strength: 0.0,
            playback_rate: 1.0,
            body_duration: KEYFRAMED_FIRST_BODY_SECONDS,
            entry_duration: KEYFRAMED_FIRST_ENTRY_SECONDS,
            turns: 0,
            gesture: false,
            stopped: false,
            two_turns: false,
            restart_pending: false,
            clip: None,
            addition: None,
        };
        out.reset(0.0, 0.0);
        out
    }
}

impl KeyframedTurnInPlace {
    fn reset(&mut self, body_yaw: f32, look: f32) {
        self.body_yaw = body_yaw;
        self.origin_yaw = body_yaw;
        self.start_yaw = body_yaw;
        self.target_yaw = body_yaw;
        self.previous_look = look;
        self.look_target = body_yaw + wrap_degrees360(look - body_yaw);
        self.now = 0.0;
        self.gesture_time = 0.0;
        self.reverse_distance = 0.0;
        self.time = 0.0;
        self.strength = 0.0;
        self.restart_since = 0.0;
        self.large_since = f32::NAN;
        self.playback_rate = 1.0;
        self.body_duration = KEYFRAMED_FIRST_BODY_SECONDS;
        self.entry_duration = KEYFRAMED_FIRST_ENTRY_SECONDS;
        self.last_motion = f32::NEG_INFINITY;
        self.turns = 0;
        self.gesture = false;
        self.stopped = false;
        self.two_turns = false;
        self.restart_pending = false;
        self.clip = None;
        self.addition = None;
        self.from.clear();
    }

    fn update(&mut self, look: f32, dt: f32, key_pressed: bool, settings: &TurnSettings) -> f32 {
        self.now += dt;
        let delta = wrap_degrees360(look - self.previous_look);
        self.previous_look = look;
        self.look_target += delta;
        if delta.abs() > 1.0e-6 {
            let restart_wanted =
                self.clip.is_none() && (self.turns == 2 || self.turns == 1 && !self.two_turns);
            if !self.gesture
                || restart_wanted
                || self.clip.is_none() && self.turns == 0 && self.stopped
            {
                self.start_gesture(delta.signum(), self.now);
            }
            if self.gesture && delta * self.gesture_direction < 0.0 {
                self.reverse_distance += delta;
            } else {
                self.reverse_distance = 0.0;
            }
            if self.reverse_distance.abs() >= KEYFRAMED_REVERSAL_DEGREES
                || self.clip.is_some() && (self.turns == 2 || self.turns == 1 && !self.two_turns)
            {
                if self.clip.is_none() {
                    self.start_gesture(delta.signum(), self.now);
                } else if !self.restart_pending {
                    self.restart_pending = true;
                    self.restart_since = self.now;
                }
            }
            self.last_motion = self.now;
        } else if key_pressed && !self.gesture && (self.look_target - self.body_yaw).abs() > 1.0 {
            self.start_gesture((self.look_target - self.body_yaw).signum(), self.now);
            self.last_motion = self.now - KEYFRAMED_QUIET_SECONDS;
        }
        self.stopped =
            self.gesture && self.now - self.last_motion >= KEYFRAMED_QUIET_SECONDS - 1.0e-6;
        if let Some(clip) = &self.clip {
            let length = clip.length;
            let mut swung =
                (self.look_target - self.start_yaw) * (self.target_yaw - self.start_yaw).signum();
            if self.turns == 1 && self.two_turns {
                swung *= KEYFRAMED_FIRST_TURN_SHARE;
            }
            let target_strength = blend_strength(
                (self.target_yaw - self.start_yaw).abs().max(swung),
                settings.reference,
            );
            let response = -(-dt / KEYFRAMED_BLEND_RESPONSE_SECONDS).exp_m1();
            self.strength += (target_strength - self.strength).max(0.0) * response;
            self.time = length.min(self.time + dt * self.playback_rate);
            self.body_yaw = self.start_yaw
                + (self.target_yaw - self.start_yaw)
                    * ease_turn((self.time / self.body_duration.min(length)).min(1.0));
            if self.time >= self.entry_duration {
                self.from.clear();
            }
            if self.time >= length - 1.0e-4 {
                self.time = length;
                self.body_yaw = self.target_yaw;
                self.clip = None;
                self.addition = None;
                self.strength = 0.0;
                self.from.clear();
                if self.restart_pending {
                    self.start_gesture(
                        (self.look_target - self.body_yaw).signum(),
                        self.restart_since,
                    );
                    self.stopped = self.now - self.last_motion >= KEYFRAMED_QUIET_SECONDS - 1.0e-6;
                }
            }
        }
        let mut pending = self.look_target
            - if self.turns == 0 {
                self.body_yaw
            } else {
                self.origin_yaw
            };
        let large = pending.abs() >= settings.double_turn - 0.001;
        if !self.gesture || self.turns != 0 || !large {
            self.large_since = f32::NAN;
        } else if self.large_since.is_nan() {
            self.large_since = self.now;
        }
        let collected = self.stopped
            || large
                && (pending.abs() * KEYFRAMED_FIRST_TURN_SHARE
                    >= settings.reference * KEYFRAMED_MAX_BLEND as f32
                    || self.now - self.large_since >= KEYFRAMED_MAX_COLLECTION_SECONDS - 1.0e-6);
        if self.gesture
            && self.clip.is_none()
            && self.turns == 0
            && (key_pressed || self.now - self.gesture_time >= settings.delay - 1.0e-6)
            && collected
            && pending.abs() >= if key_pressed { 1.0 } else { settings.threshold }
        {
            self.origin_yaw = self.body_yaw;
            pending = self.look_target - self.origin_yaw;
            self.two_turns = pending.abs() >= settings.double_turn - 0.001;
            let first = if self.two_turns {
                pending * KEYFRAMED_FIRST_TURN_SHARE
            } else {
                pending
            };
            self.begin(self.body_yaw + first, false, settings);
            self.turns = 1;
        } else if self.gesture
            && self.turns == 1
            && self.two_turns
            && self.clip.is_none()
            && self.stopped
        {
            if (self.look_target - self.body_yaw).abs()
                >= if key_pressed { 1.0 } else { settings.threshold }
            {
                self.begin(self.look_target, true, settings);
                self.turns = 2;
            } else {
                self.two_turns = false;
            }
        }
        self.body_yaw
    }

    fn start_gesture(&mut self, direction: f32, at: f32) {
        self.gesture = true;
        self.stopped = false;
        self.two_turns = false;
        self.gesture_time = at;
        self.origin_yaw = self.body_yaw;
        self.gesture_direction = direction;
        self.reverse_distance = 0.0;
        self.large_since = f32::NAN;
        self.turns = 0;
        self.restart_pending = false;
    }

    fn begin(&mut self, target: f32, second: bool, settings: &TurnSettings) {
        let mut from = HashMap::new();
        for bone in KEYFRAMED_BONES {
            from.insert(bone.to_string(), self.sample(bone));
        }
        self.start_yaw = self.body_yaw;
        self.target_yaw = target;
        self.strength = blend_strength((target - self.start_yaw).abs(), settings.reference);
        self.playback_rate = if second {
            KEYFRAMED_SECOND_PLAYBACK_RATE
        } else {
            1.0
        };
        self.body_duration = if second {
            KEYFRAMED_SECOND_BODY_SECONDS
        } else {
            KEYFRAMED_FIRST_BODY_SECONDS
        };
        self.entry_duration = if second {
            KEYFRAMED_SECOND_ENTRY_SECONDS
        } else {
            KEYFRAMED_FIRST_ENTRY_SECONDS
        };
        self.clip = if target < self.body_yaw {
            settings.left.clone()
        } else {
            settings.right.clone()
        };
        self.addition = if target < self.body_yaw {
            settings.left_add.clone()
        } else {
            settings.right_add.clone()
        };
        self.time = 0.0;
        self.from = from;
    }

    fn sample(&self, bone: &str) -> Pose {
        let Some(clip) = &self.clip else {
            return Pose::identity();
        };
        let pose = sample_authored(clip, self.addition.as_ref(), bone, self.time);
        let scaled = Pose {
            position: pose.position * self.strength,
            rotation: pose.rotation * self.strength,
            scale: Vec3::new(1.0, 1.0, 1.0).lerp(pose.scale, self.strength),
        };
        if self.time >= self.entry_duration {
            return scaled;
        }
        mix(
            self.from.get(bone).cloned().unwrap_or_else(Pose::identity),
            scaled,
            ease_turn((self.time / self.entry_duration).min(1.0)),
        )
    }

    fn active(&self) -> bool {
        self.clip.is_some()
    }
}

/// `blendStrength`.
fn blend_strength(degrees: f32, reference: f32) -> f32 {
    ((degrees / reference) as f64).clamp(0.0, KEYFRAMED_MAX_BLEND) as f32
}

/// The turn controllers' smoothstep.
fn ease_turn(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// The turn controllers' degree wrap.
fn wrap_degrees360(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

/// `TurnInPlaceController`: the body yaw standing still, with the mode the
/// settings pick.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnInPlaceController {
    initialized: bool,
    managed: bool,
    releasing: bool,
    last_time: f64,
    last_look_change: f64,
    body_yaw: f32,
    quiet_yaw: f32,
    quiet_pitch: f32,
    start_yaw: f32,
    target_yaw: f32,
    progress: f32,
    weight: f32,
    time: f32,
    release_time: f32,
    release_yaw: f32,
    strength: f32,
    clip: Option<Clip>,
    mirrored: bool,
    camera_turn: CameraTurnInPlace,
    keyframed_turn: KeyframedTurnInPlace,
    settings_addition: Option<Clip>,
    camera_mode: bool,
    dynamic_mode: bool,
}

impl Default for TurnInPlaceController {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnInPlaceController {
    /// A controller resting on the vanilla body yaw.
    pub fn new() -> Self {
        Self {
            initialized: false,
            managed: false,
            releasing: false,
            last_time: 0.0,
            last_look_change: 0.0,
            body_yaw: 0.0,
            quiet_yaw: 0.0,
            quiet_pitch: 0.0,
            start_yaw: 0.0,
            target_yaw: 0.0,
            progress: 0.0,
            weight: 0.0,
            time: 0.0,
            release_time: 0.0,
            release_yaw: 0.0,
            strength: 1.0,
            clip: None,
            mirrored: false,
            camera_turn: CameraTurnInPlace {
                state: CameraTurnState::Idle,
                previous_look: 0.0,
                target_yaw: 0.0,
                body_yaw: 0.0,
                tracking_start_yaw: 0.0,
                time: 0.0,
                strength: 0.0,
                retarget_time: 0.0,
                last_motion_time: 0.0,
                retargeting: false,
                clip: None,
                addition: None,
            },
            keyframed_turn: KeyframedTurnInPlace::default(),
            settings_addition: None,
            camera_mode: false,
            dynamic_mode: false,
        }
    }

    /// `update`: `body_yaw` in, the look yaw and pitch, the animation time,
    /// `compatible` whether the body is free to turn in place, `stationary`
    /// whether the gait says the player stands still, `key_pressed` whether
    /// the player holds a key that would turn, and the settings.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        body_yaw: f32,
        look_yaw: f32,
        look_pitch: f32,
        time: f64,
        compatible: bool,
        stationary: bool,
        key_pressed: bool,
        settings: &TurnSettings,
    ) -> f32 {
        if !(body_yaw.is_finite()
            && look_yaw.is_finite()
            && look_pitch.is_finite()
            && time.is_finite())
        {
            self.clear();
            return body_yaw;
        }
        if !self.initialized || time < self.last_time || time - self.last_time > 0.5 {
            self.reset(body_yaw, look_yaw, look_pitch, time);
        }
        if !compatible || !settings.enabled {
            self.reset(body_yaw, look_yaw, look_pitch, time);
            return body_yaw;
        }
        let dt = (time - self.last_time) as f32;
        if dt <= 0.0 {
            return if self.managed {
                self.body_yaw
            } else {
                body_yaw
            };
        }
        self.last_time = time;
        if wrap_degrees360(look_yaw - self.quiet_yaw).abs() > 0.15
            || (look_pitch - self.quiet_pitch).abs() > 0.15
        {
            self.quiet_yaw = look_yaw;
            self.quiet_pitch = look_pitch;
            self.last_look_change = time;
        }
        if !stationary {
            // The feet moved: hand the yaw back to them over the fade.
            self.camera_mode = false;
            self.dynamic_mode = false;
            self.clip = None;
            self.weight = 0.0;
            if !self.managed {
                self.body_yaw = body_yaw;
                self.reset_continuous_turns(look_yaw, time);
                return body_yaw;
            }
            if !self.releasing {
                self.releasing = true;
                self.release_yaw = self.body_yaw;
                self.release_time = 0.0;
            }
            self.release_time += dt;
            let eased = ease_turn((self.release_time / settings.fade).min(1.0));
            self.body_yaw = self.release_yaw + wrap_degrees360(body_yaw - self.release_yaw) * eased;
            if eased >= 1.0 {
                self.managed = false;
                self.releasing = false;
                self.body_yaw = body_yaw;
            }
            self.reset_continuous_turns(look_yaw, time);
            self.last_look_change = time;
            return self.body_yaw;
        }
        // Stationary: the playback runs, in the mode the settings pick.
        self.managed = true;
        self.releasing = false;
        self.dynamic_mode = settings.mode == TurnMode::Dynamic;
        if self.dynamic_mode {
            self.camera_mode = false;
            self.body_yaw = self
                .keyframed_turn
                .update(look_yaw, dt, key_pressed, settings);
            self.weight = if self.keyframed_turn.active() {
                1.0
            } else {
                0.0
            };
            return self.body_yaw;
        }
        if settings.mode == TurnMode::Camera {
            self.camera_mode = true;
            self.body_yaw = self.camera_turn.update(look_yaw, time, dt, settings);
            self.weight = if self.camera_turn.active() { 1.0 } else { 0.0 };
            return self.body_yaw;
        }
        self.camera_mode = false;
        if let Some(clip) = &self.clip {
            let length = clip.length;
            self.time = length.min(self.time + dt);
            self.progress = self.progress.max(rotation_progress(clip, self.time));
            self.body_yaw =
                self.start_yaw + wrap_degrees360(self.target_yaw - self.start_yaw) * self.progress;
            let fade = settings.fade.min(length * 0.2);
            self.weight = self.strength
                * ease_turn((self.time / TURN_ENTRY_SECONDS).min(1.0))
                * ease_turn(((length - self.time) / fade).min(1.0));
            if self.time >= length {
                self.body_yaw = self.target_yaw;
                self.clip = None;
                self.weight = 0.0;
            }
            return self.body_yaw;
        }
        let delta = wrap_degrees360(look_yaw - self.body_yaw);
        if delta.abs() > TURN_ALIGNMENT_EPSILON
            && (key_pressed
                || delta.abs() >= settings.threshold
                || time - self.last_look_change >= settings.delay as f64)
        {
            self.start_yaw = self.body_yaw;
            self.target_yaw = self.body_yaw + delta;
            self.mirrored = settings.mode == TurnMode::Scaled && delta < 0.0;
            self.clip = if settings.mode == TurnMode::Scaled {
                settings.template.clone()
            } else if delta < 0.0 {
                settings.left.clone()
            } else {
                settings.right.clone()
            };
            self.settings_addition = if settings.mode == TurnMode::Scaled {
                None
            } else if delta < 0.0 {
                settings.left_add.clone()
            } else {
                settings.right_add.clone()
            };
            self.strength = (delta.abs() / settings.reference).clamp(0.0, 1.0);
            self.progress = 0.0;
            self.time = 0.0;
            self.weight = 0.0;
        }
        self.body_yaw
    }

    /// `sample`: the pose one leg or arm carries this frame.
    pub fn sample(&self, bone: &str) -> Pose {
        if self.dynamic_mode {
            return self.keyframed_turn.sample(bone);
        }
        if self.camera_mode {
            return self.camera_turn.sample(bone);
        }
        let Some(clip) = &self.clip else {
            return Pose::identity();
        };
        if !self.mirrored {
            return sample_authored(clip, self.settings_addition.as_ref(), bone, self.time);
        }
        let mirrored_bone = match bone {
            "left_leg" => "right_leg",
            "right_leg" => "left_leg",
            "left_arm" => "right_arm",
            "right_arm" => "left_arm",
            other => other,
        };
        let pose = sample_authored(
            clip,
            self.settings_addition.as_ref(),
            mirrored_bone,
            self.time,
        );
        Pose {
            position: Vec3::new(-pose.position.x, pose.position.y, pose.position.z),
            rotation: Vec3::new(pose.rotation.x, -pose.rotation.y, -pose.rotation.z),
            scale: pose.scale,
        }
    }

    /// `sampleAuthored`: the clip's pose plus the settings addition's.
    pub fn sample_authored(clip: &Clip, addition: Option<&Clip>, bone: &str, time: f32) -> Pose {
        sample_authored(clip, addition, bone, time)
    }

    /// `apply`: the pose added onto a part, scaled by `weight`.
    pub fn apply_part(part: &mut PartTransform, pose: &Pose, weight: f32) {
        if weight <= 0.0 {
            return;
        }
        part.x += pose.position.x * weight;
        part.y -= pose.position.y * weight;
        part.z += pose.position.z * weight;
        part.x_rot += pose.rotation.x.to_radians() * weight;
        part.y_rot += pose.rotation.y.to_radians() * weight;
        part.z_rot += pose.rotation.z.to_radians() * weight;
        part.x_scale *= 1.0 + (pose.scale.x - 1.0) * weight;
        part.y_scale *= 1.0 + (pose.scale.y - 1.0) * weight;
        part.z_scale *= 1.0 + (pose.scale.z - 1.0) * weight;
    }

    /// How drawn the turn playback is this frame.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// The body yaw the controller holds.
    pub fn body_yaw(&self) -> f32 {
        self.body_yaw
    }

    fn reset(&mut self, body_yaw: f32, look_yaw: f32, look_pitch: f32, time: f64) {
        self.initialized = true;
        self.last_time = time;
        self.body_yaw = body_yaw;
        self.quiet_yaw = look_yaw;
        self.quiet_pitch = look_pitch;
        self.last_look_change = time;
        self.managed = false;
        self.releasing = false;
        self.weight = 0.0;
        self.time = 0.0;
        self.progress = 0.0;
        self.clip = None;
        self.mirrored = false;
        self.settings_addition = None;
        self.camera_mode = false;
        self.dynamic_mode = false;
        self.reset_continuous_turns(look_yaw, time);
    }

    fn reset_continuous_turns(&mut self, look: f32, time: f64) {
        self.camera_turn.reset(self.body_yaw, look, time);
        self.keyframed_turn.reset(self.body_yaw, look);
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.reset(0.0, 0.0, 0.0, 0.0);
        self.initialized = false;
    }
}

/// `TurnInPlaceController.rotationProgress`.
fn rotation_progress(clip: &Clip, time: f32) -> f32 {
    let start = clip.sample_once("root", 0.0).rotation.y;
    let end = clip.sample_once("root", clip.length).rotation.y;
    if (end - start).abs() < 0.001 {
        ease_turn((time / clip.length).clamp(0.0, 1.0))
    } else {
        ((clip.sample_once("root", time).rotation.y - start) / (end - start)).clamp(0.0, 1.0)
    }
}

/// `TurnInPlaceController.sampleAuthored`.
fn sample_authored(clip: &Clip, addition: Option<&Clip>, bone: &str, time: f32) -> Pose {
    let pose = clip.sample_once(bone, time);
    let Some(addition) = addition else {
        return pose;
    };
    let mut extra = addition.sample_once(bone, time);
    if time > addition.length && clip.length > addition.length {
        let fade =
            ease_turn(((clip.length - time) / (clip.length - addition.length)).clamp(0.0, 1.0));
        extra = Pose {
            position: extra.position * fade,
            rotation: extra.rotation * fade,
            scale: Vec3::new(1.0, 1.0, 1.0).lerp(extra.scale, fade),
        };
    }
    Pose {
        position: pose.position + extra.position,
        rotation: pose.rotation + extra.rotation,
        scale: pose.scale + extra.scale - Vec3::new(1.0, 1.0, 1.0),
    }
}

/// `Mth.lerp(delta, start, end)`.
fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

/// The Java `positiveModulo`: the remainder taken positive.
fn positive_modulo(value: f32, modulus: f32) -> f32 {
    (value % modulus + modulus) % modulus
}

// ---------------------------------------------------------------------------
// DirectionChangeController
// ---------------------------------------------------------------------------

/// `DirectionChangeController.MIN_SPEED`.
pub const DIRECTION_MIN_SPEED: f32 = 0.04;
/// `DirectionChangeController.MAX_DEGREES`.
pub const DIRECTION_MAX_DEGREES: f32 = 3.0;

/// `DirectionChangeController`: the lean the travel direction's changes add,
/// in degrees, per tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DirectionChangeController {
    last_tick: i32,
    slow_ticks: i32,
    tracking: bool,
    travel_x: f32,
    travel_z: f32,
    previous_pitch: f32,
    pitch: f32,
    previous_roll: f32,
    roll: f32,
}

impl DirectionChangeController {
    /// `tick`: `dx`/`dz` the movement since the last tick, `yaw` the player's
    /// yaw in degrees, `alive` whether the player is still there.
    pub fn tick(&mut self, tick: i32, dx: f64, dz: f64, yaw: f32, alive: bool) {
        if tick == self.last_tick {
            return;
        }
        let stale =
            self.last_tick == i32::MIN || tick < self.last_tick || (tick - self.last_tick) > 2;
        self.last_tick = tick;
        if alive && !stale && dx.is_finite() && dz.is_finite() && yaw.is_finite() {
            self.previous_pitch = self.pitch;
            self.previous_roll = self.roll;
            let distance = dx.hypot(dz);
            let mut pitch = 0.0f32;
            let mut roll = 0.0f32;
            if distance >= DIRECTION_MIN_SPEED as f64 {
                let nx = (dx / distance) as f32;
                let nz = (dz / distance) as f32;
                if self.tracking {
                    let turn = nx - self.travel_x;
                    let sideways = nz - self.travel_z;
                    let radians = yaw as f64 * (std::f64::consts::PI / 180.0);
                    let (sin, cos) = radians.sin_cos();
                    let forward = turn * cos as f32 + sideways * sin as f32;
                    let across = -turn * sin as f32 + sideways * cos as f32;
                    let scaled = (distance as f32 / 0.215).clamp(0.0, 1.0);
                    roll = (-forward * 5.0 * scaled)
                        .clamp(-DIRECTION_MAX_DEGREES, DIRECTION_MAX_DEGREES);
                    pitch = (-across * 5.0 * scaled)
                        .clamp(-DIRECTION_MAX_DEGREES, DIRECTION_MAX_DEGREES);
                } else {
                    self.travel_x = nx;
                    self.travel_z = nz;
                    self.tracking = true;
                }
                self.travel_x = lerp(0.45, self.travel_x, nx);
                self.travel_z = lerp(0.45, self.travel_z, nz);
                self.slow_ticks = 0;
            } else {
                self.slow_ticks += 1;
                if self.slow_ticks > 3 {
                    self.tracking = false;
                }
            }
            self.pitch = direction_approach(self.pitch, pitch);
            self.roll = direction_approach(self.roll, roll);
        } else {
            self.reset();
        }
    }

    /// `pitch` between the last two ticks.
    pub fn pitch(&self, partial: f32) -> f32 {
        lerp(partial.clamp(0.0, 1.0), self.previous_pitch, self.pitch)
    }

    /// `roll` between the last two ticks.
    pub fn roll(&self, partial: f32) -> f32 {
        lerp(partial.clamp(0.0, 1.0), self.previous_roll, self.roll)
    }

    fn reset(&mut self) {
        self.tracking = false;
        self.slow_ticks = 0;
        self.previous_pitch = 0.0;
        self.pitch = 0.0;
        self.previous_roll = 0.0;
        self.roll = 0.0;
    }
}

/// `DirectionChangeController.approach`.
fn direction_approach(current: f32, target: f32) -> f32 {
    let out = lerp(0.4, current, target);
    if out.abs() < 0.001 { 0.0 } else { out }
}

// ---------------------------------------------------------------------------
// TurnLeanController
// ---------------------------------------------------------------------------

/// `TurnLeanController.MAX_DEGREES`.
pub const TURN_LEAN_MAX_DEGREES: f32 = 14.0;

/// `TurnLeanController`: the sideways lean the look turn adds while the
/// player is visible or holding jump.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TurnLeanController {
    yaw: f32,
    previous: f32,
    current: f32,
    last_tick: i32,
    visible: bool,
    holding_jump: bool,
}

impl TurnLeanController {
    /// `tick`: `yaw` the look yaw, `speed` the walk speed, `x_input`/`z_input`
    /// the movement inputs, `visible` whether the player is on screen,
    /// `jumping` whether the jump key is pressed, `holding_jump` whether it
    /// is held.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        tick: i32,
        yaw: f32,
        speed: f32,
        x_input: f32,
        z_input: f32,
        visible: bool,
        jumping: bool,
        holding_jump: bool,
    ) {
        if tick == self.last_tick {
            return;
        }
        if yaw.is_finite()
            && self.last_tick != i32::MIN
            && tick >= self.last_tick
            && tick - self.last_tick <= 2
        {
            let turned = wrap_degrees360(yaw - self.yaw);
            self.yaw = yaw;
            self.last_tick = tick;
            self.previous = self.current;
            self.holding_jump = holding_jump && (self.holding_jump || jumping && self.visible);
            self.visible = visible || self.holding_jump;
            if !self.holding_jump {
                let mut target = 0.0f32;
                if visible && speed.is_finite() && x_input.is_finite() && z_input.is_finite() {
                    let scaled = ((speed - 0.01) / 0.205).clamp(0.0, 1.25);
                    let from_turn = if turned.abs() <= 60.0 {
                        -turned * 1.4 * x_input.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    };
                    let from_strafe = 6.0 * z_input.clamp(-1.0, 1.0);
                    target = ((from_turn + from_strafe) * scaled)
                        .clamp(-TURN_LEAN_MAX_DEGREES, TURN_LEAN_MAX_DEGREES);
                }
                let rate = if target == 0.0 { 0.3 } else { 0.38 };
                self.current += (target - self.current) * rate;
                if self.current.abs() < 0.001 {
                    self.current = 0.0;
                }
            }
        } else {
            self.yaw = if yaw.is_finite() { yaw } else { 0.0 };
            self.previous = 0.0;
            self.current = 0.0;
            self.visible = false;
            self.holding_jump = false;
            self.last_tick = tick;
        }
    }

    /// `sample` between the last two ticks.
    pub fn sample(&self, partial: f32) -> f32 {
        if self.visible {
            lerp(partial.clamp(0.0, 1.0), self.previous, self.current)
        } else {
            0.0
        }
    }

    /// `apply`: the lean rotates a part about the world Y through the part's
    /// own rotation, keeping the head's top in place; `extra` adds degrees
    /// and `extra_z` a sideways offset in degrees.
    pub fn apply_part(&self, part: &mut PartTransform, partial: f32, extra_z: f32, extra: f32) {
        let degrees =
            (self.sample(partial) + extra).clamp(-TURN_LEAN_MAX_DEGREES, TURN_LEAN_MAX_DEGREES);
        if degrees.abs() < 0.001 && extra_z.abs() < 0.001 {
            return;
        }
        let own = Quat::from_zyx_degrees(Vec3::new(
            part.x_rot.to_degrees(),
            part.y_rot.to_degrees(),
            part.z_rot.to_degrees(),
        ));
        let leaned = Quat::from_zyx_degrees(Vec3::new(degrees, 0.0, extra_z));
        let pivot = Vec3::new(0.0, 24.0 * part.y_scale, 0.0);
        let moved = own.transform(pivot) - leaned.transform(pivot);
        part.x += moved.x;
        part.y += moved.y;
        part.z += moved.z;
        let euler = leaned.to_euler_zyx_degrees();
        part.x_rot = euler.x.to_radians();
        part.y_rot = euler.y.to_radians();
        part.z_rot = euler.z.to_radians();
    }
}

// ---------------------------------------------------------------------------
// CreativeFlightState
// ---------------------------------------------------------------------------

/// `CreativeFlightState.Motion`: the hover detector for remote players in
/// creative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreativeFlightMotion {
    deltas: [f64; 8],
    tick: i32,
    y: f64,
    count: usize,
    cursor: usize,
    flying: bool,
}

impl Default for CreativeFlightMotion {
    fn default() -> Self {
        Self {
            deltas: [0.0; 8],
            tick: i32::MIN,
            y: 0.0,
            count: 0,
            cursor: 0,
            flying: false,
        }
    }
}

impl CreativeFlightMotion {
    /// `Motion.update`: `y` the player's height, `eligible` whether every
    /// other flying reason is out.
    pub fn update(&mut self, tick: i32, y: f64, eligible: bool) -> bool {
        if !eligible || self.tick == i32::MIN || tick < self.tick || tick - self.tick > 2 {
            self.count = 0;
            self.cursor = 0;
            self.flying = false;
            self.tick = tick;
            self.y = y;
            return false;
        }
        if tick == self.tick {
            return self.flying;
        }
        self.deltas[self.cursor] = (y - self.y) / (tick - self.tick) as f64;
        self.cursor = (self.cursor + 1) % self.deltas.len();
        self.count = self.deltas.len().min(self.count + 1);
        self.tick = tick;
        self.y = y;
        let lowest = self.deltas[..self.count]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let highest = self.deltas[..self.count]
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        if self.count == self.deltas.len()
            && highest - lowest < 0.035
            && lowest > -0.65
            && highest < 0.65
        {
            self.flying = true;
        } else if self.flying {
            // A steady descent ends the hover: the three newest deltas
            // (just behind the cursor) sink faster than the three before
            // them.
            let mut newest = 0.0;
            let mut older = 0.0;
            for j in 0..3 {
                older += self.deltas[(self.cursor + j) % self.deltas.len()] / 3.0;
                newest += self.deltas
                    [(self.cursor + self.deltas.len() - 3 + j) % self.deltas.len()]
                    / 3.0;
            }
            if newest < -0.2 && newest - older < -0.18 {
                self.flying = false;
            }
        }
        self.flying
    }

    /// Whether the hover is on.
    pub fn flying(&self) -> bool {
        self.flying
    }
}

// ---------------------------------------------------------------------------
// ElytraFlightAlignment
// ---------------------------------------------------------------------------

/// `ElytraFlightAlignment.MESH_SCALE`.
pub const ELYTRA_MESH_SCALE: f32 = 0.9375;
/// `ElytraFlightAlignment.MODEL_BASE`.
pub const ELYTRA_MODEL_BASE: f32 = 1.501;

/// `ElytraFlightAlignment`: the glide pose's height matched to the hitbox.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElytraFlightAlignment {
    normal_height: f32,
    fast_height: f32,
    target_height: f32,
    configured: bool,
}

impl ElytraFlightAlignment {
    /// `apply`: the model part's y moves by the difference between the
    /// hitbox's half height and the pose's reference height.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        part: &mut PartTransform,
        hitbox_height: Option<f32>,
        pitch_scale: f32,
        weight: f32,
        blend: f32,
        normal: Option<&Clip>,
        fast: Option<&Clip>,
    ) {
        if let Some(height) = hitbox_height
            && height.is_finite()
        {
            self.target_height = height * 0.5;
        }
        let Some(_) = hitbox_height else {
            return;
        };
        // The reference heights need both clips; without them there is no
        // correction.
        let (Some(rotate), Some(fast_or_normal)) = (normal, fast.or(normal)) else {
            return;
        };
        if !self.configured {
            self.normal_height = reference_height_pair(rotate, normal);
            self.fast_height = reference_height_pair(rotate, fast);
            self.configured = true;
        }
        let _ = fast_or_normal;
        if weight <= 1.0e-5 || pitch_scale <= 1.0e-5 {
            return;
        }
        let reference = lerp(blend.clamp(0.0, 1.0), self.normal_height, self.fast_height) * weight;
        let offset = (self.target_height - reference) * pitch_scale.clamp(0.0, 1.0);
        part.y -= offset * 16.0 / (ELYTRA_MESH_SCALE * weight);
    }
}

/// `ElytraFlightAlignment.referenceHeight`: the pose's torso height over the
/// model base, averaged over 64 samples.
fn reference_height_pair(rotate: &Clip, over: Option<&Clip>) -> f32 {
    let mut total = 0.0f32;
    for i in 0..64 {
        let over_time = over.map(|clip| clip.length * i as f32 / 64.0);
        let mut matrix = Mat4Translate::new(0.0, -ELYTRA_MODEL_BASE, 0.0);
        apply_pose_transform(
            &mut matrix,
            rotate.sample("root", rotate.length),
            over.map(|clip| clip.sample("root", over_time.unwrap_or(0.0))),
        );
        apply_pose_transform(
            &mut matrix,
            rotate.sample("body", rotate.length),
            over.map(|clip| clip.sample("body", over_time.unwrap_or(0.0))),
        );
        total -= matrix.transform_position(0.0, 0.375, 0.0)[1] * ELYTRA_MESH_SCALE;
    }
    total / 64.0
}

/// A tiny translate/rotate/scale matrix chain, the Joml calls the reference
/// height math makes.
#[derive(Debug, Clone, Copy)]
struct Mat4Translate {
    translation: [f32; 3],
    rotation: Quat,
    scale: [f32; 3],
}

impl Mat4Translate {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: [x, y, z],
            rotation: Quat::IDENTITY,
            scale: [1.0, 1.0, 1.0],
        }
    }

    fn transform_position(&self, x: f32, y: f32, z: f32) -> [f32; 3] {
        let v = Vec3::new(x * self.scale[0], y * self.scale[1], z * self.scale[2]);
        let rotated = self.rotation.transform(v);
        [
            rotated.x + self.translation[0],
            rotated.y + self.translation[1],
            rotated.z + self.translation[2],
        ]
    }
}

/// `ElytraFlightAlignment.transform`: the base pose plus the overlay's.
fn apply_pose_transform(matrix: &mut Mat4Translate, base: Pose, over: Option<Pose>) {
    let mut position = base.position;
    let mut rotation = base.rotation;
    let mut scale = base.scale;
    if let Some(over) = over {
        position = position + over.position;
        rotation = rotation + over.rotation;
        scale = scale + over.scale - Vec3::new(1.0, 1.0, 1.0);
    }
    matrix.translation[0] += position.x / 16.0;
    matrix.translation[1] += -position.y / 16.0;
    matrix.translation[2] += position.z / 16.0;
    matrix.rotation = Quat::from_zyx_degrees(rotation).mul(matrix.rotation);
    matrix.scale = [
        matrix.scale[0] * scale.x,
        matrix.scale[1] * scale.y,
        matrix.scale[2] * scale.z,
    ];
}

// ---------------------------------------------------------------------------
// SwimSurfaceAlignment
// ---------------------------------------------------------------------------

/// `SwimSurfaceAlignment.DT`.
pub const SWIM_DT: f32 = 0.05;
/// `SwimSurfaceAlignment.EYE_ABOVE_TORSO`.
pub const SWIM_EYE_ABOVE_TORSO: f32 = 0.2;
/// The depth the alignment falls back to.
pub const SWIM_DEFAULT_DEPTH: f32 = 0.12;

/// `SwimSurfaceAlignment`: keeps the swimming body at the water surface.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SwimSurfaceAlignment {
    reference_torso_height: f32,
    has_reference: bool,
    eye_initialized: bool,
    previous_eye_height: f32,
    eye_height: f32,
    previous_weight: f32,
    weight: f32,
    previous_surface: f32,
    surface: f32,
    previous_water_y: f64,
    water_y: f64,
    depth: f32,
}

impl SwimSurfaceAlignment {
    /// `prepareEyeHeight`.
    pub fn prepare_eye_height(&mut self, height: f32) {
        if height.is_finite() {
            if !self.eye_initialized {
                self.previous_eye_height = height;
                self.eye_height = height;
                self.eye_initialized = true;
            } else {
                self.previous_eye_height = self.eye_height;
                self.eye_height = lerp(1.0 - (-0.5f32).exp(), self.eye_height, height);
            }
        }
    }

    /// `tick`.
    pub fn tick(&mut self, swimming: bool, at_surface: bool, water_y: f64, depth: f32) {
        self.previous_weight = self.weight;
        self.previous_surface = self.surface;
        self.previous_water_y = self.water_y;
        self.weight = swim_approach(
            self.weight,
            if self.has_reference && swimming {
                1.0
            } else {
                0.0
            },
            0.25,
        );
        if swimming && self.has_reference {
            self.surface = swim_approach(self.surface, if at_surface { 1.0 } else { 0.0 }, 0.25);
        }
        self.depth = if depth.is_finite() {
            depth.max(0.0)
        } else {
            SWIM_DEFAULT_DEPTH
        };
        if swimming && self.has_reference && water_y.is_finite() {
            self.water_y = if self.water_y.is_finite() {
                self.water_y + (water_y - self.water_y) * (1.0 - (-0.5f64).exp())
            } else {
                water_y
            };
            if !self.previous_water_y.is_finite() {
                self.previous_water_y = self.water_y;
            }
        }
    }

    /// Sets the reference clip's torso height (the glue measures it once).
    pub fn set_reference_height(&mut self, height: f32) {
        self.reference_torso_height = height;
        self.has_reference = true;
    }

    /// `active`.
    pub fn active(&self) -> bool {
        self.weight > 1.0e-4 || self.previous_weight > 1.0e-4
    }

    /// `worldOffset`.
    pub fn world_offset(&self, player_y: f64, scale: f32, partial: f32) -> f32 {
        if !(self.eye_initialized && self.has_reference && scale > 1.0e-5) {
            return 0.0;
        }
        let amount = self.amount(partial);
        if amount <= 0.0 {
            return 0.0;
        }
        let eye = lerp(partial, self.previous_eye_height, self.eye_height);
        let torso = eye - SWIM_EYE_ABOVE_TORSO * scale;
        let water = if self.water_y.is_finite() && self.previous_water_y.is_finite() {
            self.previous_water_y + (self.water_y - self.previous_water_y) * partial as f64
        } else {
            f64::NAN
        };
        let surface = if water.is_finite() {
            smooth_swim(lerp(partial, self.previous_surface, self.surface))
        } else {
            0.0
        };
        let above = (water - player_y) as f32 - self.depth * scale;
        let offset = if surface <= 0.0 {
            torso
        } else {
            lerp(surface, torso, above)
        };
        (offset - self.reference_torso_height * scale) * amount
    }

    /// `apply`: the model part's y moves by the world offset, in model units.
    pub fn apply(&self, part: &mut PartTransform, player_y: f64, scale: f32, partial: f32) {
        let offset = self.world_offset(player_y, scale, partial);
        part.y -= offset * 16.0 / (ELYTRA_MESH_SCALE * scale.max(1.0e-5));
    }

    /// `cameraEyeHeight`.
    pub fn camera_eye_height(&self, player_y: f64, scale: f32, partial: f32, standing: f32) -> f32 {
        let raised = (self.reference_torso_height + SWIM_EYE_ABOVE_TORSO) * scale;
        lerp(self.amount(partial), standing, raised) + self.world_offset(player_y, scale, partial)
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.previous_weight = 0.0;
        self.weight = 0.0;
        self.previous_surface = 0.0;
        self.surface = 0.0;
        self.previous_water_y = f64::NAN;
        self.water_y = f64::NAN;
        self.eye_initialized = false;
    }

    fn amount(&self, partial: f32) -> f32 {
        smooth_swim(lerp(partial, self.previous_weight, self.weight))
    }
}

/// `SwimSurfaceAlignment.referenceHeight`: the reference clip's torso height.
pub fn swim_reference_height(clip: &Clip) -> f32 {
    let mut total = 0.0f32;
    for i in 0..64 {
        let time = clip.length * i as f32 / 64.0;
        let mut matrix = Mat4Translate::new(0.0, -ELYTRA_MODEL_BASE, 0.0);
        apply_pose_transform(&mut matrix, clip.sample("root", time), None);
        apply_pose_transform(&mut matrix, clip.sample("body", time), None);
        total += -matrix.transform_position(0.0, 0.375, 0.0)[1] * ELYTRA_MESH_SCALE;
    }
    total / 64.0
}

fn swim_approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        target.min(current + step)
    } else {
        target.max(current - step)
    }
}

fn smooth_swim(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

// ---------------------------------------------------------------------------
// FishingLinePhysics
// ---------------------------------------------------------------------------

/// `FishingLinePhysics.GRAVITY`.
pub const FISHING_GRAVITY: f64 = 16.0;
/// `FishingLinePhysics.STEP`: one integration step, 1/120 s.
pub const FISHING_STEP: f64 = 0.008_333_333_333_333_333;
/// `FishingLinePhysics.LONGEST_GAP`: the most seconds one frame may simulate.
pub const FISHING_LONGEST_GAP: f64 = 0.25;
/// `FishingLinePhysics.KEEP`: how much of the bob's velocity each step keeps.
pub fn fishing_keep() -> f64 {
    (-0.013_333_333_333_333_334f64).exp()
}

/// `FishingLinePhysics.Swing`: the bob's verlet-ish fall along the line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FishingSwing {
    bob: [f64; 3],
    previous: [f64; 3],
    last_pivot: [f64; 3],
    time: f64,
    carried: f64,
}

impl Default for FishingSwing {
    fn default() -> Self {
        Self {
            bob: [0.0; 3],
            previous: [0.0; 3],
            last_pivot: [0.0; 3],
            time: f64::NAN,
            carried: 0.0,
        }
    }
}

impl FishingSwing {
    /// `update`: `pivot` where the rod tip is, `length` the line's length,
    /// `time` the animation time; returns the unit vector from the pivot to
    /// the bob.
    pub fn update(&mut self, pivot: [f64; 3], length: f64, time: f64) -> [f64; 3] {
        let fresh = self.time.is_nan()
            || time < self.time
            || time - self.time > FISHING_LONGEST_GAP
            || length <= 1.0e-6;
        if fresh {
            self.bob = [pivot[0], pivot[1] - length.max(0.0), pivot[2]];
            self.previous = self.bob;
            self.last_pivot = pivot;
            self.time = time;
            self.carried = 0.0;
            return [0.0, -1.0, 0.0];
        }
        let elapsed = time - self.time + self.carried;
        let steps = (elapsed / FISHING_STEP) as i32;
        self.carried = elapsed - steps as f64 * FISHING_STEP;
        self.time = time;
        let keep = fishing_keep();
        for step in 1..=steps {
            let t = step as f64 / steps as f64;
            let anchor = [
                self.last_pivot[0] + (pivot[0] - self.last_pivot[0]) * t,
                self.last_pivot[1] + (pivot[1] - self.last_pivot[1]) * t,
                self.last_pivot[2] + (pivot[2] - self.last_pivot[2]) * t,
            ];
            let velocity = [
                (self.bob[0] - self.previous[0]) * keep,
                (self.bob[1] - self.previous[1]) * keep,
                (self.bob[2] - self.previous[2]) * keep,
            ];
            self.previous = self.bob;
            self.bob[0] += velocity[0];
            self.bob[1] += velocity[1] - 0.001_111_111_111_111_111_1;
            self.bob[2] += velocity[2];
            fishing_pull_in(&mut self.bob, &anchor, length);
        }
        if steps > 0 {
            self.last_pivot = pivot;
        }
        let mut out = [
            self.bob[0] - pivot[0],
            self.bob[1] - pivot[1],
            self.bob[2] - pivot[2],
        ];
        let len = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
        if len < 1.0e-9 {
            out = [0.0, -1.0, 0.0];
        } else {
            out = [out[0] / len, out[1] / len, out[2] / len];
        }
        out
    }
}

/// `FishingLinePhysics.pullIn`.
fn fishing_pull_in(bob: &mut [f64; 3], anchor: &[f64; 3], length: f64) {
    let dx = bob[0] - anchor[0];
    let dy = bob[1] - anchor[1];
    let dz = bob[2] - anchor[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if distance > length {
        let scale = length / distance;
        *bob = [
            anchor[0] + dx * scale,
            anchor[1] + dy * scale,
            anchor[2] + dz * scale,
        ];
    }
}

// ---------------------------------------------------------------------------
// BoatPaddleAnimation
// ---------------------------------------------------------------------------

/// `BoatPaddleAnimation.STEP`: a quarter paddle stroke, in radians.
pub const PADDLE_STEP: f32 = std::f32::consts::PI / 8.0;
/// The phase magnitude the paddles wrap at.
const PADDLE_PHASE_WRAP: f32 = 100.53097;
/// How much weight a paddling boat gains per tick.
const PADDLE_WEIGHT_PER_TICK: f32 = 0.2;

/// `BoatPaddleAnimation.Pose`: one paddle's pitch and yaw, in radians.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PaddlePose {
    pub pitch: f32,
    pub yaw: f32,
}

/// `BoatPaddleAnimation.vanillaPose`: the vanilla paddle angle for one side.
pub fn paddle_vanilla_pose(side: i32, phase: f32) -> PaddlePose {
    let pitch = lerp(
        ((-phase).sin() + 1.0) / 2.0,
        -std::f32::consts::PI / 3.0,
        -std::f32::consts::PI / 12.0,
    );
    let yaw = lerp(
        ((-phase + 1.0).sin() + 1.0) / 2.0,
        -std::f32::consts::PI / 4.0,
        std::f32::consts::PI / 4.0,
    );
    PaddlePose {
        pitch,
        yaw: if side == 1 {
            std::f32::consts::PI - yaw
        } else {
            yaw
        },
    }
}

/// `BoatPaddleAnimation.oppositeSide`.
pub fn paddle_opposite_side(pose: PaddlePose) -> PaddlePose {
    PaddlePose {
        pitch: pose.pitch,
        yaw: std::f32::consts::PI - pose.yaw,
    }
}

/// `BoatPaddleAnimation.Paddle`: one paddle's eased stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paddle {
    side: i32,
    direction: i32,
    phase_end: f32,
    offset: PaddlePose,
    velocity_offset: PaddlePose,
    previous_time: f32,
    time: f32,
}

impl Paddle {
    /// `Paddle(side, paddling, phase)`.
    pub fn new(side: i32, direction: i32, phase_end: f32) -> Self {
        Self {
            side,
            direction,
            phase_end,
            offset: PaddlePose::default(),
            velocity_offset: PaddlePose::default(),
            previous_time: 0.3,
            time: 0.3,
        }
    }

    /// `tick`.
    pub fn tick(&mut self, direction: i32, phase_end: f32) {
        if direction != self.direction {
            let pose = self.sample(1.0);
            let velocity = self.velocity(1.0);
            self.direction = direction;
            self.phase_end = phase_end;
            let target = self.target(0.0);
            let target_velocity = self.target_velocity(0.0);
            self.offset = PaddlePose {
                pitch: pose.pitch - target.pitch,
                yaw: pose.yaw - target.yaw,
            };
            self.velocity_offset = PaddlePose {
                pitch: velocity.pitch - target_velocity.pitch,
                yaw: velocity.yaw - target_velocity.yaw,
            };
            self.previous_time = 0.0;
            self.time = 0.05;
        } else {
            self.previous_time = self.time;
            self.time = (self.time + 0.05).min(0.3);
        }
        self.phase_end = phase_end;
    }

    /// `sample`.
    pub fn sample(&self, partial: f32) -> PaddlePose {
        let partial = partial.clamp(0.0, 1.0);
        let target = self.target(partial);
        let progress = self.progress(partial);
        let hold =
            1.0 - progress * progress * progress * (10.0 + progress * (-15.0 + 6.0 * progress));
        let drift = self.duration()
            * (progress
                - progress * progress * progress * (6.0 + progress * (-8.0 + 3.0 * progress)));
        PaddlePose {
            pitch: target.pitch + self.offset.pitch * hold + self.velocity_offset.pitch * drift,
            yaw: target.yaw + self.offset.yaw * hold + self.velocity_offset.yaw * drift,
        }
    }

    /// `velocity`.
    pub fn velocity(&self, partial: f32) -> PaddlePose {
        let target = self.target_velocity(partial);
        let progress = self.progress(partial);
        let swing =
            -30.0 * progress * progress * (1.0 - progress) * (1.0 - progress) / self.duration();
        let hold = 1.0 - progress * progress * (18.0 + progress * (-32.0 + 15.0 * progress));
        PaddlePose {
            pitch: target.pitch + self.offset.pitch * swing + self.velocity_offset.pitch * hold,
            yaw: target.yaw + self.offset.yaw * swing + self.velocity_offset.yaw * hold,
        }
    }

    fn duration(&self) -> f32 {
        if self.direction != 0 { 0.15 } else { 0.3 }
    }

    fn progress(&self, partial: f32) -> f32 {
        (lerp(partial, self.previous_time, self.time) / self.duration()).clamp(0.0, 1.0)
    }

    fn phase(&self, partial: f32) -> f32 {
        lerp(
            partial,
            self.phase_end - self.direction as f32 * PADDLE_STEP,
            self.phase_end,
        )
    }

    fn target(&self, partial: f32) -> PaddlePose {
        paddle_vanilla_pose(
            self.side,
            if self.direction != 0 {
                self.phase(partial)
            } else {
                0.0
            },
        )
    }

    fn target_velocity(&self, partial: f32) -> PaddlePose {
        if self.direction == 0 {
            return PaddlePose::default();
        }
        let phase = self.phase(partial);
        let rate = self.direction as f32 * PADDLE_STEP / 0.05;
        let pitch = -PADDLE_STEP * (-phase).cos() * rate;
        let yaw = -std::f32::consts::PI / 4.0 * (-phase + 1.0).cos() * rate;
        PaddlePose {
            pitch,
            yaw: if self.side == 1 { -yaw } else { yaw },
        }
    }
}

/// `BoatPaddleAnimation.Animation`: both paddles and the rowing weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatAnimation {
    paddles: [Paddle; 2],
    phase: [f32; 2],
    tick: i32,
    rower_id: i32,
    previous_weight: f32,
    weight: f32,
}

impl BoatAnimation {
    /// `new Animation(boat)`: `paddling` per side, `phase` per side, `chest`
    /// whether the boat is a chest boat (its paddles run mirrored).
    pub fn new(
        rower_id: i32,
        tick: i32,
        paddling: [bool; 2],
        phases: [f32; 2],
        chest: bool,
    ) -> Self {
        let sign = if chest { -1.0 } else { 1.0 };
        let mut paddles = [Paddle::new(0, 0, 0.0), Paddle::new(1, 0, 0.0)];
        let mut phase = [0.0f32; 2];
        for k in 0..2 {
            phase[k] = phases[k] * sign;
            paddles[k] = Paddle::new(
                k as i32,
                if paddling[k] { sign as i32 } else { 0 },
                phase[k],
            );
        }
        Self {
            paddles,
            phase,
            tick,
            rower_id,
            previous_weight: 0.0,
            weight: 0.0,
        }
    }

    /// `animation(boat)`'s per-tick step: `ticks` the boat's ticks, `backing`
    /// whether the rower is steering backwards, `paddle_state` per side.
    pub fn step(
        &mut self,
        ticks: i32,
        rower_id: i32,
        backing: bool,
        paddle_state: [bool; 2],
        chest: bool,
    ) {
        let passed = ticks - self.tick;
        if !(0..=40).contains(&passed) || self.rower_id != rower_id {
            *self = BoatAnimation::new(rower_id, ticks, paddle_state, self.phase, chest);
            return;
        }
        if passed > 0 {
            for _ in 0..passed {
                self.previous_weight = self.weight;
                self.weight = (self.weight + PADDLE_WEIGHT_PER_TICK).min(1.0);
                for (k, stroking) in paddle_state.into_iter().enumerate() {
                    let mut direction = if stroking {
                        1
                    } else if backing {
                        -1
                    } else {
                        0
                    };
                    if chest {
                        direction = -direction;
                    }
                    self.phase[k] += direction as f32 * PADDLE_STEP;
                    if self.phase[k].abs() > PADDLE_PHASE_WRAP {
                        self.phase[k] %= std::f32::consts::TAU;
                    }
                    self.paddles[k].tick(direction, self.phase[k]);
                }
            }
            self.tick = ticks;
        }
    }

    /// `weight`: the eased rowing weight at `partial`.
    pub fn weight(&self, partial: f32) -> f32 {
        let f = lerp(partial.clamp(0.0, 1.0), self.previous_weight, self.weight);
        f * f * f * (10.0 + f * (-15.0 + 6.0 * f))
    }

    /// The two paddles' samples at `partial`.
    pub fn sample(&self, partial: f32) -> [PaddlePose; 2] {
        [
            self.paddles[0].sample(partial),
            self.paddles[1].sample(partial),
        ]
    }
}

/// `BoatPaddleAnimation.RiderMotion.Response`: the critically damped follow.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MotionResponse {
    rate: f32,
    minimum: f32,
    maximum: f32,
    time: f64,
    value: f32,
    velocity: f32,
}

impl MotionResponse {
    fn new(rate: f32, minimum: f32, maximum: f32) -> Self {
        Self {
            rate,
            minimum,
            maximum,
            time: f64::NAN,
            value: 0.0,
            velocity: 0.0,
        }
    }

    fn update(&mut self, target: f32, time: f64) -> f32 {
        if !(target.is_finite() && time.is_finite()) {
            return self.value;
        }
        let dt = time - self.time;
        if !self.time.is_finite() || dt < 0.0 || dt > 0.5 {
            self.time = time;
            self.value = target;
            self.velocity = 0.0;
            return self.value;
        }
        if dt == 0.0 {
            return self.value;
        }
        let error = self.value - target;
        let decay = (-self.rate as f64 * dt).exp() as f32;
        let carried = (self.velocity + self.rate * error) * dt as f32;
        self.value = target + (error + carried) * decay;
        self.velocity = (self.velocity - self.rate * carried) * decay;
        self.value = self.value.clamp(self.minimum, self.maximum);
        self.time = time;
        self.value
    }
}

/// `BoatPaddleAnimation.RiderMotion`: how the rower's body follows the boat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiderMotion {
    torso: MotionResponse,
    pitch: [MotionResponse; 2],
    yaw: [MotionResponse; 2],
}

impl Default for RiderMotion {
    fn default() -> Self {
        Self {
            torso: MotionResponse::new(16.0, -std::f32::consts::PI / 4.0, 0.610_865_24),
            pitch: [
                MotionResponse::new(20.0, -1.2, 1.2),
                MotionResponse::new(20.0, -1.2, 1.2),
            ],
            yaw: [
                MotionResponse::new(20.0, -1.2, 1.2),
                MotionResponse::new(20.0, -1.2, 1.2),
            ],
        }
    }
}

impl RiderMotion {
    /// `paddle`: the rider's paddle pose following the boat's.
    pub fn paddle(
        &mut self,
        side: usize,
        boat: PaddlePose,
        rider: PaddlePose,
        time: f64,
    ) -> PaddlePose {
        PaddlePose {
            pitch: boat.pitch + self.pitch[side].update(rider.pitch - boat.pitch, time),
            yaw: boat.yaw + self.yaw[side].update(rider.yaw - boat.yaw, time),
        }
    }

    /// `torso`.
    pub fn torso(&mut self, target: f32, time: f64) -> f32 {
        self.torso.update(target, time)
    }
}

// ---------------------------------------------------------------------------
// ElytraOuterRotation, HeldItemAnimation
// ---------------------------------------------------------------------------

/// `ElytraOuterRotation.correctionDegrees`: the extra pitch the gliding body
/// needs so the wings meet the flight path.
pub fn elytra_correction_degrees(flying: bool, gliding: bool, paused: bool, weight: f32) -> f32 {
    if flying && gliding && !paused && weight.is_finite() {
        90.0 * weight.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// `HeldItemAnimation`: the matrix a held item's model is drawn through, from
/// the locomotion pose's position, rotation and scale (model units).
pub fn held_item_matrix(position: Vec3, rotation: Vec3, scale: Vec3) -> Mat4Builder {
    let mut out = Mat4Builder::identity();
    // The Java translates by (-x, -z, y) / 16.
    out = out.translate(-position.x / 16.0, -position.z / 16.0, position.y / 16.0);
    if rotation.length_squared() > 1.0e-6 {
        let q = Quat::from_zyx_degrees(Vec3::new(-rotation.x, -rotation.y, -rotation.z));
        out = out.rotate(q);
    }
    out.scale(scale.x.max(0.001), scale.y.max(0.001), scale.z.max(0.001))
}

/// A small matrix builder for the held-item transform, column-major.
pub struct Mat4Builder([f32; 16]);

impl Mat4Builder {
    /// The identity.
    pub fn identity() -> Self {
        let mut values = [0.0f32; 16];
        values[0] = 1.0;
        values[5] = 1.0;
        values[10] = 1.0;
        values[15] = 1.0;
        Self(values)
    }

    /// The raw column-major values.
    pub fn values(&self) -> &[f32; 16] {
        &self.0
    }

    /// Starts from an existing column-major matrix.
    pub fn from_matrix(values: &[f32; 16]) -> Self {
        Self(*values)
    }

    fn multiply(&self, other: &[f32; 16]) -> [f32; 16] {
        let a = crate::camera::Mat4::from_column_major(&self.0);
        let b = crate::camera::Mat4::from_column_major(other);
        let out = a.mul(&b);
        let mut values = [0.0f32; 16];
        for c in 0..4 {
            for r in 0..4 {
                values[c * 4 + r] = out.columns[c][r];
            }
        }
        values
    }

    /// `translate`.
    pub fn translate(self, x: f32, y: f32, z: f32) -> Self {
        let mut t = [0.0f32; 16];
        t[0] = 1.0;
        t[5] = 1.0;
        t[10] = 1.0;
        t[15] = 1.0;
        t[12] = x;
        t[13] = y;
        t[14] = z;
        Self(self.multiply(&t))
    }

    /// `rotate`.
    pub fn rotate(self, q: Quat) -> Self {
        let m = q.to_matrix3();
        let mut r = [0.0f32; 16];
        // Column-major rotation matrix.
        r[0] = m[0][0];
        r[1] = m[1][0];
        r[2] = m[2][0];
        r[4] = m[0][1];
        r[5] = m[1][1];
        r[6] = m[2][1];
        r[8] = m[0][2];
        r[9] = m[1][2];
        r[10] = m[2][2];
        r[15] = 1.0;
        Self(self.multiply(&r))
    }

    /// `scale`.
    pub fn scale(self, x: f32, y: f32, z: f32) -> Self {
        let mut s = [0.0f32; 16];
        s[0] = x;
        s[5] = y;
        s[10] = z;
        s[15] = 1.0;
        Self(self.multiply(&s))
    }
}

// ---------------------------------------------------------------------------
// PoseOwnershipBlend, OuterPoseOwnershipBlend
// ---------------------------------------------------------------------------

/// One ownership track's state.
#[derive(Debug, Clone, PartialEq)]
struct OwnershipTrack {
    last: Option<HashMap<String, [f32; 9]>>,
    source: Option<HashMap<String, [f32; 9]>>,
    external: bool,
    enabled: bool,
    time: f32,
    started: f32,
    duration: f32,
}

impl Default for OwnershipTrack {
    fn default() -> Self {
        Self {
            last: None,
            source: None,
            external: false,
            enabled: false,
            time: f32::NAN,
            started: 0.0,
            duration: 0.0,
        }
    }
}

/// `PoseOwnershipBlend`: the mod's pose hands the parts over to vanilla (or
/// takes them back) through a smooth blend, per part, reading and writing the
/// nine floats a model part carries.
#[derive(Debug, Clone, Default)]
pub struct PoseOwnershipBlend {
    tracks: HashMap<u64, OwnershipTrack>,
}

impl PoseOwnershipBlend {
    /// `apply`: `parts` are the nine floats each model part carries
    /// (position, rotation, scale), `external` whether the mod owns them
    /// this frame, `time` the animation time, `enabled` whether blending is
    /// on at all.
    pub fn apply(
        &mut self,
        key: u64,
        parts: &mut [(&'static str, &mut [f32; 9])],
        external: bool,
        time: f32,
        enabled: bool,
    ) {
        let track = self.tracks.entry(key).or_default();
        let continuous = time.is_finite()
            && track.time.is_finite()
            && time >= track.time
            && time - track.time <= 5.0
            && track.last.is_some();
        if !enabled || !track.enabled || !continuous {
            track.source = None;
        } else if external != track.external {
            track.source = track.last.take();
            track.started = time;
            track.duration = if external { 3.0 } else { 4.0 };
        }
        let mut weight = match &track.source {
            None => 1.0,
            Some(_) => ((time - track.started) / track.duration).clamp(0.0, 1.0),
        };
        weight = weight * weight * (3.0 - 2.0 * weight);
        let mut last: HashMap<String, [f32; 9]> = HashMap::new();
        for (name, values) in parts.iter_mut() {
            let mut current = **values;
            let source = track
                .source
                .as_ref()
                .filter(|_| weight < 1.0)
                .and_then(|source| source.get(*name).copied());
            if let Some(source) = source {
                for i in 0..9 {
                    let mut delta = current[i] - source[i];
                    if (3..=5).contains(&i) {
                        delta = delta.sin().atan2(delta.cos());
                    }
                    current[i] = source[i] + delta * weight;
                }
            }
            last.insert((*name).to_string(), current);
            **values = current;
        }
        if weight >= 1.0 {
            track.source = None;
        }
        track.last = Some(last);
        track.external = external;
        track.time = time;
        track.enabled = enabled;
    }
}

/// `OuterPoseOwnershipBlend.Rigid`: the outer transform's translation and
/// rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RigidPose {
    position: [f32; 3],
    rotation: Quat,
}

/// One outer track's state.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OuterTrack {
    last: Option<RigidPose>,
    source: Option<RigidPose>,
    external: bool,
    time: f32,
    started: f32,
    duration: f32,
}

impl Default for OuterTrack {
    fn default() -> Self {
        Self {
            last: None,
            source: None,
            external: false,
            time: f32::NAN,
            started: 0.0,
            duration: 0.0,
        }
    }
}

/// `OuterPoseOwnershipBlend`: the whole-model matrix the mod draws with
/// hands over to vanilla (or takes back) through a smooth blend, keeping the
/// transform rigid.
#[derive(Debug, Clone, Default)]
pub struct OuterPoseOwnershipBlend {
    tracks: HashMap<u64, OuterTrack>,
}

impl OuterPoseOwnershipBlend {
    /// `apply`: `outer` is the mod's matrix (column-major), `render` the
    /// vanilla one this frame would draw with, both read as 16 floats.
    pub fn apply(
        &mut self,
        key: u64,
        outer: Option<&[f32; 16]>,
        render: &mut [f32; 16],
        external: bool,
        time: f32,
        enabled: bool,
    ) {
        let Some(outer) = outer else {
            return;
        };
        if !enabled || !time.is_finite() {
            self.tracks.remove(&key);
            return;
        }
        let outer_matrix = crate::camera::Mat4::from_column_major(outer);
        if outer_matrix.determinant().abs() < 1.0e-8 {
            self.tracks.remove(&key);
            return;
        }
        let render_matrix = crate::camera::Mat4::from_column_major(render);
        // The delta from the mod's matrix to the render's, made rigid.
        let delta = outer_matrix.invert().map(|inv| inv.mul(&render_matrix));
        let Some(delta) = delta else {
            self.tracks.remove(&key);
            return;
        };
        let scale = delta.scale_factors();
        if !delta.is_finite()
            || scale.iter().any(|s| (s - 1.0).abs() > 0.001)
            || (delta.determinant() - 1.0).abs() > 0.001
        {
            self.tracks.remove(&key);
            return;
        }
        let rigid = RigidPose {
            position: delta.translation(),
            rotation: delta.unnormalized_rotation().normalized(),
        };
        let track = self.tracks.entry(key).or_default();
        let continuous = track.time.is_finite()
            && time >= track.time
            && time - track.time <= 5.0
            && track.last.is_some();
        if !continuous {
            track.source = None;
        } else if external != track.external {
            track.source = track.last;
            track.started = time;
            track.duration = if external { 3.0 } else { 4.0 };
        }
        let mut weight = match track.source {
            None => 1.0,
            Some(_) => ((time - track.started) / track.duration).clamp(0.0, 1.0),
        };
        weight = weight * weight * (3.0 - 2.0 * weight);
        let blended = match track.source {
            Some(source) if weight < 1.0 => RigidPose {
                position: [
                    source.position[0] + (rigid.position[0] - source.position[0]) * weight,
                    source.position[1] + (rigid.position[1] - source.position[1]) * weight,
                    source.position[2] + (rigid.position[2] - source.position[2]) * weight,
                ],
                rotation: source.rotation.slerp(rigid.rotation, weight),
            },
            _ => rigid,
        };
        // render = outer * translate(blended.position) * rotate(blended.rotation)
        let applied = Mat4Builder::from_matrix(outer)
            .translate(
                blended.position[0],
                blended.position[1],
                blended.position[2],
            )
            .rotate(blended.rotation);
        *render = *applied.values();
        track.last = Some(blended);
        track.external = external;
        track.time = time;
        if weight >= 1.0 {
            track.source = None;
        }
    }
}

/// `ClimbFacing.yaw`: the yaw climbing a block faces, from its `facing` (or
/// its boolean direction flags, or the climbable neighbours), the position
/// across the block face, the player's yaw, and the climbable-direction test.
pub fn climb_facing_yaw(
    facing: Option<&str>,
    boolean_flags: &[(&str, bool)],
    climbable: impl Fn(&str) -> bool,
    across_x: f64,
    across_z: f64,
    yaw: f32,
) -> f32 {
    if let Some(facing) = facing
        && matches!(facing, "north" | "south" | "west" | "east")
    {
        return climb_opposite_yaw(facing);
    }
    let mut directions: Vec<&str> = Vec::new();
    for (flag, on) in boolean_flags {
        if *on && matches!(*flag, "north" | "south" | "west" | "east") {
            directions.push(flag);
        }
    }
    if directions.is_empty() {
        for direction in ["north", "south", "west", "east"] {
            if climbable(direction) {
                directions.push(direction);
            }
        }
    }
    if directions.is_empty() {
        return yaw;
    }
    let x = across_x.clamp(0.0, 1.0);
    let z = across_z.clamp(0.0, 1.0);
    let mut best = directions[0];
    for direction in &directions {
        if climb_distance(direction, x, z) < climb_distance(best, x, z) {
            best = direction;
        }
    }
    for direction in &directions {
        let yaw_of = climb_direction_yaw(direction);
        if wrap_degrees360(yaw_of - yaw).abs() < 0.01
            && climb_distance(direction, x, z) <= climb_distance(best, x, z) + 0.075
        {
            return yaw_of;
        }
    }
    climb_direction_yaw(best)
}

/// The yaw a direction faces, the Java `Direction.toYRot`.
fn climb_direction_yaw(direction: &str) -> f32 {
    match direction {
        "south" => 0.0,
        "west" => 90.0,
        "north" => 180.0,
        "east" => -90.0,
        _ => 0.0,
    }
}

/// `direction.getOpposite().toYRot()`.
fn climb_opposite_yaw(direction: &str) -> f32 {
    match direction {
        "north" => climb_direction_yaw("south"),
        "south" => climb_direction_yaw("north"),
        "west" => climb_direction_yaw("east"),
        "east" => climb_direction_yaw("west"),
        _ => 0.0,
    }
}

/// `ClimbFacing.distance`: how far across the block face the direction's
/// surface sits.
fn climb_distance(direction: &str, x: f64, z: f64) -> f64 {
    match direction {
        "north" => z,
        "south" => 1.0 - z,
        "west" => x,
        "east" => 1.0 - x,
        _ => f64::INFINITY,
    }
}

// ---------------------------------------------------------------------------
// ItemSequenceController
// ---------------------------------------------------------------------------

/// `ItemSequenceController.TICK_SECONDS` — every controller steps at 20 Hz.
pub const CONTROLLER_TICK_SECONDS: f32 = 0.05;
/// The default transition, the Java `0.16F`.
pub const SEQUENCE_TRANSITION_SECONDS: f32 = 0.16;
/// The default locomotion retain, the Java `0.15F`.
pub const SEQUENCE_RETAIN: f32 = 0.15;
/// The stop transition the interrupted sequences use.
pub const SEQUENCE_STOP_SECONDS: f32 = 0.08;

/// `ItemSequenceController.Input`: what one tick knows about the player's
/// hands. Items are registry ids (`None` when the hand is empty); the game
/// glue builds this.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemInput {
    pub main: Option<String>,
    pub off: Option<String>,
    pub using_stack: Option<String>,
    /// `true` when the used hand is the off hand.
    pub used_off_hand: bool,
    /// `true` when the main arm is the left one.
    pub main_arm_left: bool,
    pub slot: i32,
    pub tick: i32,
    pub using_ticks: i32,
    pub vertical_speed: f64,
    pub grounded: bool,
    pub using: bool,
    /// Whether this client is the player itself (not a remote one).
    pub local: bool,
    /// Whether the fishing bobber is out.
    pub hook_present: bool,
}

impl Default for ItemInput {
    fn default() -> Self {
        Self {
            main: None,
            off: None,
            using_stack: None,
            used_off_hand: false,
            main_arm_left: false,
            slot: 0,
            tick: 0,
            using_ticks: 0,
            vertical_speed: 0.0,
            grounded: true,
            using: false,
            local: true,
            hook_present: false,
        }
    }
}

impl ItemInput {
    /// `Input.item(hand)`.
    pub fn item(&self, off_hand: bool) -> Option<&String> {
        if off_hand {
            self.off.as_ref()
        } else {
            self.main.as_ref()
        }
    }
}

/// `ItemSequenceController`: the held item's state machine, the Java class
/// ported state for state. The library may be absent (no pack loaded); the
/// behavior then reads blank and the playback stops.
pub struct ItemSequenceController {
    playback: ClipSequencePlayer,
    owner: Option<String>,
    off_hand: bool,
    left: bool,
    initialized: bool,
    was_using: bool,
    hook_was_present: bool,
    airborne: bool,
    slam_started: bool,
    other_hand_occupied: bool,
    confirmed_action: bool,
    upper_body_only: bool,
    hands_only_use: bool,
    continuous_use: bool,
    riding: bool,
    behavior: String,
    phase: String,
    binding_type: String,
    attack_index: usize,
    last_attack_tick: i32,
    use_ticks: i32,
    owner_slot: i32,
    retain: f32,
    previous_retain: f32,
    target_retain: f32,
    legs_retain: f32,
    arms_only_influence: bool,
    movement_blend: f32,
    previous_movement_blend: f32,
    moving_legs_influence: f32,
    previous_legs_influence: f32,
    target_legs_influence: f32,
    requested_charge_time: f32,
    locomotion_influence: Option<LocomotionInfluence>,
    suppressed_arms: i32,
}

impl Default for ItemSequenceController {
    fn default() -> Self {
        Self::new()
    }
}

/// The library calls the state machine makes, bundled so the machine reads
/// the same either way.
struct SeqLibrary<'a> {
    library: Option<&'a crate::library::AnimationLibrary>,
}

impl<'a> SeqLibrary<'a> {
    fn new(library: Option<&'a crate::library::AnimationLibrary>) -> Self {
        Self { library }
    }

    fn behavior(&self, item: Option<&String>) -> String {
        match (self.library, item) {
            (Some(library), Some(item)) => library.bindings().string_action_setting(
                Some(item),
                None,
                "item_sequence",
                "behavior",
                "",
            ),
            _ => String::new(),
        }
    }

    fn numeric(&self, item: Option<&String>, action: &str, key: &str, default: f32) -> f32 {
        match (self.library, item) {
            (Some(library), Some(item)) => {
                library
                    .bindings()
                    .numeric_action_setting(Some(item), None, action, key, default)
            }
            _ => default,
        }
    }

    fn clip(&self, item: Option<&String>, action: &str, left: bool, other: bool) -> Option<Clip> {
        let library = self.library?;
        library.handed_clip_for(item.map(String::as_str), None, action, left, other)
    }

    fn clips(&self, item: Option<&String>, action: &str, left: bool, other: bool) -> Vec<Clip> {
        match self.library {
            Some(library) => {
                library.handed_clips_for(item.map(String::as_str), None, action, left, other)
            }
            None => Vec::new(),
        }
    }

    fn overlays(&self, item: Option<&String>, action: &str, left: bool) -> Vec<ResolvedOverlay> {
        match self.library {
            Some(library) => {
                library.overlays_resolved(item.map(String::as_str), None, action, left, false)
            }
            None => Vec::new(),
        }
    }

    fn retention(&self, item: Option<&String>, action: &str) -> BasePoseRetention {
        match self.library {
            Some(library) => {
                library.base_pose_retention_for(item.map(String::as_str), None, action)
            }
            None => BasePoseRetention::default(),
        }
    }

    fn influence(&self, item: Option<&String>, action: &str) -> Option<LocomotionInfluence> {
        self.library.and_then(|library| {
            library.configured_locomotion_influence(item.map(String::as_str), None, action)
        })
    }

    fn retrigger(&self, item: Option<&String>, action: &str, default: f32) -> f32 {
        match self.library {
            Some(library) => {
                library.retrigger_progress(item.map(String::as_str), None, action, default)
            }
            None => default,
        }
    }
}

/// The additive retention the shield reactions compose with, the Java
/// `ADDITIVE`.
#[allow(dead_code)]
fn additive_retention() -> BasePoseRetention {
    let mut retention = BasePoseRetention::default();
    for bone in [
        "root",
        "waist",
        "body",
        "head",
        "left_arm",
        "right_arm",
        "left_leg",
        "right_leg",
    ] {
        retention.set(bone, 1.0);
    }
    retention
}

impl ItemSequenceController {
    /// A controller at rest.
    pub fn new() -> Self {
        Self {
            playback: ClipSequencePlayer::new(),
            owner: None,
            off_hand: false,
            left: false,
            initialized: false,
            was_using: false,
            hook_was_present: false,
            airborne: false,
            slam_started: false,
            other_hand_occupied: false,
            confirmed_action: false,
            upper_body_only: false,
            hands_only_use: false,
            continuous_use: false,
            riding: false,
            behavior: String::new(),
            phase: String::new(),
            binding_type: String::new(),
            attack_index: 0,
            last_attack_tick: i32::MIN,
            use_ticks: 0,
            owner_slot: -1,
            retain: SEQUENCE_RETAIN,
            previous_retain: SEQUENCE_RETAIN,
            target_retain: SEQUENCE_RETAIN,
            legs_retain: f32::NAN,
            arms_only_influence: false,
            movement_blend: 0.0,
            previous_movement_blend: 0.0,
            moving_legs_influence: 1.0,
            previous_legs_influence: 1.0,
            target_legs_influence: 1.0,
            requested_charge_time: f32::NAN,
            locomotion_influence: None,
            suppressed_arms: 0,
        }
    }

    /// `behavior`.
    pub fn behavior(
        library: Option<&crate::library::AnimationLibrary>,
        item: Option<&String>,
    ) -> String {
        SeqLibrary::new(library).behavior(item)
    }

    /// `ownsMain`.
    pub fn owns_main(
        &self,
        library: Option<&crate::library::AnimationLibrary>,
        item: Option<&String>,
    ) -> bool {
        !Self::behavior(library, item).is_empty()
    }

    /// `active`.
    pub fn active(&self) -> bool {
        self.playback.active()
    }

    /// `hasClip`.
    pub fn has_clip(&self) -> bool {
        self.playback.clip().is_some()
    }

    /// `ownsDirectTrack`.
    pub fn owns_direct_track(&self, bone: &str) -> bool {
        self.playback.clip().is_some() && self.playback.has_direct_track(bone)
    }

    /// `hasLocomotionInfluence`.
    pub fn has_locomotion_influence(&self) -> bool {
        self.locomotion_influence.is_some()
    }

    /// `allowsStationarySteps`.
    pub fn allows_stationary_steps(&self) -> bool {
        !self.playback.active() || self.phase == "idle" || self.phase == "idle_one_handed"
    }

    /// `setHandsOnlyUse`.
    pub fn set_hands_only_use(&mut self, hands_only: bool) {
        self.hands_only_use = hands_only;
    }

    /// `setRiding`.
    pub fn set_riding(&mut self, riding: bool) {
        self.riding = riding;
    }

    /// `interruptRiding`.
    pub fn interrupt_riding(&mut self) {
        self.playback.stop(0.1);
        self.phase = String::new();
        self.was_using = false;
    }

    /// `clearRiding`.
    pub fn clear_riding(&mut self) {
        self.playback.clear();
        self.phase = String::new();
        self.was_using = false;
        self.hook_was_present = false;
        self.initialized = false;
        self.owner = None;
        self.owner_slot = -1;
    }

    /// `replacesFalling`.
    pub fn replaces_falling(&self) -> bool {
        let legs = self.playback.has_direct_track("root")
            || self.playback.has_direct_track("right_leg")
            || self.playback.has_direct_track("left_leg");
        Self::replaces_falling_for(
            &self.behavior,
            &self.phase,
            self.off_hand,
            self.playback.clip().is_some(),
        ) && legs
            && (self.phase != "slam" || self.airborne)
    }

    /// `replacesFalling(behavior, phase, hand, hasClip)`.
    pub fn replaces_falling_for(
        behavior: &str,
        phase: &str,
        off_hand: bool,
        has_clip: bool,
    ) -> bool {
        has_clip && !off_hand && behavior == "mace" && (phase == "charge" || phase == "slam")
    }

    /// `slamStarted`.
    pub fn slam_started(&self) -> bool {
        self.slam_started
    }

    /// `startsMaceCharge`.
    pub fn starts_mace_charge(
        grounded: bool,
        vertical_speed: f64,
        fall_height: f64,
        charge_height: f64,
        charging: bool,
    ) -> bool {
        !grounded && vertical_speed < 0.0 && (charging || fall_height > charge_height)
    }

    /// `canMaceSmash`.
    pub fn can_mace_smash(attacking: bool, distance: f64, threshold: f64) -> bool {
        !attacking && distance > threshold
    }

    /// `tick`, the Java signature with the arguments named after the
    /// LocomotionController call site.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        input: &ItemInput,
        library: Option<&crate::library::AnimationLibrary>,
        body_available: bool,
        attack_allowed: bool,
        grounded: bool,
        fall_distance: f64,
        use_item: bool,
        use_off_hand: bool,
        use_stack: Option<&String>,
        use_event: &str,
        slot_changed: bool,
        movement: f32,
        projected_landing: f64,
    ) {
        self.requested_charge_time = f32::NAN;
        let seq = SeqLibrary::new(library);
        self.update(
            &seq,
            input,
            body_available,
            attack_allowed,
            grounded,
            fall_distance,
            use_item,
            use_off_hand,
            use_stack,
            use_event,
            slot_changed,
            movement,
            projected_landing,
        );
        self.playback.tick(CONTROLLER_TICK_SECONDS);
        if self.phase == "charge" && self.requested_charge_time.is_finite() {
            self.playback.seek(self.requested_charge_time);
        }
    }

    /// `suspend`.
    pub fn suspend(&mut self) {
        self.playback.stop(0.18);
        self.playback.tick(CONTROLLER_TICK_SECONDS);
        self.phase = String::new();
        self.was_using = false;
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.playback.clear();
        self.phase = String::new();
        self.was_using = false;
        self.hook_was_present = false;
        self.initialized = false;
        self.owner = None;
        self.owner_slot = -1;
        self.confirmed_action = false;
        self.behavior = String::new();
        self.binding_type = String::new();
        self.attack_index = 0;
        self.retain = SEQUENCE_RETAIN;
        self.target_retain = SEQUENCE_RETAIN;
        self.legs_retain = f32::NAN;
        self.moving_legs_influence = 1.0;
        self.target_legs_influence = 1.0;
        self.locomotion_influence = None;
        self.suppressed_arms = 0;
    }

    /// `sample`.
    pub fn sample(&self, bone: &str, partial: f32) -> Sample {
        if self.riding
            && !(bone != "root"
                && !bone.ends_with("_leg")
                && (!bone.ends_with("_arm") || self.playback.has_direct_track(bone)))
        {
            return Sample::empty();
        }
        if self.hands_only_use && !bone.ends_with("_arm") && !bone.ends_with("_item") {
            return Sample::empty();
        }
        if !self.upper_body_only
            || (self.confirmed_action || self.continuous_use)
                && bone != "root"
                && bone != "left_leg"
                && bone != "right_leg"
        {
            let movement = lerp(partial, self.previous_movement_blend, self.movement_blend);
            let legs = lerp(
                partial,
                self.previous_legs_influence,
                self.moving_legs_influence,
            );
            let sampled = self.playback.sample(bone, partial);
            Self::with_moving_legs(sampled, bone, movement, legs)
        } else {
            Sample::empty()
        }
    }

    /// `withMovingLegs`.
    pub fn with_moving_legs(
        sample: Sample,
        bone: &str,
        movement: f32,
        legs_influence: f32,
    ) -> Sample {
        if bone != "left_leg" && bone != "right_leg" {
            return sample;
        }
        sample.with_weight(sample.weight * (1.0 - movement * (1.0 - legs_influence)))
    }

    /// `retain`.
    pub fn retain(&self, bone: &str, partial: f32) -> f32 {
        self.retain_for(bone, partial, false, false)
    }

    /// `retain(bone, partial, sprinting, jumping)`.
    pub fn retain_for(&self, bone: &str, partial: f32, sprinting: bool, jumping: bool) -> f32 {
        let arm = if bone == "left_arm" {
            1
        } else if bone == "right_arm" {
            2
        } else {
            0
        };
        if (self.suppressed_arms & arm) != 0 {
            return 0.0;
        }
        let hand_blocks = self.locomotion_influence.as_ref().is_some_and(|influence| {
            influence.hand_only && (if self.left { arm != 1 } else { arm != 2 })
        });
        if self.locomotion_influence.is_none()
            || (arm == 0 && self.arms_only_influence)
            || hand_blocks
        {
            self.legacy_retain(bone, partial)
        } else {
            self.locomotion_influence
                .as_ref()
                .map_or(0.0, |influence| influence.select(sprinting, jumping))
        }
    }

    fn legacy_retain(&self, bone: &str, partial: f32) -> f32 {
        let retain = lerp(partial, self.previous_retain, self.retain).clamp(0.0, 1.0);
        if bone != "left_leg" && bone != "right_leg" {
            return retain;
        }
        if self.legs_retain.is_finite() {
            return self.legs_retain.clamp(0.0, 1.0);
        }
        let movement = lerp(partial, self.previous_movement_blend, self.movement_blend);
        retain + (1.0 - retain) * movement
    }

    /// The `update` body, the Java method with its flags named.
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        seq: &SeqLibrary,
        input: &ItemInput,
        body_available: bool,
        attack_allowed: bool,
        _grounded: bool,
        fall_distance: f64,
        use_item: bool,
        use_off_hand: bool,
        use_stack: Option<&String>,
        use_event: &str,
        slot_changed: bool,
        movement: f32,
        projected_landing: f64,
    ) {
        self.slam_started = false;
        self.airborne = !input.grounded;
        self.upper_body_only = !body_available;
        self.previous_movement_blend = self.movement_blend;
        let mut blend = ((movement - 0.001) / 0.007).clamp(0.0, 1.0);
        blend = blend * blend * (3.0 - 2.0 * blend);
        self.movement_blend += (blend - self.movement_blend) * 0.4;
        self.previous_legs_influence = self.moving_legs_influence;
        self.moving_legs_influence +=
            (self.target_legs_influence - self.moving_legs_influence) * 0.4;
        self.previous_retain = self.retain;
        self.retain += (self.target_retain - self.retain) * 0.3;
        let hook_present = input.hook_present;
        let supports = use_item
            && Self::supports_use(
                &seq.behavior(Some(use_stack.unwrap_or(&String::new()))),
                use_event,
            );
        let same_slot = self.owner_slot == input.slot
            && !slot_changed
            && (self.owner == input.item(self.off_hand).cloned()
                || input.item(self.off_hand).is_none()
                || self.behavior == "bucket"
                    && seq.behavior(input.item(self.off_hand)) == "bucket");
        let using_behavior = seq.behavior(use_stack);
        self.continuous_use = self.hands_only_use
            && input.using
            && (using_behavior == "bow" || using_behavior == "trident");
        let keep_bow = self.hands_only_use
            && same_slot
            && self.was_using
            && (self.behavior == "bow" || self.behavior == "trident");
        if !body_available && !supports && !self.continuous_use && !keep_bow {
            if self.confirmed_action && same_slot && !self.playback.finished() {
                self.hook_was_present = hook_present;
            } else {
                self.playback.stop(SEQUENCE_STOP_SECONDS);
                self.phase = String::new();
                self.was_using = false;
                self.hook_was_present = hook_present;
            }
            return;
        }
        let other_behaved = !supports && input.using && !seq.behavior(use_stack).is_empty();
        let off_item = input.off.clone();
        let continues_off = self.off_hand
            && !slot_changed
            && !attack_allowed
            && self.owner_slot == input.slot
            && Self::continues_offhand_action(
                &self.phase,
                self.was_using,
                self.playback.finished(),
            )
            && (self.owner == off_item
                || off_item.is_none()
                || self.behavior == "bucket" && seq.behavior(off_item.as_ref()) == "bucket");
        let hand_off = Self::select_hand(
            other_behaved,
            input.used_off_hand,
            supports,
            use_off_hand,
            continues_off,
        );
        let was_occupied = self.other_hand_occupied;
        self.other_hand_occupied = input.item(!hand_off).is_some();
        let held = if other_behaved || supports {
            use_stack.cloned()
        } else if continues_off {
            self.owner.clone()
        } else {
            input.main.clone()
        };
        let behavior = seq.behavior(held.as_ref());
        let bucket_carry = self.behavior == "bucket"
            && behavior == "bucket"
            && self.off_hand == hand_off
            && self.owner_slot == input.slot
            && !slot_changed
            && !supports;
        let confirmed_keep = self.confirmed_action
            && !self.playback.finished()
            && input.item(self.off_hand).is_none()
            && self.owner_slot == input.slot
            && !slot_changed
            && !use_item;
        let changed = self.owner != held || self.off_hand != hand_off || self.behavior != behavior;
        if changed && bucket_carry {
            self.owner = held;
            if !self.phase.starts_with("use") || self.playback.finished() {
                self.enter(seq, "idle", true, f32::INFINITY, None);
            }
        } else if changed && !confirmed_keep {
            let first = !self.initialized;
            self.initialized = true;
            self.owner = held;
            self.off_hand = hand_off;
            self.left = Self::visual_left(input, self.off_hand);
            self.behavior = behavior;
            self.owner_slot = input.slot;
            self.was_using = false;
            self.attack_index = 0;
            self.phase = String::new();
            if self.behavior.is_empty() {
                self.playback.stop(0.18);
            } else if !first && !supports {
                let phase = self.equip_phase(seq, input);
                self.enter(seq, &phase, false, f32::INFINITY, None);
            }
        } else if slot_changed && !self.behavior.is_empty() && !supports {
            self.left = Self::visual_left(input, self.off_hand);
            let phase = self.equip_phase(seq, input);
            self.enter(seq, &phase, false, f32::INFINITY, None);
        }
        if self.behavior.is_empty() {
            self.hook_was_present = hook_present;
            return;
        }
        if self.left != Self::visual_left(input, self.off_hand) {
            self.left = Self::visual_left(input, self.off_hand);
            let phase = if self.phase.is_empty() {
                "idle".to_string()
            } else {
                self.phase.clone()
            };
            let looped = Self::is_loop_phase(&phase);
            self.enter(seq, &phase, looped, f32::INFINITY, None);
        }
        if was_occupied != self.other_hand_occupied {
            self.refresh_held_hand_variant(seq);
        }
        let using_this_hand = !supports
            && input.using
            && input.used_off_hand == self.off_hand
            && self.owner == input.using_stack;
        if self.behavior == "bow" || self.behavior == "trident" {
            if using_this_hand {
                self.use_ticks = input.using_ticks;
                if !self.was_using {
                    self.enter(seq, "charge", false, f32::INFINITY, None);
                }
                if self.phase == "charge" {
                    let charge_ticks =
                        seq.numeric(self.owner.as_ref(), "item_sequence", "charge_ticks", 20.0);
                    if let Some(clip) = self.playback.clip() {
                        self.requested_charge_time =
                            clip.length * (self.use_ticks as f32 / charge_ticks.max(1.0)).min(1.0);
                        if self.requested_charge_time >= clip.length
                            && self.clip_for(seq, "hold").is_some()
                        {
                            self.enter(seq, "hold", true, f32::INFINITY, None);
                        }
                    }
                }
            } else if self.was_using {
                let remote = !input.local;
                let release = use_item && use_event == "release"
                    || remote
                        && self.use_ticks as f32
                            >= seq.numeric(
                                self.owner.as_ref(),
                                "item_sequence",
                                "minimum_charge_ticks",
                                3.0,
                            );
                if release {
                    self.enter(seq, "release", false, f32::INFINITY, None);
                    self.confirmed_action = true;
                } else {
                    self.enter(seq, "idle", true, f32::INFINITY, None);
                }
            } else if use_item && use_event == "release" {
                self.enter(seq, "release", false, f32::INFINITY, None);
                self.confirmed_action = true;
            }
            self.was_using = using_this_hand;
        }
        if body_available && attack_allowed && !supports && !self.off_hand && !using_this_hand {
            self.attack(
                seq,
                attack_allowed,
                fall_distance,
                input.tick,
                projected_landing,
            );
        }
        if body_available
            && !self.off_hand
            && self.behavior == "mace"
            && self.phase != "slam"
            && !self.phase.starts_with("attack")
        {
            let charging = self.phase == "charge";
            let start = Self::starts_mace_charge(
                input.grounded,
                input.vertical_speed,
                projected_landing,
                seq.numeric(self.owner.as_ref(), "item_sequence", "charge_height", 3.0) as f64,
                charging,
            );
            if start && !charging {
                self.enter(seq, "charge", false, f32::INFINITY, None);
            } else if !start && charging {
                self.enter(seq, "idle", true, f32::INFINITY, None);
            }
        }
        let use_this = supports && use_off_hand == self.off_hand;
        if use_this {
            self.use_action(seq, Self::one_handed_for(input, self.off_hand), use_event);
        }
        if self.behavior == "fishing" && !use_this {
            if hook_present && !self.hook_was_present && self.phase != "cast" {
                self.enter(seq, "cast", false, f32::INFINITY, None);
            }
            if !hook_present && self.hook_was_present && self.phase != "reel" {
                self.enter(seq, "reel", false, f32::INFINITY, None);
            }
        }
        if self.playback.finished()
            && !using_this_hand
            && self.phase != "charge"
            && self.phase != "idle_one_handed"
        {
            if self.behavior == "fishing" && hook_present {
                if self.phase != "fishing" {
                    self.enter(seq, "fishing", true, f32::INFINITY, None);
                }
            } else if self.phase != "idle" {
                self.enter(seq, "idle", true, f32::INFINITY, None);
            }
        }
        if self.phase.is_empty() {
            let phase = if self.behavior == "fishing" && hook_present {
                "fishing"
            } else {
                "idle"
            };
            self.enter(seq, phase, true, f32::INFINITY, None);
        }
        if self.phase == "idle" || self.phase == "idle_one_handed" {
            let wanted = if Self::one_handed_for(input, self.off_hand)
                && self.clip_for(seq, "idle_one_handed").is_some()
            {
                "idle_one_handed"
            } else {
                "idle"
            };
            if self.phase != wanted {
                self.enter(seq, wanted, true, f32::INFINITY, None);
            }
        }
        self.hook_was_present = hook_present;
    }

    /// `attack`.
    fn attack(
        &mut self,
        seq: &SeqLibrary,
        attacking: bool,
        distance: f64,
        tick: i32,
        _projected_landing: f64,
    ) {
        if self.behavior == "mace"
            && Self::can_mace_smash(
                attacking,
                distance,
                seq.numeric(self.owner.as_ref(), "item_sequence", "smash_distance", 1.5) as f64,
            )
        {
            self.enter(seq, "slam", false, f32::INFINITY, None);
            self.slam_started = true;
        } else if self.ready(seq) {
            if self.behavior != "flint" && self.behavior != "shears" {
                let mut list = seq.clips(
                    self.owner.as_ref(),
                    "sequence_attack",
                    self.left,
                    self.other_hand_occupied,
                );
                if list.is_empty() && self.behavior == "throwable" {
                    self.enter(seq, "use", false, f32::INFINITY, None);
                } else {
                    let plain = list.is_empty();
                    if plain {
                        list = seq.clips(
                            self.owner.as_ref(),
                            "attack",
                            self.left,
                            self.other_hand_occupied,
                        );
                    }
                    if !list.is_empty() {
                        if tick - self.last_attack_tick > 30 {
                            self.attack_index = 0;
                        }
                        let index = self.attack_index % list.len();
                        self.attack_index += 1;
                        let clip = list[index].clone();
                        let name = format!("attack_{}", index + 1);
                        self.enter_with(
                            seq,
                            &name,
                            Some(clip),
                            false,
                            f32::INFINITY,
                            if plain { Some("attack") } else { None },
                        );
                        self.last_attack_tick = tick;
                    }
                }
            } else {
                self.enter(seq, "use", false, f32::INFINITY, None);
            }
        }
    }

    /// `refreshHeldHandVariant`.
    fn refresh_held_hand_variant(&mut self, seq: &SeqLibrary) {
        if self.phase == "charge" || self.phase == "hold" || self.phase == "fishing" {
            let clip = self.clip_for(seq, &self.phase);
            if let Some(clip) = clip {
                let differs = self
                    .playback
                    .clip()
                    .is_none_or(|playing| playing.name != clip.name);
                if differs {
                    let progress = self.playback.progress();
                    let confirmed = self.confirmed_action;
                    let looped = Self::is_loop_phase(&self.phase);
                    let phase = self.phase.clone();
                    self.enter(seq, &phase, looped, f32::INFINITY, None);
                    if self.phase != "hold" {
                        self.playback.seek(progress * clip.length);
                    }
                    self.confirmed_action = confirmed;
                }
            }
        }
    }

    /// `use`.
    fn use_action(&mut self, seq: &SeqLibrary, one_handed: bool, event: &str) {
        if self.behavior == "fishing" {
            let reel = event == "reel" || self.hook_was_present && event != "cast";
            let phase = if reel { "reel" } else { "cast" };
            self.enter_accepted_use(seq, phase);
        } else if self.behavior == "flint"
            || self.behavior == "shears"
            || self.behavior == "bucket"
            || self.behavior == "throwable"
        {
            let phase = if one_handed && self.clip_for(seq, "use_one_handed").is_some() {
                "use_one_handed"
            } else {
                "use"
            };
            self.enter_accepted_use(seq, phase);
        }
    }

    /// `ready`.
    fn ready(&self, seq: &SeqLibrary) -> bool {
        if self.phase != "idle"
            && self.phase != "idle_one_handed"
            && self.phase != "fishing"
            && self.phase != "charge"
            && !self.phase.starts_with("equip")
        {
            let percent = seq.retrigger(self.owner.as_ref(), "item_sequence", 70.0) * 100.0;
            self.playback.progress()
                >= seq.retrigger(self.owner.as_ref(), &self.binding_type, percent)
        } else {
            true
        }
    }

    /// `selectHand`.
    pub fn select_hand(
        other_behaved: bool,
        used_off_hand: bool,
        supports: bool,
        use_off_hand: bool,
        continues_off: bool,
    ) -> bool {
        if other_behaved {
            used_off_hand
        } else if supports {
            use_off_hand
        } else {
            continues_off
        }
    }

    /// `supportsUse`.
    pub fn supports_use(behavior: &str, event: &str) -> bool {
        match behavior {
            "bow" | "trident" => event == "release",
            "fishing" | "flint" | "shears" | "bucket" | "throwable" => true,
            _ => false,
        }
    }

    /// `continuesOffhandAction`.
    pub fn continues_offhand_action(phase: &str, was_using: bool, finished: bool) -> bool {
        if !was_using || phase != "charge" && phase != "hold" {
            !finished
                && (phase == "release"
                    || phase.starts_with("use")
                    || phase == "cast"
                    || phase == "reel"
                    || phase == "fishing")
        } else {
            true
        }
    }

    /// `equipPhase`.
    fn equip_phase(&self, seq: &SeqLibrary, input: &ItemInput) -> String {
        if Self::one_handed_for(input, self.off_hand)
            && self.clip_for(seq, "equip_one_handed").is_some()
        {
            "equip_one_handed".to_string()
        } else {
            "equip".to_string()
        }
    }

    /// `oneHanded`: the other hand carries something.
    fn one_handed_for(input: &ItemInput, off_hand: bool) -> bool {
        input.item(!off_hand).is_some()
    }

    /// `visualLeft`.
    fn visual_left(input: &ItemInput, off_hand: bool) -> bool {
        // The main arm's side, flipped when the item sits in the off hand.
        if off_hand {
            !input.main_arm_left
        } else {
            input.main_arm_left
        }
    }

    /// `clip(action)`: the sequence-prefixed clip for the phase.
    fn clip_for(&self, seq: &SeqLibrary, action: &str) -> Option<Clip> {
        seq.clip(
            self.owner.as_ref(),
            &format!("sequence_{}", action),
            self.left,
            self.other_hand_occupied,
        )
    }

    /// `enter(phase, looped)`.
    fn enter(
        &mut self,
        seq: &SeqLibrary,
        phase: &str,
        looped: bool,
        transition: f32,
        named: Option<&str>,
    ) {
        let clip = self.clip_for(seq, phase);
        self.enter_with(seq, phase, clip, looped, transition, named);
    }

    /// `overIdle`.
    fn over_idle(&self, seq: &SeqLibrary, phase: &str) -> Option<Clip> {
        if !phase.starts_with("idle")
            && !phase.starts_with("equip")
            && seq.numeric(self.owner.as_ref(), &self.binding_type, "over_idle", 0.0) > 0.5
        {
            self.clip_for(seq, "idle")
        } else {
            None
        }
    }

    /// `enterAcceptedUse`.
    fn enter_accepted_use(&mut self, seq: &SeqLibrary, phase: &str) {
        self.enter(seq, phase, false, 0.08, None);
        self.confirmed_action = true;
    }

    /// `enter(phase, clip, looped, transition, named)`.
    fn enter_with(
        &mut self,
        seq: &SeqLibrary,
        phase: &str,
        clip: Option<Clip>,
        looped: bool,
        transition: f32,
        named: Option<&str>,
    ) {
        self.confirmed_action = false;
        if self.riding
            && !(phase.starts_with("idle") || phase.starts_with("equip") || phase == "fishing")
        {
            self.phase = phase.to_string();
            self.playback.stop(0.28);
            return;
        }
        let mut phase = phase.to_string();
        let mut clip = clip;
        let renamed = if named.is_none() {
            self.one_handed_phase(seq, &phase, self.other_hand_occupied)
        } else {
            phase.clone()
        };
        if renamed != phase {
            phase = renamed;
            clip = self.clip_for(seq, &phase);
        }
        self.phase = phase.clone();
        self.binding_type = named.map_or_else(
            || format!("sequence_{}", self.settings_phase(seq, &phase)),
            str::to_string,
        );
        self.arms_only_influence = named.is_some();
        self.target_retain = self.action_setting(seq, "locomotion_retain", SEQUENCE_RETAIN);
        self.locomotion_influence = seq.influence(self.owner.as_ref(), &self.binding_type);
        if self.locomotion_influence.is_none() && self.binding_type.ends_with("_one_handed") {
            self.locomotion_influence = seq.influence(
                self.owner.as_ref(),
                &self.binding_type[..self.binding_type.len() - "_one_handed".len()],
            );
        }
        if self.locomotion_influence.is_none() {
            self.locomotion_influence = seq.influence(self.owner.as_ref(), "item_sequence");
        }
        self.suppressed_arms = self.action_setting(seq, "disable_locomotion_arm_swing", 0.0) as i32;
        self.legs_retain = self.action_setting(seq, "legs_locomotion_retain", f32::NAN);
        self.target_legs_influence = self
            .action_setting(seq, "moving_legs_influence", 1.0)
            .clamp(0.0, 1.0);
        let retention = self
            .retention_of(seq, &self.binding_type)
            .over(&self.retention_of(seq, "item_sequence"));
        let transition = transition.min(self.action_setting(
            seq,
            "transition_seconds",
            SEQUENCE_TRANSITION_SECONDS,
        ));
        let over_idle = self.over_idle(seq, &phase);
        self.playback.play(
            clip.clone(),
            seq.overlays(self.owner.as_ref(), &self.binding_type, self.left),
            looped && clip.as_ref().is_some_and(|clip| clip.loop_),
            transition,
            phase == "hold",
            retention,
            self.left,
            over_idle,
        );
        if self.action_setting(seq, "start_at_end", 0.0) > 0.5 {
            self.playback.hold_last_frame();
        }
    }

    /// `oneHandedPhase`.
    fn one_handed_phase(&self, seq: &SeqLibrary, phase: &str, other_occupied: bool) -> String {
        if !other_occupied || phase.ends_with("_one_handed") {
            return phase.to_string();
        }
        if phase != "use" && phase != "equip" && phase != "idle" && !phase.starts_with("attack") {
            return phase.to_string();
        }
        let named = format!("{}_one_handed", phase);
        if self.clip_for(seq, &named).is_none() {
            phase.to_string()
        } else {
            named
        }
    }

    /// `actionSetting`.
    fn action_setting(&self, seq: &SeqLibrary, key: &str, default: f32) -> f32 {
        let mut value = seq.numeric(self.owner.as_ref(), "item_sequence", key, default);
        if self.binding_type.ends_with("_one_handed") {
            let base = &self.binding_type[..self.binding_type.len() - "_one_handed".len()];
            value = seq.numeric(self.owner.as_ref(), base, key, value);
        }
        seq.numeric(self.owner.as_ref(), &self.binding_type, key, value)
    }

    /// `basePoseRetention(stack, action)`.
    fn retention_of(&self, seq: &SeqLibrary, action: &str) -> BasePoseRetention {
        seq.retention(self.owner.as_ref(), action)
    }

    /// `settingsPhase`.
    fn settings_phase(&self, seq: &SeqLibrary, phase: &str) -> String {
        if self.other_hand_occupied && !phase.ends_with("_one_handed") {
            let named = format!("{}_one_handed", phase);
            if self.clip_for(seq, &named).is_none() {
                phase.to_string()
            } else {
                named
            }
        } else {
            phase.to_string()
        }
    }

    /// `isLoopPhase`.
    pub fn is_loop_phase(phase: &str) -> bool {
        phase == "idle" || phase == "idle_one_handed" || phase == "hold" || phase == "fishing"
    }
}

/// `ItemUseController.mirrorFallback`.
pub fn mirror_fallback(clip: Option<&Clip>, left: bool) -> bool {
    clip.is_some_and(|clip| left && !clip.name.ends_with("_left"))
}

/// `ItemUseController.sampledBone`.
pub fn sampled_bone(bone: &str, mirror: bool) -> String {
    if !mirror {
        return bone.to_string();
    }
    if let Some(rest) = bone.strip_prefix("left_") {
        format!("right_{}", rest)
    } else if let Some(rest) = bone.strip_prefix("right_") {
        format!("left_{}", rest)
    } else {
        bone.to_string()
    }
}

/// `ItemUseController.sample`: the playback's sample, mirrored when the hand
/// asks for it.
pub fn item_use_sample(
    playback: &ClipSequencePlayer,
    bone: &str,
    partial: f32,
    mirror: bool,
) -> Sample {
    let sample = playback.sample(&sampled_bone(bone, mirror), partial);
    if mirror && sample.weight > 0.0 {
        Sample {
            pose: Pose {
                position: Vec3::new(
                    -sample.pose.position.x,
                    sample.pose.position.y,
                    sample.pose.position.z,
                ),
                rotation: Vec3::new(
                    sample.pose.rotation.x,
                    -sample.pose.rotation.y,
                    -sample.pose.rotation.z,
                ),
                scale: sample.pose.scale,
            },
            ..sample
        }
    } else {
        sample
    }
}

// ---------------------------------------------------------------------------
// CrouchSequenceController
// ---------------------------------------------------------------------------

/// `CrouchSequenceController.BLEND_SECONDS`.
pub const CROUCH_BLEND_SECONDS: f32 = 0.1;
/// The crouch amount's per-tick limits, the Java `-0.2F`/`0.4F`.
pub const CROUCH_AMOUNT_OUT: f32 = 0.2;
pub const CROUCH_AMOUNT_IN: f32 = 0.4;
/// The stop transition, the Java `0.125F`.
pub const CROUCH_STOP_SECONDS: f32 = 0.125;

/// `CrouchSequenceController`: the crouch's enter/hold/exit playback.
pub struct CrouchSequenceController {
    playback: ClipSequencePlayer,
    initialized: bool,
    crouching: bool,
    phase: String,
    amount: f32,
    previous_amount: f32,
    single_pose: bool,
}

impl Default for CrouchSequenceController {
    fn default() -> Self {
        Self::new()
    }
}

impl CrouchSequenceController {
    /// A controller at rest.
    pub fn new() -> Self {
        Self {
            playback: ClipSequencePlayer::new(),
            initialized: false,
            crouching: false,
            phase: String::new(),
            amount: 0.0,
            previous_amount: 0.0,
            single_pose: false,
        }
    }

    /// `tick`, the full form: `crouching`, `sprinting`, the enter/hold/exit
    /// clips and the playback rate. The eight arguments mirror the Java
    /// signature.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        crouching: bool,
        sprinting: bool,
        enter: Option<Clip>,
        hold: Option<Clip>,
        exit: Option<Clip>,
        rate: f32,
        single_pose: bool,
    ) {
        if self.single_pose != single_pose {
            self.cancel();
        }
        self.single_pose = single_pose;
        if single_pose {
            self.tick_single_pose(crouching, enter.or(hold), rate);
            return;
        }
        if !crouching {
            self.cancel();
            return;
        }
        self.previous_amount = self.amount;
        let target = if !sprinting || (enter.is_none() && hold.is_none()) {
            0.0
        } else {
            1.0
        };
        self.amount += (target - self.amount).clamp(-CROUCH_AMOUNT_OUT, CROUCH_AMOUNT_IN);
        if !self.initialized {
            self.initialized = true;
            self.crouching = crouching;
            if crouching {
                self.hold(hold, enter);
            }
        } else if crouching != self.crouching {
            self.crouching = crouching;
            if crouching {
                if enter.is_some() && enter != hold {
                    self.play("enter", enter, false);
                } else {
                    self.hold(hold, enter);
                }
            } else if let Some(exit) = exit {
                self.play("exit", Some(exit), false);
            } else {
                self.stop();
            }
        } else if crouching && self.phase == "enter" && self.playback.finished() {
            self.hold(hold, enter);
        } else if !crouching && self.phase == "exit" && self.playback.finished() {
            self.stop();
        } else if crouching
            && self.phase == "hold"
            && hold.is_some()
            && self.playback.clip() != hold.as_ref()
        {
            self.hold(hold, enter);
        }
        self.playback.set_playback_rate(rate);
        self.playback.tick(CONTROLLER_TICK_SECONDS);
    }

    fn tick_single_pose(&mut self, crouching: bool, clip: Option<Clip>, rate: f32) {
        if !crouching {
            self.cancel();
            return;
        }
        self.previous_amount = self.amount;
        let target = if crouching && clip.is_some() {
            1.0
        } else {
            0.0
        };
        self.amount += (target - self.amount).clamp(-CROUCH_AMOUNT_OUT, CROUCH_AMOUNT_IN);
        if crouching && clip.is_some() {
            if !self.crouching || self.playback.clip() != clip.as_ref() {
                self.play("enter", clip.clone(), false);
            }
            if self.playback.finished() {
                self.phase = "hold".to_string();
                self.playback.hold_last_frame();
            }
        } else if self.crouching || self.playback.clip().is_some() {
            self.stop();
        }
        self.initialized = true;
        self.crouching = crouching;
        self.playback
            .set_playback_rate(if self.phase == "hold" { 0.0 } else { rate });
        self.playback.tick(CONTROLLER_TICK_SECONDS);
    }

    /// Whether the single-pose mode is on.
    pub fn single_pose(&self) -> bool {
        self.single_pose
    }

    fn hold(&mut self, hold: Option<Clip>, enter: Option<Clip>) {
        let clip = hold.or(enter);
        let Some(clip) = clip else {
            self.stop();
            return;
        };
        if self.playback.clip() != Some(&clip) || self.phase != "hold" {
            let looped = clip.loop_;
            self.play("hold", Some(clip), looped);
        }
        if !self.playback.clip().is_some_and(|clip| clip.loop_) {
            self.playback.hold_last_frame();
        }
    }

    fn play(&mut self, phase: &str, clip: Option<Clip>, looped: bool) {
        self.phase = phase.to_string();
        self.playback.play(
            clip,
            Vec::new(),
            looped,
            CROUCH_BLEND_SECONDS,
            false,
            BasePoseRetention::default(),
            false,
            None,
        );
    }

    fn stop(&mut self) {
        self.phase = String::new();
        self.playback.stop(CROUCH_STOP_SECONDS);
    }

    /// `sample`.
    pub fn sample(&self, bone: &str, partial: f32) -> Sample {
        self.playback.sample(bone, partial)
    }

    /// `isActive`.
    pub fn is_active(&self) -> bool {
        self.playback.active()
    }

    /// The clip playing.
    pub fn clip(&self) -> Option<&Clip> {
        self.playback.clip()
    }

    /// The phase name.
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// `renderTime`.
    pub fn render_time(&self, partial: f32) -> f32 {
        self.playback.render_time(partial)
    }

    /// `renderAmount`.
    pub fn render_amount(&self, partial: f32) -> f32 {
        let amount =
            (self.previous_amount + (self.amount - self.previous_amount) * partial).clamp(0.0, 1.0);
        amount * amount * (3.0 - 2.0 * amount)
    }

    /// `cancel`.
    pub fn cancel(&mut self) {
        self.playback.clear();
        self.initialized = false;
        self.crouching = false;
        self.single_pose = false;
        self.phase = String::new();
        self.previous_amount = 0.0;
        self.amount = 0.0;
    }
}

/// The glue's crouch state: the controller plus the vanilla-shaped amount
/// the pose builder reads when no pack carries crouch clips.
#[derive(Default)]
pub struct CrouchSequenceState {
    pub controller: CrouchSequenceController,
}

impl CrouchSequenceState {
    fn new() -> Self {
        Self {
            controller: CrouchSequenceController::new(),
        }
    }
}

/// The glue's item state: the controller plus whether an emote plays.
#[derive(Default)]
pub struct ItemSequenceState {
    pub controller: ItemSequenceController,
    emote: Option<(ItemAction, f32)>,
}

impl ItemSequenceState {
    fn new() -> Self {
        Self {
            controller: ItemSequenceController::new(),
            emote: None,
        }
    }

    /// The emote hand action our glue plays, with its seconds left.
    pub fn play_emote(&mut self, action: ItemAction) {
        self.emote = Some((action, 0.6));
    }

    /// Steps the emote action down; `Some(action)` while it plays.
    pub fn tick_emote(&mut self, dt: f32) -> Option<ItemAction> {
        let Some((action, seconds)) = &mut self.emote else {
            return None;
        };
        *seconds -= dt;
        if *seconds <= 0.0 {
            self.emote = None;
            None
        } else {
            Some(*action)
        }
    }

    /// The action playing, if any.
    pub fn emote(&self) -> Option<ItemAction> {
        self.emote.map(|(action, _)| action)
    }
}

/// The gait phase wrapped into `0.0..1.0` (`x - floor(x)`).
fn gait_wrap(phase: f32) -> f32 {
    phase - phase.floor()
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

// ---------------------------------------------------------------------------
// PlayerAnimationEngine
// ---------------------------------------------------------------------------

/// The whole-model transform the body hook multiplies in: how the player
/// leans, banks, bobs, squashes and rolls.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RootMotion {
    /// Forward lean, radians about x (positive leans forward).
    pub lean: f32,
    /// Sideways bank, radians about z (positive leans right).
    pub bank: f32,
    /// Vertical bob, blocks.
    pub bob: f32,
    /// Vertical squash, `0.0..=1.0`.
    pub squash: f32,
    /// Roll about y, radians (gliding and swimming).
    pub roll: f32,
}

/// One player's animation state: the signals, the controllers, and the pose
/// the last two updates built.
pub struct PlayerState {
    /// The signals of the last update.
    pub signals: MotionSignals,
    pub locomotion: LocomotionController,
    /// The gait of the last update, kept so a read costs no step.
    pub gait: LocomotionFrame,
    /// The gait before it, for a partial-tick read.
    previous_gait: LocomotionFrame,
    pub turn: TurnInPlaceController,
    /// The item sequence state machine (the Java `ItemSequenceController`).
    pub items: crate::engine::ItemSequenceState,
    /// The boat's paddles, while rowing.
    pub boat: BoatAnimation,
    /// The fishing line's bob.
    pub fishing: FishingSwing,
    /// The direction-change lean.
    pub direction: DirectionChangeController,
    /// The sideways lean from turning.
    pub turn_lean: TurnLeanController,
    /// The creative-flight hover detector.
    pub flight: CreativeFlightMotion,
    /// The swimming surface alignment.
    pub swim: SwimSurfaceAlignment,
    /// The crouch sequence state machine.
    pub crouch: CrouchSequenceState,
    /// The glide height alignment.
    pub elytra: ElytraFlightAlignment,
    /// The pose ownership blends.
    pub ownership: PoseOwnershipBlend,
    pub outer_ownership: OuterPoseOwnershipBlend,
    /// The player's face: blinks, eyes and cues.
    pub face: FaceRig,
    /// Whether the player is rowing a boat.
    pub riding: bool,
    /// Whether a fishing line is out.
    pub fishing_line: bool,
    /// The emote hand action our glue plays (the Java emote system plays a
    /// named clip; this is the game-side stand-in).
    pub emote_action: Option<(ItemAction, f32)>,
    /// The pose input the last `begin_pose` carried (the yaw and pitch the
    /// turn controllers read).
    pub pose_input: PoseInput,
    /// The animated pose the last update built.
    pose: HumanoidPose,
    /// The pose before it, for a partial-tick blend.
    previous: HumanoidPose,
    /// Seconds of the player's animation clock.
    clock: f32,
    /// How much of the animation shows, `0.0..=1.0`.
    blend: f32,
    /// The bones whose base pose this player keeps.
    pub retention: BasePoseRetention,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            signals: MotionSignals::new(),
            locomotion: LocomotionController::new(),
            gait: LocomotionFrame::default(),
            previous_gait: LocomotionFrame::default(),
            turn: TurnInPlaceController::new(),
            items: ItemSequenceState::new(),
            boat: BoatAnimation::new(0, 0, [false, false], [0.0, 0.0], false),
            fishing: FishingSwing::default(),
            direction: DirectionChangeController::default(),
            turn_lean: TurnLeanController::default(),
            flight: CreativeFlightMotion::default(),
            swim: SwimSurfaceAlignment::default(),
            crouch: CrouchSequenceState::new(),
            elytra: ElytraFlightAlignment::default(),
            ownership: PoseOwnershipBlend::default(),
            outer_ownership: OuterPoseOwnershipBlend::default(),
            face: FaceRig::default(),
            riding: false,
            fishing_line: false,
            emote_action: None,
            pose_input: PoseInput::default(),
            pose: HumanoidPose::identity(),
            previous: HumanoidPose::identity(),
            clock: 0.0,
            blend: 0.0,
            retention: BasePoseRetention::default(),
        }
    }
}

impl PlayerState {
    /// Seconds of animation so far.
    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// How much of the animation shows, `0.0..=1.0`.
    pub fn blend(&self) -> f32 {
        self.blend
    }

    /// The pose the last update built.
    pub fn pose(&self) -> &HumanoidPose {
        &self.pose
    }

    /// One update: advance every controller and rebuild the pose. The
    /// item-sequence machine only runs when the glue hands it a library and
    /// input (see [`PlayerAnimationEngine::tick_items`]); without one it
    /// stays idle, like a player with an empty hand.
    fn advance(&mut self, signals: MotionSignals, dt: f32, tuning: &EngineTuning) {
        self.previous = self.pose.clone();
        self.signals = signals;
        self.clock += dt;
        self.face
            .update(&signals, &crate::faces::current_settings(), dt);
        let loco = self.locomotion.update(&signals, dt, tuning);
        self.previous_gait = self.gait;
        self.gait = loco;
        // Turn in place, the Java update, driven from the pose input's yaw.
        let input = self.pose_input;
        let settings = tuning.turn_settings.clone();
        self.turn.update(
            input.yaw,
            input.yaw,
            input.pitch,
            self.clock as f64,
            !signals.elytra && !signals.climbing && !signals.crawling,
            !signals.moving,
            false,
            &settings,
        );
        // The crouch machine: no clips here, so it tracks the amount only.
        self.crouch.controller.tick(
            signals.crouching,
            signals.running,
            None,
            None,
            None,
            1.0,
            false,
        );
        // The direction-change lean and the turn lean, per tick.
        self.direction.tick(signals.tick, 0.0, 0.0, input.yaw, true);
        self.turn_lean.tick(
            signals.tick,
            input.yaw,
            signals.walk_forward,
            0.0,
            signals.walk_forward,
            true,
            signals.jumped != 0,
            false,
        );
        // The creative-flight hover.
        self.flight
            .update(signals.tick, 0.0, signals.creative_flight);
        // The swim alignment.
        self.swim.tick(
            signals.surface_swimming || signals.submerged_swimming,
            signals.swim_vertical > 0,
            f64::NAN,
            SWIM_DEFAULT_DEPTH,
        );
        // The boat's paddles, while rowing.
        if self.riding {
            let paddling = signals.moving;
            self.boat
                .step(signals.tick, 0, false, [paddling, paddling], false);
        }
        // The emote hand action runs down.
        let _ = self.items.tick_emote(dt);
        self.blend = approach(self.blend, 1.0, 8.0 * dt);
        let animated = self.build_pose(&loco, tuning);
        self.pose = HumanoidPose {
            blend: self.blend,
            retention: self.retention.clone(),
            ..animated
        }
        .over(&HumanoidPose::identity());
    }

    /// The whole-model transform for this player at `partial`.
    pub fn root_motion(&self, partial: f32) -> RootMotion {
        let partial = if partial.is_finite() {
            partial.clamp(0.0, 1.0)
        } else {
            1.0
        };
        // Read between the last two updates, so a partial tick moves the
        // model smoothly instead of in whole ticks.
        let mix = |a: f32, b: f32| a + (b - a) * partial;
        let loco = LocomotionFrame {
            phase: mix(self.previous_gait.phase, self.gait.phase),
            stride: mix(self.previous_gait.stride, self.gait.stride),
            lift: mix(self.previous_gait.lift, self.gait.lift),
            speed: mix(self.previous_gait.speed, self.gait.speed),
            run_seconds: self.gait.run_seconds,
        };
        let phase = loco.phase.to_radians();
        let stride = loco.stride;
        let glide = if self.signals.elytra { 1.0 } else { 0.0 };
        RootMotion {
            lean: stride * 0.12 + glide * 0.45,
            bank: self.signals.elytra_bank * glide,
            bob: phase.cos() * 0.03 * stride - loco.lift * 0.05,
            squash: loco.lift * 0.12,
            roll: self.signals.swim_vertical as f32
                * if swimming(&self.signals) { 0.35 } else { 0.0 },
        }
    }
}

/// The pose the gait and the ported controllers add up to: the vanilla
/// comparison shape, with the turn-in-place, crouch, direction and lean
/// controllers layered on the way the Java `apply` calls do.
impl PlayerState {
    fn build_pose(&self, loco: &LocomotionFrame, tuning: &EngineTuning) -> HumanoidPose {
        let phase = loco.phase.to_radians();
        let (stride, speed) = (loco.stride.clamp(0.0, 1.0), loco.speed.clamp(0.0, 1.0));
        let swing = phase.cos() * LIMB_SWING * stride * tuning.gait_amplitude;
        let mut pose = HumanoidPose::identity();
        let signals = &self.signals;
        let glide = if signals.elytra { 1.0 } else { 0.0 };
        let swim = if swimming(signals) { 1.0 } else { 0.0 };
        let crouch = self.crouch.controller.render_amount(1.0);
        let climb = if signals.climbing { 1.0 } else { 0.0 };
        let busy = self.items.emote().is_some() as i32 as f32;

        // Legs: opposite swings, still while gliding or swimming, tucked
        // while climbing.
        pose.right_leg.x_rot = swing;
        pose.left_leg.x_rot = -swing;
        let turn_yaw = self.turn.body_yaw();
        pose.left_leg.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.1;
        pose.right_leg.y_rot = pose.left_leg.y_rot;

        // Arms: the legs' opposite swing on foot; back and out gliding; up
        // climbing; the emote's hand action.
        let arm_swing = -swing * 1.05;
        pose.right_arm.x_rot = arm_swing;
        pose.left_arm.x_rot = -arm_swing;
        let action = self.items.emote();
        let charge = if action == Some(ItemAction::Charge) {
            1.4 * busy
        } else {
            0.0
        };
        let use_arm = if action == Some(ItemAction::Use) || action == Some(ItemAction::Eat) {
            -0.9 * busy
        } else {
            0.0
        };
        let swing_arm = if action == Some(ItemAction::Swing) {
            (self.clock * std::f32::consts::PI).sin() * 1.2 * busy
        } else {
            0.0
        };
        let paddles = self.boat.sample(1.0);
        let rowing = self.riding as i32 as f32;
        pose.right_arm.x_rot += -swing_arm - charge - use_arm - paddles[1].pitch * 1.1 * rowing;
        pose.left_arm.x_rot += -paddles[0].pitch * 1.1 * rowing;
        pose.right_arm.z_rot = -(0.09 + 0.35 * glide + 0.25 * swim) - 0.35 * climb;
        pose.left_arm.z_rot = 0.09 + 0.35 * glide + 0.25 * swim + 0.35 * climb;

        // Body: crouches (through the crouch machine's amount), dips on a
        // landing, leans into a run and while gliding.
        pose.body.y = -0.12 * crouch - 0.10 * loco.lift;
        pose.body.x_rot = speed * 0.10 + 0.45 * glide - 0.18 * climb;
        pose.body.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.35;

        // Head: takes most of the turn, looks along the climb, and banks
        // with the glide.
        pose.head.y_rot = (input_yaw(self) - turn_yaw).to_radians() * 0.65;
        pose.head.x_rot = -(signals.climb_direction as f32) * 0.4 * climb - 0.20 * glide;
        pose.head.z_rot = signals.elytra_bank * 0.35 * glide;

        // The turn-in-place playback lands on the limbs, body and head, the
        // Java `apply`'s weights.
        let weight = self.turn.weight();
        if weight > 0.0 {
            let partial = 1.0;
            TurnInPlaceController::apply_part(
                &mut pose.left_leg,
                &self.turn.sample("left_leg"),
                weight * 1.0,
            );
            TurnInPlaceController::apply_part(
                &mut pose.right_leg,
                &self.turn.sample("right_leg"),
                weight * 1.0,
            );
            TurnInPlaceController::apply_part(
                &mut pose.left_arm,
                &self.turn.sample("left_arm"),
                weight * 1.0,
            );
            TurnInPlaceController::apply_part(
                &mut pose.right_arm,
                &self.turn.sample("right_arm"),
                weight * 1.0,
            );
            TurnInPlaceController::apply_part(&mut pose.body, &self.turn.sample("body"), weight);
            TurnInPlaceController::apply_part(&mut pose.head, &self.turn.sample("head"), weight);
            let _ = partial;
        }

        // Cape: streams behind with speed and glide, lifts when falling.
        pose.cape.x_rot = -(speed * 0.25 + 0.55 * glide) - 0.30 * loco.lift;
        pose.cape.z_rot = swing * 0.15;

        pose.blend = 1.0;
        pose
    }
}

/// The yaw the pose input last carried.
fn input_yaw(state: &PlayerState) -> f32 {
    state.pose_input.yaw
}

/// Every player's animation state, keyed by entity id, with the tuning they
/// share.
#[derive(Default)]
pub struct PlayerAnimationEngine {
    players: HashMap<u64, PlayerState>,
    tuning: EngineTuning,
}

impl PlayerAnimationEngine {
    /// An engine with the given tuning.
    pub fn new(tuning: EngineTuning) -> Self {
        Self {
            players: HashMap::new(),
            tuning,
        }
    }

    /// The tuning every player reads.
    pub fn tuning(&self) -> &EngineTuning {
        &self.tuning
    }

    /// Replaces the tuning; the next update uses it.
    pub fn set_tuning(&mut self, tuning: EngineTuning) {
        self.tuning = tuning;
    }

    /// One update for `entity`; the state is created on its first update.
    pub fn update(&mut self, entity: u64, signals: MotionSignals, dt: f32) -> &PlayerState {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let tuning = &self.tuning;
        let state = self.players.entry(entity).or_default();
        state.advance(signals, dt, tuning);
        state
    }

    /// One update for `entity`, carrying the pose input (the body yaw and
    /// look pitch the turn controllers read) the render handed over.
    pub fn update_with_input(
        &mut self,
        entity: u64,
        signals: MotionSignals,
        dt: f32,
        input: PoseInput,
    ) -> &PlayerState {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let tuning = &self.tuning;
        {
            let state = self.players.entry(entity).or_default();
            state.pose_input = input;
            state.advance(signals, dt, tuning);
        }
        self.players.get(&entity).expect("just inserted")
    }

    /// `entity`'s state, if it has been updated.
    pub fn state(&self, entity: u64) -> Option<&PlayerState> {
        self.players.get(&entity)
    }

    /// `entity`'s state, to change (the vehicle and fishing flags live there).
    pub fn state_mut(&mut self, entity: u64) -> Option<&mut PlayerState> {
        self.players.get_mut(&entity)
    }

    /// The pose to draw for `entity` at `partial` ticks past the last update:
    /// the two latest poses blended, or the identity pose for an entity the
    /// engine has never seen.
    pub fn sample_pose(&self, entity: u64, partial: f32) -> HumanoidPose {
        let partial = if partial.is_finite() {
            partial.clamp(0.0, 1.0)
        } else {
            1.0
        };
        match self.players.get(&entity) {
            Some(state) if partial >= 1.0 => state.pose.clone(),
            Some(state) => state.previous.lerp(&state.pose, partial),
            None => HumanoidPose::identity(),
        }
    }

    /// The whole-model transform for `entity`, if it has been updated.
    pub fn root_motion(&self, entity: u64, partial: f32) -> Option<RootMotion> {
        self.players
            .get(&entity)
            .map(|state| state.root_motion(partial))
    }

    /// `entity`'s face quads for this frame, if it has a state.
    pub fn face_quads(&self, entity: u64) -> Option<FaceQuads> {
        let state = self.players.get(&entity)?;
        Some(state.face.quads(&crate::faces::current_settings()))
    }

    /// Sets how much of each bone's base pose `entity` keeps.
    pub fn set_retention(&mut self, entity: u64, retention: BasePoseRetention) {
        self.players.entry(entity).or_default().retention = retention;
    }

    /// Whether `entity` has a state.
    pub fn contains(&self, entity: u64) -> bool {
        self.players.contains_key(&entity)
    }

    /// Forgets `entity`; whether it was there.
    pub fn remove(&mut self, entity: u64) -> bool {
        self.players.remove(&entity).is_some()
    }

    /// Forgets every player (a world unload, a dimension change).
    pub fn clear(&mut self) {
        self.players.clear();
    }

    /// How many players the engine holds.
    pub fn len(&self) -> usize {
        self.players.len()
    }

    /// Whether the engine holds no player.
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}

// ---------------------------------------------------------------------------
// The frame the game's hooks call
// ---------------------------------------------------------------------------

/// What the game's model pose state gives the engine.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PoseInput {
    /// The entity's age in ticks.
    pub age: f32,
    /// The body yaw, in degrees.
    pub yaw: f32,
    /// The look pitch, in degrees.
    pub pitch: f32,
    /// Its limb speed, `0.0..=1.0`.
    pub speed: f32,
    /// Its walk position, in the units the gait cycle advances with.
    pub position: f32,
    /// Ticks past the last update, for the pose sampled.
    pub partial: f32,
}

impl PoseInput {
    /// The movement signals this pose state implies: walking at its limb
    /// speed and nothing else.
    pub fn signals(self) -> MotionSignals {
        let speed = if self.speed.is_finite() {
            self.speed.clamp(0.0, 1.0)
        } else {
            0.0
        };
        MotionSignals {
            tick: if self.age.is_finite() {
                self.age.max(0.0) as i32
            } else {
                0
            },
            grounded: true,
            moving: speed > 0.01,
            running: speed >= 0.99,
            walk_forward: speed,
            ..MotionSignals::new()
        }
    }
}

/// One pose being built: the entity and the tick it is being drawn at.
#[derive(Debug, Clone, Copy)]
struct Posing {
    entity: u64,
    partial: f32,
}

#[derive(Default)]
struct Glue {
    engine: PlayerAnimationEngine,
    /// Seconds since the last `begin_frame`.
    frame_dt: f32,
    last_frame: Option<Instant>,
    /// The entity whose pose is being built.
    posing: Option<Posing>,
}

thread_local! {
    static GLUE: std::cell::RefCell<Option<Glue>> = const { std::cell::RefCell::new(None) };
}

/// Runs `f` with this thread's engine.
pub fn with_engine<R>(f: impl FnOnce(&mut PlayerAnimationEngine) -> R) -> R {
    GLUE.with_borrow_mut(|glue| {
        let glue = glue.get_or_insert_with(|| Glue {
            engine: PlayerAnimationEngine::default(),
            frame_dt: 1.0 / 20.0,
            last_frame: None,
            posing: None,
        });
        f(&mut glue.engine)
    })
}

/// Starts a frame and returns its length in seconds: the first call and a
/// stalled frame both read `1/20`, so nothing jumps.
pub fn begin_frame() -> f32 {
    GLUE.with_borrow_mut(|slot| {
        let glue = slot.get_or_insert_with(|| Glue {
            engine: PlayerAnimationEngine::default(),
            frame_dt: 1.0 / 20.0,
            last_frame: None,
            posing: None,
        });
        let now = Instant::now();
        let dt = glue
            .last_frame
            .map_or(1.0 / 20.0, |last| now.duration_since(last).as_secs_f32());
        glue.last_frame = Some(now);
        glue.frame_dt = dt.clamp(0.0, 1.0 / 20.0);
        glue.frame_dt
    })
}

/// The current frame's length, for a pose asked for without a frame first.
pub fn frame_dt() -> f32 {
    GLUE.with_borrow(|slot| slot.as_ref().map_or(1.0 / 20.0, |glue| glue.frame_dt))
}

/// Tells the engine which entity the player model is being posed for, and
/// steps its animation by this frame.
pub fn begin_pose(entity: u64, input: PoseInput) {
    let dt = frame_dt();
    let signals = input.signals();
    GLUE.with_borrow_mut(|slot| {
        let glue = slot.get_or_insert_with(|| Glue {
            engine: PlayerAnimationEngine::default(),
            frame_dt: dt,
            last_frame: None,
            posing: None,
        });
        glue.engine.update_with_input(entity, signals, dt, input);
        glue.posing = Some(Posing {
            entity,
            partial: if input.partial.is_finite() {
                input.partial.clamp(0.0, 1.0)
            } else {
                1.0
            },
        });
    });
}

/// The pose for the entity whose model is being posed, or `None` when the
/// game posed one the engine knows nothing about.
pub fn finish_pose() -> Option<HumanoidPose> {
    GLUE.with_borrow_mut(|slot| {
        let glue = slot.as_mut()?;
        let posing = glue.posing.take()?;
        Some(glue.engine.sample_pose(posing.entity, posing.partial))
    })
}

/// `entity`'s face quads, or `None` when it has no state.
pub fn sampled_face_quads(entity: u64) -> Option<FaceQuads> {
    GLUE.with_borrow(|slot| {
        slot.as_ref()
            .and_then(|glue| glue.engine.face_quads(entity))
    })
}

/// `entity`'s head transform, for a face drawn on the front of the head.
pub fn sampled_head_pose(entity: u64) -> Option<PartTransform> {
    GLUE.with_borrow(|slot| {
        slot.as_ref()
            .and_then(|glue| glue.engine.state(entity))
            .map(|state| state.pose.head)
    })
}

/// `entity`'s whole-model transform, or `None` when it has no state.
pub fn sampled_root_motion(entity: u64, partial: f32) -> Option<RootMotion> {
    GLUE.with_borrow(|slot| {
        slot.as_ref()
            .and_then(|glue| glue.engine.root_motion(entity, partial))
    })
}

/// Forgets every player and the frame clock (a world unload).
pub fn reset() {
    GLUE.with_borrow_mut(|slot| {
        if let Some(glue) = slot.as_mut() {
            glue.engine.clear();
            glue.posing = None;
            glue.frame_dt = 1.0 / 20.0;
            glue.last_frame = None;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(name: &str, length: f32, bones: &[(&str, [f32; 3])]) -> Clip {
        let mut map = std::collections::HashMap::new();
        for (bone, rotation) in bones {
            // One linear channel: the rotation ramps 0..=value over the clip.
            let mut keys = Vec::new();
            for t in [0.0f32, length] {
                keys.push((
                    t,
                    crate::library::Key {
                        pre: Vec3::new(
                            rotation[0] * t / length,
                            rotation[1] * t / length,
                            rotation[2] * t / length,
                        ),
                        post: Vec3::new(
                            rotation[0] * t / length,
                            rotation[1] * t / length,
                            rotation[2] * t / length,
                        ),
                        interpolation: "linear".into(),
                        easing: None,
                    },
                ));
            }
            map.insert(
                bone.to_string(),
                crate::library::BoneTrack::new(
                    crate::library::Channel::new(keys.clone(), Vec3::ZERO),
                    crate::library::Channel::new(keys.clone(), Vec3::ZERO),
                    crate::library::Channel::new(keys, Vec3::ONE),
                ),
            );
        }
        Clip::new(name.to_string(), length, false, map)
    }

    #[test]
    fn a_step_turn_eases_the_body_across_the_clip() {
        let left = clip("step_left", 0.3, &[("root", [0.0, -45.0, 0.0])]);
        let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
        let settings = TurnSettings {
            enabled: true,
            threshold: 45.0,
            delay: 2.0,
            fade: 0.12,
            mode: TurnMode::Steps,
            reference: 45.0,
            double_turn: 75.0,
            template: None,
            left: Some(left),
            right: Some(right),
            left_add: None,
            right_add: None,
        };
        let mut turn = TurnInPlaceController::new();
        // The look jumps 60 degrees right while standing: the body follows
        // through the clip.
        let mut time = 0.0;
        turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        time += 0.05;
        turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        time += 0.05;
        turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        assert!(turn.weight() > 0.0, "the step plays");
        // Half way through the 0.3 s clip the body has turned most of the way.
        let mut yaw = 0.0;
        for _ in 0..4 {
            time += 0.05;
            yaw = turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        }
        assert!(yaw > 20.0, "{}", yaw);
        // Settled at the target.
        for _ in 0..20 {
            time += 0.05;
            yaw = turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        }
        assert!((yaw - 60.0).abs() < 1e-3, "{}", yaw);
        assert!((turn.weight()).abs() < 1e-4, "the playback is spent");
    }

    #[test]
    fn a_step_turn_releases_back_to_the_feet_while_moving() {
        let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
        let settings = TurnSettings {
            enabled: true,
            threshold: 45.0,
            delay: 2.0,
            fade: 0.12,
            mode: TurnMode::Steps,
            reference: 45.0,
            double_turn: 75.0,
            template: None,
            left: None,
            right: Some(right),
            left_add: None,
            right_add: None,
        };
        let mut turn = TurnInPlaceController::new();
        let mut time = 0.0;
        turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        time += 0.05;
        turn.update(0.0, 60.0, 0.0, time, true, true, false, &settings);
        // Walking again hands the yaw back to the feet over the fade.
        time += 0.05;
        let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
        assert!(yaw < 60.0);
        for _ in 0..10 {
            time += 0.05;
            let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
            let _ = yaw;
        }
        time += 0.05;
        let yaw = turn.update(10.0, 60.0, 0.0, time, true, false, false, &settings);
        assert!((yaw - 10.0).abs() < 1e-3, "{}", yaw);
    }

    #[test]
    fn the_camera_turn_tracks_the_look() {
        let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
        let settings = TurnSettings {
            enabled: true,
            threshold: 45.0,
            delay: 2.0,
            fade: 0.12,
            mode: TurnMode::Camera,
            reference: 45.0,
            double_turn: 75.0,
            template: None,
            left: None,
            right: Some(right),
            left_add: None,
            right_add: None,
        };
        let mut turn = TurnInPlaceController::new();
        let mut time = 0.0f64;
        // The look moves 30 degrees between calls: the body tracks it.
        turn.update(0.0, 0.0, 0.0, time, true, true, false, &settings);
        time += 0.05;
        let yaw = turn.update(0.0, 30.0, 0.0, time, true, true, false, &settings);
        assert!((yaw - 30.0).abs() < 1e-3, "{}", yaw);
        // The clip plays once the look settles past the input gap.
        time += 0.05;
        turn.update(30.0, 30.0, 0.0, time, true, true, false, &settings);
        assert!(turn.weight() > 0.0 || turn.body_yaw() == 30.0);
    }

    #[test]
    fn the_keyframed_turn_splits_a_double_turn() {
        let left = clip("step_left", 0.3, &[("root", [0.0, -45.0, 0.0])]);
        let right = clip("step_right", 0.3, &[("root", [0.0, 45.0, 0.0])]);
        let settings = TurnSettings {
            enabled: true,
            threshold: 45.0,
            delay: 0.12,
            fade: 0.12,
            mode: TurnMode::Dynamic,
            reference: 45.0,
            double_turn: 75.0,
            template: None,
            left: Some(left),
            right: Some(right),
            left_add: None,
            right_add: None,
        };
        let mut turn = TurnInPlaceController::new();
        let mut time = 0.0f64;
        // A 160-degree look swing: the first turn covers 48% of it.
        turn.update(0.0, 0.0, 0.0, time, true, true, false, &settings);
        time += 0.05;
        turn.update(0.0, 160.0, 0.0, time, true, true, false, &settings);
        // The swing splits: the first clip passes near 48% of it, the
        // second completes the turn.
        let mut yaw = 0.0;
        let mut hit_mid = false;
        for _ in 0..40 {
            time += 0.05;
            yaw = turn.update(160.0, 160.0, 0.0, time, true, true, false, &settings);
            if (yaw - 160.0 * 0.48).abs() < 12.0 {
                hit_mid = true;
            }
        }
        assert!(hit_mid, "the first clip covered about half the swing");
        assert!((yaw - 160.0).abs() < 2.0, "{}", yaw);
    }

    #[test]
    fn playback_plays_stops_and_holds() {
        let mut player = ClipSequencePlayer::new();
        let clip = clip("wave", 0.4, &[("right_arm", [0.0, 0.0, 30.0])]);
        player.play(
            Some(clip.clone()),
            Vec::new(),
            false,
            0.1,
            false,
            BasePoseRetention::default(),
            false,
            None,
        );
        assert!(player.active());
        assert!(!player.finished());
        player.tick(0.2);
        assert!(
            (player.progress() - 0.5).abs() < 1e-4,
            "{}",
            player.progress()
        );
        // The rotation ramps linearly to half its value.
        let sample = player.sample("right_arm", 1.0);
        assert!(
            (sample.pose.rotation.z - 15.0).abs() < 1e-3,
            "{}",
            sample.pose.rotation.z
        );
        player.hold_last_frame();
        assert!((player.render_time(1.0) - 0.4).abs() < 1e-4);
        assert!(player.finished());
        player.stop(0.1);
        player.tick(0.2);
        player.tick(0.05);
        assert!(!player.active());
    }

    #[test]
    fn playback_cross_blends_into_the_next_clip() {
        let mut player = ClipSequencePlayer::new();
        let a = clip("a", 0.4, &[("right_arm", [0.0, 0.0, 30.0])]);
        let b = clip("b", 0.4, &[("right_arm", [0.0, 0.0, 60.0])]);
        player.play(
            Some(a),
            Vec::new(),
            false,
            0.2,
            false,
            BasePoseRetention::default(),
            false,
            None,
        );
        player.tick(0.1);
        player.play(
            Some(b),
            Vec::new(),
            false,
            0.2,
            false,
            BasePoseRetention::default(),
            false,
            None,
        );
        // Mid-blend the sample sits between the two clips.
        player.tick(0.1);
        let sample = player.sample("right_arm", 1.0);
        assert!(
            sample.pose.rotation.z > 0.0 && sample.pose.rotation.z < 60.0,
            "{}",
            sample.pose.rotation.z
        );
        // The blend settles on the new clip.
        for _ in 0..20 {
            player.tick(0.05);
        }
        let sample = player.sample("right_arm", 1.0);
        assert!((sample.pose.rotation.z - 60.0).abs() < 1e-3);
    }

    #[test]
    fn exclusive_overlays_replace_their_tracks() {
        let base = clip(
            "base",
            0.4,
            &[("right_arm", [0.0, 0.0, 30.0]), ("head", [0.0, 10.0, 0.0])],
        );
        let over = clip("over", 0.4, &[("right_arm", [0.0, 0.0, 90.0])]);
        let merged = merge_exclusive_tracks(
            &base,
            &[ResolvedOverlay {
                clip: over.clone(),
                mode: OverlayMode::Override,
                bones: Vec::new(),
            }],
        );
        let sample = merged.sample_once("right_arm", 0.2);
        assert!(
            (sample.rotation.z - 45.0).abs() < 1e-3,
            "the overlay's track wins"
        );
        let head = merged.sample_once("head", 0.2);
        assert!((head.rotation.y - 5.0).abs() < 1e-3, "the others stay");
        assert!(has_track(Some(&merged), "right_arm"));
        assert!(!has_track(None, "right_arm"));
    }

    #[test]
    fn compose_keeps_the_retained_part_of_the_base() {
        let base = Pose {
            position: Vec3::ZERO,
            rotation: Vec3::new(10.0, 0.0, 0.0),
            scale: Vec3::ONE,
        };
        let sample = Sample {
            pose: Pose {
                position: Vec3::ZERO,
                rotation: Vec3::new(30.0, 0.0, 0.0),
                scale: Vec3::ONE,
            },
            weight: 1.0,
            base_retain: 0.5,
            legacy_retain_weight: 0.0,
        };
        let composed = compose(base, sample, 0.0);
        // Half the base rotation kept, plus the clip's.
        assert!(
            composed.rotation.x > 30.0 && composed.rotation.x < 40.0,
            "{}",
            composed.rotation.x
        );
        // Zero weight leaves the base alone.
        let untouched = compose(
            base,
            Sample {
                weight: 0.0,
                ..sample
            },
            0.0,
        );
        assert_eq!(untouched, base);
    }

    #[test]
    fn mix_rotation_goes_the_short_way() {
        let mixed = mix_rotation(Vec3::new(170.0, 0.0, 0.0), Vec3::new(-170.0, 0.0, 0.0), 0.5);
        assert!(
            mixed.x.abs() < 1e-3
                || (mixed.x - 180.0).abs() < 1e-3
                || (mixed.x + 180.0).abs() < 1e-3,
            "{}",
            mixed.x
        );
        assert_eq!(
            mix_rotation(Vec3::new(10.0, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), 0.0).x,
            10.0
        );
        assert_eq!(
            mix_rotation(Vec3::new(10.0, 0.0, 0.0), Vec3::new(30.0, 0.0, 0.0), 1.0).x,
            30.0
        );
    }

    #[test]
    fn the_clip_pose_blend_runs_its_clock() {
        let mut blend = ClipPoseBlend::new();
        let clip = clip("pose", 0.4, &[("body", [0.0, 0.0, 40.0])]);
        blend.start(Some(clip), 0.0, 1.0, false, 0.0, Vec::new());
        blend.tick(0.2, 0.0, 0.2, 1.0);
        let pose = blend.sample("body", 1.0);
        assert!((pose.rotation.z - 20.0).abs() < 1e-3, "{}", pose.rotation.z);
        // The carry form reads the same clip without the local arms.
        let carry = blend.sample_carry("body", 1.0);
        assert!((carry.rotation.z - 20.0).abs() < 1e-3);
        blend.clear();
        let cleared = blend.sample("body", 1.0);
        assert_eq!(cleared, Pose::identity());
    }

    #[test]
    fn the_direction_change_leans_within_three_degrees() {
        let mut direction = DirectionChangeController::default();
        // Walk north for two ticks, then turn hard east: the lean clamps at
        // 3 degrees.
        direction.tick(1, 0.0, 1.0, 0.0, true);
        direction.tick(2, 0.0, 1.0, 0.0, true);
        let mut peak = 0.0f32;
        for tick in 3..13 {
            direction.tick(tick, 1.0, 0.0, 90.0, true);
            peak = peak.max(direction.pitch(1.0).abs());
        }
        assert!(peak <= 3.0 + 1e-3, "{}", peak);
        assert!(peak > 0.5, "{}", peak);
        // Standing still long enough resets the tracker.
        for tick in 13..21 {
            direction.tick(tick, 0.0, 0.0, 90.0, true);
        }
        assert!(
            direction.pitch(1.0).abs() < 0.01,
            "{}",
            direction.pitch(1.0)
        );
    }

    #[test]
    fn the_turn_lean_scales_with_speed_and_clamps() {
        let mut lean = TurnLeanController::default();
        // Full speed, a hard look turn and full sideways input: the lean
        // clamps at 14.
        for tick in 1..20 {
            lean.tick(
                tick,
                -60.0 * tick as f32,
                0.215,
                1.0,
                1.0,
                true,
                false,
                false,
            );
        }
        assert!(
            (lean.sample(1.0).abs() - 14.0).abs() < 1.0,
            "{}",
            lean.sample(1.0)
        );
        // Standing still eases back to zero.
        for tick in 20..40 {
            lean.tick(tick, 0.0, 0.0, 0.0, 0.0, true, false, false);
        }
        assert!(lean.sample(1.0).abs() < 0.5, "{}", lean.sample(1.0));
        // Invisible, the lean reads zero.
        let mut lean = TurnLeanController::default();
        lean.tick(1, 0.0, 0.215, 1.0, 1.0, false, false, false);
        assert_eq!(lean.sample(1.0), 0.0);
    }

    #[test]
    fn the_creative_flight_hover_needs_steady_heights() {
        let mut motion = CreativeFlightMotion::default();
        // Eight ticks at the same height: the hover is on.
        let mut flying = false;
        for tick in 0..10 {
            flying = motion.update(tick, 64.0, true);
        }
        assert!(flying);
        // A steady descent ends the hover while the fall builds up.
        let mut ended = false;
        for tick in 10..24 {
            if !motion.update(tick, 64.0 - (tick - 10) as f64 * 0.4, true) {
                ended = true;
            }
        }
        assert!(ended, "the accelerating descent ended the hover");
        // On the ground it never starts.
        let mut motion = CreativeFlightMotion::default();
        for tick in 0..10 {
            assert!(!motion.update(tick, 64.0, false));
        }
    }

    #[test]
    fn the_swim_alignment_eases_toward_the_surface() {
        let mut swim = SwimSurfaceAlignment::default();
        swim.set_reference_height(0.6);
        swim.prepare_eye_height(1.62);
        swim.tick(true, true, 63.0, 0.12);
        for _ in 0..20 {
            swim.tick(true, true, 63.0, 0.12);
        }
        assert!(swim.active());
        // The eye height smoothed towards its target.
        swim.prepare_eye_height(1.42);
        swim.prepare_eye_height(1.42);
        let eye = swim.camera_eye_height(62.0, 1.0, 1.0, 1.62);
        assert!(eye < 1.62, "{}", eye);
        swim.clear();
        assert!(!swim.active());
    }

    #[test]
    fn the_fishing_bob_falls_and_is_pulled_in() {
        let mut swing = FishingSwing::default();
        // The first frame hangs straight down the line's length.
        let direction = swing.update([0.0, 64.0, 0.0], 2.0, 0.0);
        assert_eq!(direction, [0.0, -1.0, 0.0]);
        // Time passing: the bob stays on the line, so the direction keeps
        // pointing down the line.
        let direction = swing.update([0.0, 64.0, 0.0], 2.0, 0.5);
        assert!(direction[1] < 0.0, "{:?}", direction);
        // A broken frame restarts the swing.
        let direction = swing.update([0.0, 64.0, 0.0], 2.0, 100.0);
        assert_eq!(direction, [0.0, -1.0, 0.0]);
    }

    #[test]
    fn the_paddles_ease_between_strokes() {
        // The vanilla pose at phase 0 for the left paddle.
        let rest = paddle_vanilla_pose(0, 0.0);
        assert!(
            (rest.pitch - (-std::f32::consts::PI / 3.0 - std::f32::consts::PI / 12.0) / 2.0).abs()
                < 1e-4
        );
        let mut paddle = Paddle::new(0, 1, 0.0);
        // One stroke step moves the phase by an eighth of a turn.
        paddle.tick(1, PADDLE_STEP);
        let mid = paddle.sample(0.5);
        assert!(mid.pitch.is_finite());
        // Stopping eases back over the longer 0.3 s.
        paddle.tick(0, 0.0);
        for _ in 0..8 {
            paddle.tick(0, 0.0);
        }
        let settled = paddle.sample(1.0);
        assert!(
            (settled.pitch - paddle_vanilla_pose(0, 0.0).pitch).abs() < 1e-3,
            "{:?}",
            settled
        );
    }

    #[test]
    fn the_boat_weight_cubes_in() {
        let mut boat = BoatAnimation::new(1, 0, [false, false], [0.0, 0.0], false);
        boat.step(1, 1, false, [true, true], false);
        let early = boat.weight(1.0);
        boat.step(2, 1, false, [true, true], false);
        boat.step(3, 1, false, [true, true], false);
        let later = boat.weight(1.0);
        assert!(later > early);
        assert!(later <= 1.0);
        // The vanilla pose mirrors across the boat.
        let left = paddle_vanilla_pose(0, 0.0);
        let mirrored = paddle_opposite_side(left);
        assert!((mirrored.yaw - (std::f32::consts::PI - left.yaw)).abs() < 1e-5);
    }

    #[test]
    fn the_elytra_correction_is_ninety_degrees_of_the_weight() {
        assert_eq!(elytra_correction_degrees(true, true, false, 1.0), 90.0);
        assert_eq!(elytra_correction_degrees(true, true, false, 0.5), 45.0);
        assert_eq!(
            elytra_correction_degrees(true, true, false, 2.0),
            90.0,
            "clamped"
        );
        assert_eq!(
            elytra_correction_degrees(true, true, true, 1.0),
            0.0,
            "paused"
        );
        assert_eq!(elytra_correction_degrees(false, true, false, 1.0), 0.0);
        assert_eq!(elytra_correction_degrees(true, true, false, f32::NAN), 0.0);
    }

    #[test]
    fn the_held_item_matrix_translates_by_sixteenths() {
        let matrix = held_item_matrix(Vec3::new(16.0, 0.0, -32.0), Vec3::ZERO, Vec3::ONE);
        let values = matrix.values();
        // The translation column: (-1, 2, 0).
        assert!((values[12] - (-1.0)).abs() < 1e-5);
        assert!((values[13] - 2.0).abs() < 1e-5);
        assert!((values[14] - 0.0).abs() < 1e-5);
        // A rotation turns the matrix.
        let turned = held_item_matrix(Vec3::ZERO, Vec3::new(0.0, 0.0, 90.0), Vec3::ONE);
        assert!((turned.values()[0] - 90.0f32.to_radians().cos()).abs() < 1e-5);
    }

    #[test]
    fn the_pose_ownership_blend_eases_between_owners() {
        let mut blend = PoseOwnershipBlend::default();
        let mut parts: Vec<(&'static str, [f32; 9])> =
            vec![("head", [0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0])];
        // The mod owns the part first: the pose is the mod's.
        let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
        blend.apply(1, &mut heads, true, 0.0, true);
        // Vanilla takes over: over 3 seconds the rotation eases back.
        let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
        blend.apply(1, &mut heads, false, 0.1, true);
        let after = parts[0].1[3];
        assert!(
            (after - 0.5).abs() < 1e-6,
            "the blend starts where the mod left it"
        );
        // Settled, vanilla's value stands: vanilla writes its own pose into
        // the parts every frame, then the blend mixes the snapshot away.
        for step in 1..60 {
            let time = 0.1 + step as f32 * 0.1;
            parts[0].1[3] = 0.0;
            let mut heads: Vec<(&'static str, &mut [f32; 9])> = vec![("head", &mut parts[0].1)];
            blend.apply(1, &mut heads, false, time, true);
        }
        assert!(parts[0].1[3].abs() < 0.01, "{}", parts[0].1[3]);
    }

    #[test]
    fn the_outer_ownership_blend_keeps_the_transform_rigid() {
        let mut blend = OuterPoseOwnershipBlend::default();
        // The mod's matrix is a pure translation; the render's is the same
        // translation, so the delta is the identity and nothing moves.
        let outer = *Mat4Builder::identity().translate(1.0, 2.0, 3.0).values();
        let mut render = outer;
        blend.apply(1, Some(&outer), &mut render, true, 0.0, true);
        assert!((render[12] - 1.0).abs() < 1e-5);
        assert!((render[13] - 2.0).abs() < 1e-5);
        assert!((render[14] - 3.0).abs() < 1e-5);
        // A scaled outer matrix is not rigid: the track is dropped.
        let mut scaled = *Mat4Builder::identity().scale(2.0, 2.0, 2.0).values();
        blend.apply(2, Some(&scaled), &mut render, true, 0.0, true);
        scaled = *Mat4Builder::identity().translate(0.0, 0.0, 0.0).values();
        blend.apply(2, Some(&scaled), &mut render, false, 0.1, true);
    }

    #[test]
    fn climb_facing_reads_the_block() {
        // A `facing` property wins, read as its opposite.
        assert_eq!(
            climb_facing_yaw(Some("north"), &[], |_| false, 0.0, 0.0, 0.0),
            climb_direction_yaw("south"),
        );
        // Boolean flags pick the nearest face.
        let yaw = climb_facing_yaw(
            None,
            &[("north", true), ("east", true)],
            |_| false,
            0.1,
            0.1,
            0.0,
        );
        assert_eq!(
            yaw,
            climb_direction_yaw("north"),
            "north is nearest at x 0.1"
        );
        // Nothing known: the player's yaw stands.
        assert_eq!(climb_facing_yaw(None, &[], |_| false, 0.5, 0.5, 42.0), 42.0);
        // The climbable neighbours break the tie.
        let yaw = climb_facing_yaw(None, &[], |d| d == "west", 0.5, 0.5, 0.0);
        assert_eq!(yaw, climb_direction_yaw("west"));
    }

    #[test]
    fn the_engine_glue_still_poses_and_reads_root_motion() {
        GLUE.with_borrow_mut(|slot| *slot = Some(Glue::default()));
        let pose = with_engine(|engine| {
            let state = engine.update(7, MotionSignals::new(), 0.05);
            state.pose().clone()
        });
        assert!(pose.right_arm.x_rot.abs() < 1e-5, "still, the arms hang");
        assert_eq!(pose.head, PartTransform::IDENTITY);
        let root = with_engine(|engine| engine.root_motion(7, 1.0));
        assert_eq!(root, Some(RootMotion::default()));
        with_engine(|engine| engine.clear());
        GLUE.with_borrow_mut(|slot| *slot = Some(Glue::default()));
    }

    #[test]
    fn the_part_transform_carries_scale() {
        let mut part = PartTransform::IDENTITY;
        TurnInPlaceController::apply_part(
            &mut part,
            &Pose {
                position: Vec3::new(1.0, 2.0, 3.0),
                rotation: Vec3::new(90.0, 0.0, 0.0),
                scale: Vec3::new(2.0, 1.0, 1.0),
            },
            1.0,
        );
        assert!((part.x - 1.0).abs() < 1e-6);
        assert!(
            (part.y + 2.0).abs() < 1e-6,
            "the y flips, like the Java apply"
        );
        assert!((part.z - 3.0).abs() < 1e-6);
        assert!((part.x_rot - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((part.x_scale - 2.0).abs() < 1e-6);
        assert!((part.y_scale - 1.0).abs() < 1e-6);
    }

    #[test]
    fn quaternion_round_trips_through_euler() {
        for (x, y, z) in [(30.0, 45.0, 60.0), (-20.0, 0.0, 90.0), (0.0, 0.0, 0.0)] {
            let q = Quat::from_zyx_degrees(Vec3::new(x, y, z));
            let back = q.to_euler_zyx_degrees();
            assert!((back.x - x).abs() < 1e-3, "{} vs {}", back.x, x);
            assert!((back.y - y).abs() < 1e-3, "{} vs {}", back.y, y);
            assert!((back.z - z).abs() < 1e-3, "{} vs {}", back.z, z);
        }
        // Slerp walks the short arc.
        let a = Quat::from_zyx_degrees(Vec3::new(170.0, 0.0, 0.0));
        let b = Quat::from_zyx_degrees(Vec3::new(-170.0, 0.0, 0.0));
        let mid = a.slerp(b, 0.5).to_euler_zyx_degrees();
        // The short arc from +170 to -170 runs through 180.
        assert!((mid.x.abs() - 180.0).abs() < 1e-2, "{}", mid.x);
    }

    #[test]
    fn the_item_sequence_starts_and_idles_without_a_library() {
        let mut controller = ItemSequenceController::new();
        let input = ItemInput::default();
        controller.tick(
            &input, None, true, true, true, 0.0, false, false, None, "", false, 0.0, 0.0,
        );
        assert!(!controller.active(), "no pack, no playback");
        assert!(!controller.replaces_falling());
        // The mace helpers read as the Java statics do.
        assert!(ItemSequenceController::starts_mace_charge(
            false, -1.0, 4.0, 3.0, false
        ));
        assert!(!ItemSequenceController::starts_mace_charge(
            true, -1.0, 4.0, 3.0, false
        ));
        assert!(ItemSequenceController::can_mace_smash(false, 2.0, 1.5));
        assert!(!ItemSequenceController::can_mace_smash(true, 2.0, 1.5));
        assert!(ItemSequenceController::supports_use("bow", "release"));
        assert!(!ItemSequenceController::supports_use("bow", "charge"));
        assert!(ItemSequenceController::supports_use("fishing", "cast"));
        assert!(ItemSequenceController::continues_offhand_action(
            "release", false, false
        ));
        assert!(!ItemSequenceController::continues_offhand_action(
            "idle", false, false
        ));
    }

    #[test]
    fn the_crouch_machine_tracks_its_amount_without_clips() {
        let mut crouch = CrouchSequenceController::new();
        for _ in 0..5 {
            crouch.tick(true, false, None, None, None, 1.0, false);
        }
        // Sprinting crouches with no clips read as zero, and the amount
        // eases down at 0.2 a tick.
        assert!(crouch.render_amount(1.0) <= 1.0);
        crouch.tick(false, false, None, None, None, 1.0, false);
        assert_eq!(crouch.phase(), "");
        assert!(!crouch.is_active());
    }

    #[test]
    fn the_turn_settings_read_the_library_defaults() {
        let library = crate::library::AnimationLibrary::new(
            HashMap::new(),
            crate::library::ItemAnimationBindings::new(),
        );
        let settings = TurnSettings::from_library(&library);
        assert!(!settings.enabled, "off by default");
        assert_eq!(settings.mode, TurnMode::Steps);
        assert_eq!(settings.threshold, 45.0);
        assert_eq!(settings.reference, 45.0);
        assert_eq!(settings.double_turn, 75.0);
        assert_eq!(settings.fade, 0.12);
        assert_eq!(settings.delay, 2.0);
    }

    #[test]
    fn mirror_fallback_and_sampled_bones_follow_the_java() {
        let plain = clip("swing", 0.2, &[]);
        assert!(!mirror_fallback(Some(&plain), false));
        assert!(
            mirror_fallback(Some(&plain), true),
            "no _left suffix mirrors"
        );
        let left = clip("swing_left", 0.2, &[]);
        assert!(!mirror_fallback(Some(&left), true));
        assert!(!mirror_fallback(None, true));
        assert_eq!(sampled_bone("left_arm", true), "right_arm");
        assert_eq!(sampled_bone("right_leg", true), "left_leg");
        assert_eq!(sampled_bone("body", true), "body");
        assert_eq!(sampled_bone("head", false), "head");
    }
}

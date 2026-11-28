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

use std::collections::HashMap;
use std::time::Instant;

use crate::faces::{FaceQuads, FaceRig};
use crate::signals::{BasePoseRetention, GaitPhaseMatcher, MotionSignals, MovementTuning};

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
            retention: BasePoseRetention::new(),
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
            let kept = self.retention.get(name).clamp(0.0, 1.0);
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
    matcher: GaitPhaseMatcher,
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
            matcher: GaitPhaseMatcher::new(),
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

    /// The gait phase, degrees.
    pub fn phase(&self) -> f32 {
        self.matcher.phase()
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
        tuning: &MovementTuning,
    ) -> LocomotionFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let speed = if signals.moving {
            signals.locomotion()
        } else {
            0.0
        };
        // The cycle advances with the blocks travelled, so it never outruns
        // the feet however the frame rate moves.
        self.matcher
            .advance(speed * WALK_SPEED * dt, tuning.stride_distance);
        let target_stride = if signals.moving { speed.max(0.35) } else { 0.0 };
        self.stride = tuning.fade(self.stride, target_stride, dt);
        self.lift = tuning.fade(self.lift, signals.fall_weight.clamp(0.0, 1.0), dt);
        if signals.running && signals.moving {
            self.run_seconds += dt;
        } else {
            self.run_seconds = 0.0;
        }
        LocomotionFrame {
            phase: self.matcher.phase(),
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

/// The most the torso twists away from the feet, in degrees.
pub const TURN_LIMIT_DEGREES: f32 = 45.0;
/// How fast the torso follows a strafe, in degrees a second.
pub const TURN_RATE: f32 = 180.0;
/// Above this twist the feet take a step, in degrees.
pub const TURN_STEP_DEGREES: f32 = 20.0;
/// How fast the step eases in and out, per second.
const STEP_RATE: f32 = 3.0;

/// Turns the player on the spot: standing still and strafing twists the torso
/// first, and past [`TURN_STEP_DEGREES`] the feet follow in a step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnInPlaceController {
    /// How far the torso is turned from the feet, degrees.
    body_offset: f32,
    /// The step's amount, `0.0..=1.0`.
    step: f32,
}

/// One update's turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnFrame {
    /// The torso's twist away from the feet, degrees.
    pub body_offset: f32,
    /// How far the feet have turned, `0.0..=1.0`.
    pub step: f32,
    /// Whether a step is being taken.
    pub turning: bool,
}

impl Default for TurnInPlaceController {
    fn default() -> Self {
        Self {
            body_offset: 0.0,
            step: 0.0,
        }
    }
}

impl TurnInPlaceController {
    pub fn new() -> Self {
        Self::default()
    }

    /// The torso's twist, degrees.
    pub fn body_offset(&self) -> f32 {
        self.body_offset
    }

    /// The step's amount.
    pub fn step(&self) -> f32 {
        self.step
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame: `left` is the strafe input (`-1.0` right, `1.0` left) and
    /// `moving` whether the player is walking.
    pub fn update(&mut self, left: f32, moving: bool, dt: f32) -> TurnFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let left = if left.is_finite() {
            left.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let target = if moving {
            0.0
        } else {
            left * TURN_LIMIT_DEGREES
        };
        let step = TURN_RATE * dt;
        let gap = target - self.body_offset;
        self.body_offset = if gap.abs() <= step {
            target
        } else {
            self.body_offset + step * gap.signum()
        };
        let turning = !moving && self.body_offset.abs() >= TURN_STEP_DEGREES;
        self.step = approach(self.step, if turning { 1.0 } else { 0.0 }, STEP_RATE * dt);
        TurnFrame {
            body_offset: self.body_offset,
            step: self.step,
            turning,
        }
    }
}

// ---------------------------------------------------------------------------
// ItemSequenceController
// ---------------------------------------------------------------------------

/// A hand action an animation answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemAction {
    /// Nothing in hand.
    #[default]
    None,
    /// A swung tool or weapon.
    Swing,
    /// A right-click use (a block, a bucket, a food, a potion).
    Use,
    /// Eating or drinking, which keeps the arm busy for longer.
    Eat,
    /// A crossbow, spear or trident being wound up.
    Charge,
}

impl ItemAction {
    /// How long one pass of this action takes, in seconds.
    pub fn duration(self) -> f32 {
        match self {
            ItemAction::None => 0.0,
            ItemAction::Swing => 0.35,
            ItemAction::Use => 0.6,
            ItemAction::Eat => 1.6,
            ItemAction::Charge => 1.25,
        }
    }

    /// Whether the action is held, so its animation repeats while the keys
    /// stay down instead of ending after one pass.
    pub fn held(self) -> bool {
        matches!(self, ItemAction::Charge)
    }

    /// The action the movement signals ask for: a charge outranks mining,
    /// and a held map only counts when nothing else is happening.
    pub fn from_signals(signals: &MotionSignals) -> ItemAction {
        if signals.charging_crossbow || signals.charging_spear || signals.charging_trident {
            ItemAction::Charge
        } else if signals.mining {
            ItemAction::Swing
        } else if signals.map_view {
            ItemAction::Use
        } else {
            ItemAction::None
        }
    }
}

/// Plays one hand action at a time and reports how far through it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemSequenceController {
    action: ItemAction,
    elapsed: f32,
    weight: f32,
}

/// One update's hand action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceFrame {
    /// The action playing.
    pub action: ItemAction,
    /// How far through a pass it is, `0.0..=1.0`.
    pub progress: f32,
    /// How much the action's pose shows, `0.0..=1.0`.
    pub weight: f32,
}

/// How fast a hand action's pose eases in and out, per second.
const ITEM_WEIGHT_RATE: f32 = 8.0;

impl Default for ItemSequenceController {
    fn default() -> Self {
        Self {
            action: ItemAction::None,
            elapsed: 0.0,
            weight: 0.0,
        }
    }
}

impl ItemSequenceController {
    pub fn new() -> Self {
        Self::default()
    }

    /// The action playing.
    pub fn action(&self) -> ItemAction {
        self.action
    }

    /// How much the action's pose shows.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Starts `action` from the beginning, replacing whatever was playing.
    pub fn play(&mut self, action: ItemAction) {
        self.action = action;
        self.elapsed = 0.0;
    }

    /// Stops the action at once.
    pub fn cancel(&mut self) {
        self.action = ItemAction::None;
        self.elapsed = 0.0;
        self.weight = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame: starts whatever the signals ask for, and advances or ends
    /// the action already playing.
    pub fn update(&mut self, signals: &MotionSignals, dt: f32) -> SequenceFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let wanted = ItemAction::from_signals(signals);
        if wanted != self.action {
            if wanted == ItemAction::None {
                // The keys were let go: the pose eases out.
                self.action = ItemAction::None;
            } else {
                self.play(wanted);
            }
        }
        let mut progress = 0.0;
        if self.action != ItemAction::None {
            self.elapsed += dt;
            let duration = self.action.duration();
            let pass = self.elapsed / duration;
            if self.action.held() {
                // A charge pulses while it is held.
                progress = pass.fract();
            } else if pass >= 1.0 {
                // A one-pass action ends when it is done.
                self.action = ItemAction::None;
            } else {
                progress = pass;
            }
        }
        let playing = self.action != ItemAction::None;
        self.weight = approach(
            self.weight,
            if playing { 1.0 } else { 0.0 },
            ITEM_WEIGHT_RATE * dt,
        );
        SequenceFrame {
            action: self.action,
            progress,
            weight: self.weight,
        }
    }
}

// ---------------------------------------------------------------------------
// BoatPaddleAnimation
// ---------------------------------------------------------------------------

/// How fast a paddle stroke runs, in radians a second.
const PADDLE_RATE: f32 = 4.2;
/// How fast the paddling eases in and out, per second.
const PADDLE_FADE: f32 = 4.0;

/// Alternates the two arms' paddling strokes while the player is in a boat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatPaddleAnimation {
    phase: f32,
    amplitude: f32,
}

/// One update's paddling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaddleFrame {
    /// The left arm's stroke, `-1.0..=1.0`.
    pub left: f32,
    /// The right arm's stroke, the other side.
    pub right: f32,
    /// How much the paddling shows, `0.0..=1.0`.
    pub amplitude: f32,
    /// The stroke's phase, radians in `0.0..TAU`.
    pub phase: f32,
}

impl Default for BoatPaddleAnimation {
    fn default() -> Self {
        Self {
            phase: 0.0,
            amplitude: 0.0,
        }
    }
}

impl BoatPaddleAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }

    pub fn amplitude(&self) -> f32 {
        self.amplitude
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame; `paddling` is whether the boat is being rowed.
    pub fn update(&mut self, paddling: bool, dt: f32) -> PaddleFrame {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        self.amplitude = approach(
            self.amplitude,
            if paddling { 1.0 } else { 0.0 },
            PADDLE_FADE * dt,
        );
        if self.amplitude > 0.0 {
            self.phase = (self.phase + PADDLE_RATE * dt) % std::f32::consts::TAU;
        }
        let stroke = self.phase.sin() * self.amplitude;
        PaddleFrame {
            left: stroke,
            right: -stroke,
            amplitude: self.amplitude,
            phase: self.phase,
        }
    }
}

// ---------------------------------------------------------------------------
// FishingLinePhysics
// ---------------------------------------------------------------------------

/// The most the line hangs below its chord, in blocks.
pub const MAX_SAG: f32 = 0.45;
/// The spring pulling the line towards its sag, per second squared.
const SAG_STIFFNESS: f32 = 60.0;
/// How quickly the sag's speed dies away, per second.
const SAG_DAMPING: f32 = 8.0;

/// A fishing line's sag, integrated as a damped spring so a cast snaps taut
/// and the line settles instead of flipping.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FishingLinePhysics {
    sag: f32,
    velocity: f32,
}

impl FishingLinePhysics {
    pub fn new() -> Self {
        Self::default()
    }

    /// How far the line hangs below its chord, blocks.
    pub fn sag(&self) -> f32 {
        self.sag
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame towards a sag of `slack * `[`MAX_SAG`], where `slack` is
    /// `0.0` when the line is taut and `1.0` when it hangs loose.
    pub fn update(&mut self, slack: f32, dt: f32) -> f32 {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let slack = if slack.is_finite() {
            slack.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let target = slack * MAX_SAG;
        self.velocity += (target - self.sag) * SAG_STIFFNESS * dt;
        self.velocity -= self.velocity * SAG_DAMPING * dt;
        self.sag += self.velocity * dt;
        self.sag = self.sag.clamp(0.0, MAX_SAG);
        self.sag
    }

    /// The point `t` of the way along the line from `anchor` to `bobber`, with
    /// the sag hanging lowest at the middle.
    pub fn point(&self, anchor: [f32; 3], bobber: [f32; 3], t: f32) -> [f32; 3] {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let hang = 4.0 * t * (1.0 - t) * self.sag;
        [
            anchor[0] + (bobber[0] - anchor[0]) * t,
            anchor[1] + (bobber[1] - anchor[1]) * t - hang,
            anchor[2] + (bobber[2] - anchor[2]) * t,
        ]
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
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    /// The signals of the last update.
    pub signals: MotionSignals,
    pub locomotion: LocomotionController,
    /// The gait of the last update, kept so a read costs no step.
    pub gait: LocomotionFrame,
    /// The gait before it, for a partial-tick read.
    previous_gait: LocomotionFrame,
    pub turn: TurnInPlaceController,
    pub items: ItemSequenceController,
    pub boat: BoatPaddleAnimation,
    pub fishing: FishingLinePhysics,
    /// The player's face: blinks, eyes and cues.
    pub face: FaceRig,
    /// Whether the player is rowing a boat.
    pub riding: bool,
    /// Whether a fishing line is out.
    pub fishing_line: bool,
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
            items: ItemSequenceController::new(),
            boat: BoatPaddleAnimation::new(),
            fishing: FishingLinePhysics::new(),
            face: FaceRig::default(),
            riding: false,
            fishing_line: false,
            pose: HumanoidPose::identity(),
            previous: HumanoidPose::identity(),
            clock: 0.0,
            blend: 0.0,
            retention: BasePoseRetention::new(),
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

    /// One update: advance every controller and rebuild the pose.
    fn advance(&mut self, signals: MotionSignals, dt: f32, tuning: &MovementTuning) {
        self.previous = self.pose.clone();
        self.signals = signals;
        self.clock += dt;
        self.face
            .update(&signals, &crate::faces::current_settings(), dt);
        let loco = self.locomotion.update(&signals, dt, tuning);
        self.previous_gait = self.gait;
        self.gait = loco;
        let turn = self.turn.update(signals.walk_left, signals.moving, dt);
        let seq = self.items.update(&signals, dt);
        let paddle = self.boat.update(self.riding && signals.moving, dt);
        self.fishing
            .update(if self.fishing_line { 0.15 } else { 1.0 }, dt);
        self.blend = approach(self.blend, 1.0, 8.0 * dt);
        let animated = build_pose(&signals, &loco, &turn, &seq, &paddle, tuning);
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
            roll: self.signals.swim_vertical * if self.signals.swimming() { 0.35 } else { 0.0 },
        }
    }
}

/// The pose the signals, gait, turn, hands and paddles add up to.
fn build_pose(
    signals: &MotionSignals,
    loco: &LocomotionFrame,
    turn: &TurnFrame,
    seq: &SequenceFrame,
    paddle: &PaddleFrame,
    tuning: &MovementTuning,
) -> HumanoidPose {
    let phase = loco.phase.to_radians();
    let (stride, speed) = (loco.stride.clamp(0.0, 1.0), loco.speed.clamp(0.0, 1.0));
    let swing = phase.cos() * LIMB_SWING * stride * tuning.gait_amplitude;
    let mut pose = HumanoidPose::identity();
    let glide = if signals.elytra { 1.0 } else { 0.0 };
    let swim = if signals.swimming() { 1.0 } else { 0.0 };
    let crouch = if signals.crouching { 1.0 } else { 0.0 };
    let climb = if signals.climbing { 1.0 } else { 0.0 };
    let busy = seq.weight.clamp(0.0, 1.0);

    // Legs: opposite swings, still while gliding or swimming, tucked while
    // climbing, and shifted by the turn-in-place step.
    let step = turn.step.clamp(0.0, 1.0) * 0.4;
    pose.right_leg.x_rot = swing;
    pose.left_leg.x_rot = -swing;
    pose.left_leg.y_rot = turn.body_offset.to_radians() * 0.1;
    pose.right_leg.y_rot = turn.body_offset.to_radians() * 0.1;
    pose.right_leg.z_rot = step;
    pose.left_leg.z_rot = -step;

    // Arms: the legs' opposite swing on foot; back and out gliding; up
    // climbing; forward and up while a hand action plays; stroking in a boat.
    let arm_swing = -swing * 1.05;
    pose.right_arm.x_rot = arm_swing;
    pose.left_arm.x_rot = -arm_swing;
    let charge = if seq.action == ItemAction::Charge {
        1.4 * busy
    } else {
        0.0
    };
    let use_arm = if seq.action == ItemAction::Use || seq.action == ItemAction::Eat {
        -0.9 * busy
    } else {
        0.0
    };
    let swing_arm = if seq.action == ItemAction::Swing {
        (seq.progress * std::f32::consts::PI).sin() * 1.2 * busy
    } else {
        0.0
    };
    pose.right_arm.x_rot += -swing_arm - charge - use_arm - paddle.right * 1.1;
    pose.left_arm.x_rot += -paddle.left * 1.1;
    pose.right_arm.z_rot = -(0.09 + 0.35 * glide + 0.25 * swim) - 0.35 * climb;
    pose.left_arm.z_rot = 0.09 + 0.35 * glide + 0.25 * swim + 0.35 * climb;

    // Body: crouches, dips on a landing, leans into a run and while gliding,
    // and rises a touch on the paddle stroke.
    pose.body.y = -0.06 * crouch - 0.10 * loco.lift + 0.02 * paddle.amplitude;
    pose.body.x_rot = speed * 0.10 + 0.45 * glide - 0.18 * climb;
    pose.body.y_rot = turn.body_offset.to_radians() * 0.35;

    // Head: takes most of the turn, looks along the climb, and banks with the
    // glide.
    pose.head.y_rot = turn.body_offset.to_radians() * 0.65;
    pose.head.x_rot = -signals.climb_direction.clamp(-1.0, 1.0) * 0.4 * climb - 0.20 * glide;
    pose.head.z_rot = signals.elytra_bank * 0.35 * glide;

    // Cape: streams behind with speed and glide, lifts when falling.
    pose.cape.x_rot = -(speed * 0.25 + 0.55 * glide) - 0.30 * loco.lift;
    pose.cape.z_rot = swing * 0.15;

    pose.blend = 1.0;
    pose
}

/// Every player's animation state, keyed by entity id, with the tuning they
/// share.
#[derive(Debug, Clone, Default)]
pub struct PlayerAnimationEngine {
    players: HashMap<u64, PlayerState>,
    tuning: MovementTuning,
}

impl PlayerAnimationEngine {
    /// An engine with the given tuning.
    pub fn new(tuning: MovementTuning) -> Self {
        Self {
            players: HashMap::new(),
            tuning,
        }
    }

    /// The tuning every player reads.
    pub fn tuning(&self) -> &MovementTuning {
        &self.tuning
    }

    /// Replaces the tuning; the next update uses it.
    pub fn set_tuning(&mut self, tuning: MovementTuning) {
        self.tuning = tuning;
    }

    /// One update for `entity`; the state is created on its first update.
    pub fn update(&mut self, entity: u64, signals: MotionSignals, dt: f32) -> &PlayerState {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.25)
        } else {
            0.0
        };
        let tuning = self.tuning;
        let state = self.players.entry(entity).or_default();
        state.advance(signals, dt, &tuning);
        state
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
                self.age.max(0.0) as u64
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
        glue.engine.update(entity, signals, dt);
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

    fn walking() -> MotionSignals {
        MotionSignals {
            moving: true,
            walk_forward: 1.0,
            grounded: true,
            ..MotionSignals::new()
        }
    }

    fn stepping(engine: &mut PlayerAnimationEngine, signals: MotionSignals, frames: usize) {
        for _ in 0..frames {
            engine.update(1, signals, 0.05);
        }
    }

    #[test]
    fn a_part_transform_lerps_and_scales() {
        let a = PartTransform::IDENTITY;
        let b = PartTransform {
            x_rot: 1.0,
            y_rot: 2.0,
            z_rot: 3.0,
            x: 0.5,
            y: 0.25,
            z: -1.0,
        };
        assert!(a.is_identity());
        assert!(!b.is_identity());

        let half = a.lerp(b, 0.5);
        assert_eq!(half.x_rot, 0.5);
        assert_eq!(half.y_rot, 1.0);
        assert_eq!(half.x, 0.25);
        // Clamped ends.
        assert_eq!(a.lerp(b, 2.0), b);
        assert_eq!(a.lerp(b, -1.0), a);
        // A broken `t` reads as no blend.
        assert_eq!(a.lerp(b, f32::NAN), a);

        let doubled = b.scaled(2.0);
        assert_eq!(doubled.x_rot, 2.0);
        assert_eq!(doubled.z, -2.0);
        assert!(b.scaled(0.0).is_identity());
    }

    #[test]
    fn a_pose_names_its_seven_parts() {
        let mut pose = HumanoidPose::identity();
        assert_eq!(pose.parts().len(), PART_NAMES.len());
        for name in PART_NAMES {
            assert!(pose.part(name).is_some(), "{} is not a part", name);
            assert!(pose.part_mut(name).is_some());
        }
        assert!(pose.part("tail").is_none());
        assert!(pose.part_mut("tail").is_none());

        pose.head.x_rot = 1.0;
        assert_eq!(pose.part("head").unwrap().x_rot, 1.0);
        assert_eq!(pose.part("body").unwrap(), PartTransform::IDENTITY);
        // Parts come out in the documented order.
        let names: Vec<&str> = pose.parts().into_iter().map(|(name, _)| name).collect();
        assert_eq!(names, PART_NAMES);
    }

    #[test]
    fn over_mixes_the_animation_with_the_base_and_the_retention() {
        let mut animated = HumanoidPose::identity();
        animated.head.x_rot = 1.0;
        animated.right_arm.z_rot = -0.5;
        animated.blend = 0.5;

        // No retention: half of each part shows.
        let mixed = animated.over(&HumanoidPose::identity());
        assert!((mixed.head.x_rot - 0.5).abs() < 1e-6);
        assert!((mixed.right_arm.z_rot + 0.25).abs() < 1e-6);

        // Retaining a bone fully keeps the base there and lets the others move.
        animated.retention = BasePoseRetention::from_bones(&["head"], 1.0);
        let mixed = animated.over(&HumanoidPose::identity());
        assert_eq!(mixed.head.x_rot, 0.0);
        assert!((mixed.right_arm.z_rot + 0.25).abs() < 1e-6);

        // A blend of zero leaves the base exactly as it was.
        let mut still = HumanoidPose::identity();
        still.head.x_rot = 1.0;
        still.blend = 0.0;
        let mut base = HumanoidPose::identity();
        base.head.y_rot = 0.5;
        assert_eq!(still.over(&base).head.y_rot, 0.5);
    }

    #[test]
    fn the_gait_eases_in_with_the_input_and_walks_by_distance() {
        let tuning = MovementTuning::default();
        let mut locomotion = LocomotionController::new();
        let frame = locomotion.update(&walking(), 0.05, &tuning);
        // A full walk: the stride eases in at the tuning's fade rate.
        assert!((frame.stride - 0.3).abs() < 1e-6);
        assert_eq!(frame.speed, 1.0);
        let phase = frame.phase;
        assert!((phase - 38.853).abs() < 0.01, "phase was {}", phase);

        // The cycle follows the blocks travelled: twice the distance, twice
        // the turn.
        let mut faster = LocomotionController::new();
        let a = faster.update(
            &MotionSignals {
                moving: true,
                walk_forward: 1.0,
                ..MotionSignals::new()
            },
            0.1,
            &tuning,
        );
        assert!((a.phase - 2.0 * phase).abs() < 0.02);

        // Standing still lets the stride out and stops the cycle.
        let mut idle = LocomotionController::new();
        idle.update(&walking(), 0.5, &tuning);
        let still = idle.update(&MotionSignals::new(), 0.05, &tuning);
        assert!(still.stride < 1.0);
        assert_eq!(still.speed, 0.0);
        let before = still.phase;
        let again = idle.update(&MotionSignals::new(), 0.5, &tuning);
        assert_eq!(again.phase, before);
        assert!(again.stride < still.stride);
    }

    #[test]
    fn running_is_counted_and_reset() {
        let tuning = MovementTuning::default();
        let mut locomotion = LocomotionController::new();
        let sprinting = MotionSignals {
            running: true,
            ..walking()
        };
        // The fall weight eases in behind the signal.
        let mut falling = sprinting;
        falling.fall_weight = 1.0;
        locomotion.update(&falling, 0.1, &tuning);
        let frame = locomotion.update(&falling, 0.1, &tuning);
        assert!((frame.run_seconds - 0.2).abs() < 1e-6);
        assert!(frame.lift > 0.0 && frame.lift <= 1.0);

        // Walking without sprinting resets the count.
        let frame = locomotion.update(&walking(), 0.1, &tuning);
        assert_eq!(frame.run_seconds, 0.0);

        // `reset` forgets everything.
        locomotion.reset();
        assert_eq!(locomotion.stride(), 0.0);
        assert_eq!(locomotion.phase(), 0.0);
    }

    #[test]
    fn turning_on_the_spot_twists_then_takes_a_step() {
        let mut turn = TurnInPlaceController::new();
        let frame = turn.update(1.0, false, 0.1);
        assert!((frame.body_offset - 18.0).abs() < 1e-6);
        assert!(!frame.turning);
        assert_eq!(frame.step, 0.0);

        // Past the step threshold the feet follow.
        let frame = turn.update(1.0, false, 0.1);
        assert!((frame.body_offset - 36.0).abs() < 1e-6);
        assert!(frame.turning);
        assert!((frame.step - 0.3).abs() < 1e-6);

        // The twist never passes its limit.
        for _ in 0..10 {
            turn.update(1.0, false, 0.1);
        }
        assert!((turn.body_offset() - TURN_LIMIT_DEGREES).abs() < 1e-6);
        assert_eq!(turn.step(), 1.0);

        // Strafing right twists the other way.
        turn.update(-1.0, false, 0.25);
        let frame = turn.update(-1.0, false, 0.25);
        assert!(frame.body_offset < 0.0);
    }

    #[test]
    fn walking_lets_the_twist_out_and_the_step_go() {
        let mut turn = TurnInPlaceController::new();
        turn.update(1.0, false, 0.1);
        turn.update(1.0, false, 0.1);
        assert!(turn.step() > 0.0);

        let frame = turn.update(1.0, true, 0.1);
        assert!(frame.body_offset < 36.0);
        assert!(!frame.turning);

        // Enough time walking and the torso is straight again with no step.
        for _ in 0..10 {
            turn.update(1.0, true, 0.25);
        }
        assert_eq!(turn.body_offset(), 0.0);
        assert_eq!(turn.step(), 0.0);
    }

    #[test]
    fn item_actions_follow_the_signals_and_end() {
        // The signals pick one action, charges first.
        assert_eq!(
            ItemAction::from_signals(&MotionSignals {
                mining: true,
                charging_trident: true,
                ..MotionSignals::new()
            }),
            ItemAction::Charge
        );
        assert_eq!(
            ItemAction::from_signals(&MotionSignals {
                mining: true,
                map_view: true,
                ..MotionSignals::new()
            }),
            ItemAction::Swing
        );
        assert_eq!(
            ItemAction::from_signals(&MotionSignals {
                map_view: true,
                ..MotionSignals::new()
            }),
            ItemAction::Use
        );
        assert_eq!(
            ItemAction::from_signals(&MotionSignals::new()),
            ItemAction::None
        );

        let mut items = ItemSequenceController::new();
        let mining = MotionSignals {
            mining: true,
            ..MotionSignals::new()
        };
        let frame = items.update(&mining, 0.05);
        assert_eq!(frame.action, ItemAction::Swing);
        assert!((frame.weight - 0.4).abs() < 1e-6);
        assert!((frame.progress - 0.05 / 0.35).abs() < 1e-6);

        // A swing is one pass: it ends and the pose eases out. A frame is
        // never longer than a quarter second, so the pass takes two.
        items.update(&mining, 0.25);
        let frame = items.update(&mining, 0.25);
        assert_eq!(frame.action, ItemAction::None);
        assert_eq!(frame.progress, 0.0);
        assert!(frame.weight < 1.0);

        // Letting go stops at once.
        items.play(ItemAction::Use);
        assert_eq!(items.action(), ItemAction::Use);
        items.cancel();
        assert_eq!(items.action(), ItemAction::None);
        assert_eq!(items.weight(), 0.0);
    }

    #[test]
    fn a_held_charge_keeps_pulsing() {
        let mut items = ItemSequenceController::new();
        let charging = MotionSignals {
            charging_crossbow: true,
            ..MotionSignals::new()
        };
        let first = items.update(&charging, 0.1);
        assert_eq!(first.action, ItemAction::Charge);
        // Held: the pass wraps rather than ending.
        let later = items.update(&charging, 1.2);
        assert_eq!(later.action, ItemAction::Charge);
        assert!(later.progress < 1.0);
        // The held action survives its own duration.
        assert!(ItemAction::Charge.held());
        assert!(!ItemAction::Swing.held());
    }

    #[test]
    fn paddles_alternate_and_fade() {
        let mut boat = BoatPaddleAnimation::new();
        let frame = boat.update(true, 0.1);
        assert!((frame.amplitude - 0.4).abs() < 1e-6);
        assert!((frame.left + frame.right).abs() < 1e-9);
        assert!((frame.phase - PADDLE_RATE * 0.1).abs() < 1e-6);

        // Rowing on, the arms cross and swap sides.
        let mut crossed = false;
        for _ in 0..40 {
            let frame = boat.update(true, 0.05);
            if frame.left * frame.right < 0.0 && frame.left > 0.0 {
                crossed = true;
            }
        }
        assert!(crossed, "the paddles never swapped");

        // Stopped, the paddling goes and the phase stays put.
        let frame = boat.update(false, 0.25);
        assert_eq!(frame.amplitude, 0.0);
        assert_eq!(frame.left, 0.0);
        let phase = frame.phase;
        let frame = boat.update(false, 0.25);
        assert_eq!(frame.phase, phase);
        assert_eq!(boat.phase(), phase);

        boat.reset();
        assert_eq!(boat.amplitude(), 0.0);
    }

    #[test]
    fn the_fishing_line_sags_and_settles() {
        let mut line = FishingLinePhysics::new();
        let mut previous = 0.0;
        for _ in 0..60 {
            let sag = line.update(1.0, 0.05);
            assert!(sag >= previous, "the sag fell: {} after {}", sag, previous);
            assert!(sag <= MAX_SAG);
            previous = sag;
        }
        assert!((line.sag() - MAX_SAG).abs() < 0.02);

        // Reeled taut, the line comes back up.
        for _ in 0..120 {
            line.update(0.0, 0.05);
        }
        assert!(line.sag() < 0.01);

        // The curve hangs lowest in the middle and meets both ends.
        let mut line = FishingLinePhysics::new();
        for _ in 0..60 {
            line.update(1.0, 0.05);
        }
        let anchor = [0.0, 1.0, 0.0];
        let bobber = [0.0, 0.0, 4.0];
        assert_eq!(line.point(anchor, bobber, 0.0), anchor);
        assert_eq!(line.point(anchor, bobber, 1.0), bobber);
        let middle = line.point(anchor, bobber, 0.5);
        assert!((middle[1] - (0.5 - line.sag())).abs() < 1e-6);
        assert_eq!(middle[2], 2.0);
        // A broken tick reads as the anchor end.
        assert_eq!(line.point(anchor, bobber, f32::NAN), anchor);
        // A wild slack clamps.
        line.update(9.0, 1.0);
        assert!(line.sag() <= MAX_SAG);
    }

    #[test]
    fn the_engine_builds_a_pose_per_player() {
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        assert!(engine.is_empty());

        engine.update(7, walking(), 0.05);
        assert_eq!(engine.len(), 1);
        assert!(engine.contains(7));
        assert_eq!(engine.state(7).unwrap().clock(), 0.05);
        assert!(engine.state(7).unwrap().blend() > 0.0);

        // The legs swing opposite ways and the body leans into the walk.
        let pose = engine.sample_pose(7, 1.0);
        let right = pose.right_leg.x_rot;
        let left = pose.left_leg.x_rot;
        assert!(right.abs() > 0.0);
        assert!((right + left).abs() < 1e-6);
        assert!(pose.body.x_rot > 0.0);
        assert!(pose.cape.x_rot < 0.0);

        // An entity the engine never saw draws the identity pose.
        assert_eq!(engine.sample_pose(99, 1.0), HumanoidPose::identity());
        assert!(engine.root_motion(99, 1.0).is_none());

        assert!(engine.remove(7));
        assert!(!engine.remove(7));
        engine.update(8, walking(), 0.05);
        engine.clear();
        assert!(engine.is_empty());
    }

    #[test]
    fn a_partial_tick_blends_the_last_two_poses() {
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        engine.update(1, walking(), 0.05);
        let first = engine.state(1).unwrap().pose().clone();
        engine.update(1, walking(), 0.05);
        let second = engine.state(1).unwrap().pose().clone();
        assert_ne!(first, second);

        // Zero ticks past the last update is the pose before it.
        assert_eq!(engine.sample_pose(1, 0.0), first);
        // One is the newest.
        assert_eq!(engine.sample_pose(1, 1.0), second);
        // In between is in between.
        let half = engine.sample_pose(1, 0.5);
        let swing = first.right_leg.x_rot + (second.right_leg.x_rot - first.right_leg.x_rot) * 0.5;
        assert!((half.right_leg.x_rot - swing).abs() < 1e-6);
        // A broken partial reads as the newest pose.
        assert_eq!(engine.sample_pose(1, f32::NAN), second);
    }

    #[test]
    fn retention_keeps_the_base_of_the_bones_it_names() {
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        engine.set_retention(1, BasePoseRetention::from_bones(&["head"], 1.0));
        for _ in 0..5 {
            engine.update(1, walking(), 0.05);
        }
        let pose = engine.sample_pose(1, 1.0);
        // The head keeps its base (nothing here), the legs still swing.
        assert_eq!(pose.head.x_rot, 0.0);
        assert_eq!(pose.head.y_rot, 0.0);
        assert!(pose.right_leg.x_rot.abs() > 0.0);
    }

    #[test]
    fn root_motion_follows_walking_and_gliding() {
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        stepping(&mut engine, walking(), 10);
        let walk = engine.root_motion(1, 1.0).unwrap();
        assert!(walk.lean > 0.0);
        assert!(walk.bank.abs() < 1e-9);

        // Gliding banks and rolls, and leans further.
        let gliding = MotionSignals {
            moving: true,
            walk_forward: 1.0,
            grounded: false,
            elytra: true,
            elytra_bank: 0.5,
            ..MotionSignals::new()
        };
        stepping(&mut engine, gliding, 10);
        let glide = engine.root_motion(1, 1.0).unwrap();
        assert!((glide.bank - 0.5).abs() < 1e-6);
        assert!(glide.lean > walk.lean);

        // A landing squashes the model.
        let landing = MotionSignals {
            grounded: true,
            fall_weight: 1.0,
            ..MotionSignals::new()
        };
        stepping(&mut engine, landing, 10);
        let landed = engine.root_motion(1, 1.0).unwrap();
        assert!(landed.squash > 0.0);
    }

    #[test]
    fn gliding_spreads_the_arms_and_crouching_drops_the_body() {
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        let gliding = MotionSignals {
            moving: true,
            walk_forward: 1.0,
            grounded: false,
            elytra: true,
            ..MotionSignals::new()
        };
        stepping(&mut engine, gliding, 10);
        let pose = engine.sample_pose(1, 1.0);
        assert!(pose.right_arm.z_rot < 0.0);
        assert!(pose.left_arm.z_rot > 0.0);
        assert!((pose.right_arm.z_rot + pose.left_arm.z_rot).abs() < 1e-6);

        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        let crouching = MotionSignals {
            grounded: true,
            crouching: true,
            ..MotionSignals::new()
        };
        stepping(&mut engine, crouching, 10);
        let pose = engine.sample_pose(1, 1.0);
        assert!(pose.body.y < 0.0);
    }

    #[test]
    fn the_frame_glue_poses_the_entity_the_models_asked_for() {
        reset();
        let dt = begin_frame();
        assert!(dt > 0.0 && dt <= 1.0 / 20.0);

        begin_pose(
            42,
            PoseInput {
                age: 200.0,
                speed: 1.0,
                position: 3.0,
                partial: 1.0,
            },
        );
        let pose = finish_pose().expect("the posed entity was known");
        assert!(pose.blend > 0.0);
        assert!(pose.right_leg.x_rot.abs() > 0.0);
        // The pose can only be finished once.
        assert_eq!(finish_pose(), None);

        assert!(sampled_root_motion(42, 1.0).is_some());
        assert!(with_engine(|engine| engine.contains(42)));

        // A pose state with no speed stands still.
        begin_pose(
            42,
            PoseInput {
                age: 0.0,
                speed: 0.0,
                position: 0.0,
                partial: 0.5,
            },
        );
        assert!(finish_pose().is_some());

        reset();
        assert!(with_engine(|engine| engine.is_empty()));
        assert_eq!(finish_pose(), None);
        assert!(sampled_root_motion(42, 1.0).is_none());
    }

    #[test]
    fn the_engine_keeps_a_face_per_player() {
        use crate::faces::{FaceExpression, FaceSettings};

        crate::faces::reset_settings();
        let mut engine = PlayerAnimationEngine::new(MovementTuning::default());
        engine.update(3, walking(), 0.05);

        // A calm walk wears the resting face.
        let state = engine.state(3).unwrap();
        assert_eq!(state.face.expression(), FaceExpression::Idle);
        let quads = engine.face_quads(3).expect("a face for a known player");
        assert!(quads.len() >= 5, "{} quads", quads.len());
        assert!(engine.face_quads(99).is_none());

        // The settings the hooks keep reach the quads: off draws nothing.
        crate::faces::set_settings(FaceSettings {
            enabled: false,
            ..FaceSettings::default()
        });
        engine.update(3, walking(), 0.05);
        assert!(engine.face_quads(3).unwrap().is_empty());
        crate::faces::reset_settings();

        // The head transform the overlay draws on is the sampled pose's own.
        assert_eq!(
            engine.state(3).unwrap().pose().head,
            engine.sample_pose(3, 1.0).head
        );
    }

    #[test]
    fn a_pose_input_reads_a_walk_from_its_speed() {
        let signals = PoseInput {
            age: 12.9,
            speed: 0.5,
            position: 1.0,
            partial: 0.5,
        }
        .signals();
        assert_eq!(signals.tick, 12);
        assert_eq!(signals.walk_forward, 0.5);
        assert!(signals.moving);
        assert!(!signals.running);
        assert!(signals.grounded);

        let standing = PoseInput::default().signals();
        assert!(!standing.moving);
        // A broken speed is a standstill, not a NaN in the pose.
        let broken = PoseInput {
            speed: f32::NAN,
            age: f32::NAN,
            ..PoseInput::default()
        }
        .signals();
        assert_eq!(broken.walk_forward, 0.0);
        assert_eq!(broken.tick, 0);
        assert!(!broken.moving);
    }
}

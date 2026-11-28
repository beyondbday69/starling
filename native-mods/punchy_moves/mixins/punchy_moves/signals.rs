//! Motion signals, gait phase matching, base-pose retention and movement
//! tuning, ported 1:1 from the Fabric mod's `punchymoves.client.animation`
//! (`MotionSignals`, `GaitPhaseMatcher`, `BasePoseRetention`,
//! `MovementTuning`).
//!
//! Every constant, field, formula and error message here is the Java class's,
//! including the Portuguese validation strings the compat loader raises.
//! Rust shapes differ only where the language forces it: the Java statics are
//! free functions, the records are structs, and the JSON tree is the mod's own
//! `json` module instead of Gson.

use std::collections::HashMap;

use crate::json::Value;
use crate::library::Vec3;

// ---------------------------------------------------------------------------
// MotionSignals
// ---------------------------------------------------------------------------

/// `MotionSignals.NEVER`: no event has happened.
pub const NEVER: i32 = i32::MIN;
/// `MotionSignals.LIGHT`: the lightest fall or landing weight.
pub const LIGHT: i32 = 1;
/// `MotionSignals.MEDIUM`: the middle fall or landing weight.
pub const MEDIUM: i32 = 2;
/// `MotionSignals.HEAVY`: the heaviest fall or landing weight.
pub const HEAVY: i32 = 3;
/// `MotionSignals.FALL`: an ordinary fall.
pub const FALL: i32 = 1;
/// `MotionSignals.FREE_FALL`: a fall with no ground in sight.
pub const FREE_FALL: i32 = 2;
/// `MotionSignals.HARD_FALL`: a fall fast enough to hurt.
pub const HARD_FALL: i32 = 3;
/// `MotionSignals.TOSS`: a light throw.
pub const TOSS: i32 = 1;
/// `MotionSignals.PEARL`: an ender pearl throw.
pub const PEARL: i32 = 2;
/// `MotionSignals.BREW`: a potion throw.
pub const BREW: i32 = 3;

/// One tick's worth of the player's movement state: what they touch, how they
/// move, which discrete action they are in, and the tick each one-shot event
/// last happened on.
///
/// The event ticks (`jumped`, `landed`, `hurt`, ...) hold [`NEVER`] until the
/// event happens, then the tick it happened on; [`MotionSignals::since`]
/// turns one into an age in ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSignals {
    /// The tick this snapshot was taken on, [`NEVER`] before the first.
    pub tick: i32,
    /// Standing on something solid.
    pub grounded: bool,
    /// The fall's weight: [`LIGHT`], [`MEDIUM`] or [`HEAVY`].
    pub fall_weight: i32,
    /// Blocks dropped since the player last touched the ground.
    pub fall_distance: f32,
    /// Water (or flowing water) is directly below.
    pub water_below: bool,
    /// Lava is directly below.
    pub lava_below: bool,
    /// Swimming with the head above the surface.
    pub surface_swimming: bool,
    /// Swimming with the whole body under the surface.
    pub submerged_swimming: bool,
    /// Sprint-swimming: the sprint key is held in water.
    pub swim_sprint: bool,
    /// Vertical swim input: `-1` down, `1` up.
    pub swim_vertical: i32,
    /// Standing in water shallow enough to wade through.
    pub shallow_water: bool,
    /// Holding a ladder, vine or scaffolding.
    pub climbing: bool,
    /// Vertical climb input: `-1` down, `1` up.
    pub climb_direction: i32,
    /// Crawling under a one-block gap.
    pub crawling: bool,
    /// Gliding with an elytra.
    pub elytra: bool,
    /// Air speed while gliding, in blocks per tick.
    pub elytra_speed: f32,
    /// Roll while gliding, in radians: positive banks right.
    pub elytra_bank: f32,
    /// Flying in creative mode.
    pub creative_flight: bool,
    /// Flying faster than creative flight (the sprint key in the air).
    pub fast_flight: bool,
    /// Standing on a fence, wall or similar narrow-topped block.
    pub on_fence: bool,
    /// The sprint key is held and the player is moving.
    pub running: bool,
    /// Seconds the player has been running.
    pub run_seconds: f32,
    /// The sneak key is held.
    pub crouching: bool,
    /// The player's position changed since the previous tick.
    pub moving: bool,
    /// Forward movement input, `-1.0` (back) .. `1.0` (forward).
    pub walk_forward: f32,
    /// Left movement input, `-1.0` (right) .. `1.0` (left).
    pub walk_left: f32,
    /// Breaking a block.
    pub mining: bool,
    /// Drawing a crossbow.
    pub charging_crossbow: bool,
    /// Winding up a spear throw.
    pub charging_spear: bool,
    /// Holding a trident ready to throw.
    pub charging_trident: bool,
    /// Looking at a held map.
    pub map_view: bool,
    /// Music is playing within earshot.
    pub music_nearby: bool,
    /// Seconds since the player last moved.
    pub still_seconds: f32,
    /// The player is taking freezing damage.
    pub freezing: bool,
    /// A fishing line is out.
    pub fishing: bool,
    /// Something is pulling on the fishing line.
    pub biting: bool,
    /// The tick the player last jumped.
    pub jumped: i32,
    /// Whether that jump began while moving.
    pub jump_moving: bool,
    /// The tick the player last landed.
    pub landed: i32,
    /// The landing's weight: [`LIGHT`], [`MEDIUM`] or [`HEAVY`].
    pub landing_weight: i32,
    /// The tick the run began.
    pub run_started: i32,
    /// The tick the run ended.
    pub run_stopped: i32,
    /// The tick the player last crouched.
    pub crouched: i32,
    /// The tick the player last stood up.
    pub stood_up: i32,
    /// The tick the player last splashed.
    pub splashed: i32,
    /// The tick the player was last boosted (a firework, a riptide trident).
    pub boosted: i32,
    /// The tick the player last swapped hands.
    pub swapped: i32,
    /// The tick the player last equipped armour.
    pub equipped: i32,
    /// The tick the player last interacted with something.
    pub interacted: i32,
    /// The tick the player last threw an item.
    pub threw: i32,
    /// The weight of that throw: [`TOSS`], [`PEARL`] or [`BREW`].
    pub thrown: i32,
    /// The tick the player last ignited something.
    pub ignited: i32,
    /// The tick the player last sheared something.
    pub sheared: i32,
    /// The tick the player last released a draw.
    pub released: i32,
    /// The tick the player last hurled an item.
    pub hurled: i32,
    /// The tick the player last lunged.
    pub lunged: i32,
    /// The tick the player last cast a fishing line.
    pub cast: i32,
    /// The tick the player last reeled the line in.
    pub reeled: i32,
    /// The tick the player was last hurt.
    pub hurt: i32,
    /// Whether that hurt came from the front.
    pub hurt_from_front: bool,
    /// The tick the last idle event played.
    pub idle_event: i32,
}

impl Default for MotionSignals {
    fn default() -> Self {
        Self {
            tick: NEVER,
            grounded: true,
            fall_weight: 0,
            fall_distance: 0.0,
            water_below: false,
            lava_below: false,
            surface_swimming: false,
            submerged_swimming: false,
            swim_sprint: false,
            swim_vertical: 0,
            shallow_water: false,
            climbing: false,
            climb_direction: 0,
            crawling: false,
            elytra: false,
            elytra_speed: 0.0,
            elytra_bank: 0.0,
            creative_flight: false,
            fast_flight: false,
            on_fence: false,
            running: false,
            run_seconds: 0.0,
            crouching: false,
            moving: false,
            walk_forward: 0.0,
            walk_left: 0.0,
            mining: false,
            charging_crossbow: false,
            charging_spear: false,
            charging_trident: false,
            map_view: false,
            music_nearby: false,
            still_seconds: 0.0,
            freezing: false,
            fishing: false,
            biting: false,
            jumped: NEVER,
            jump_moving: false,
            landed: NEVER,
            landing_weight: 0,
            run_started: NEVER,
            run_stopped: NEVER,
            crouched: NEVER,
            stood_up: NEVER,
            splashed: NEVER,
            boosted: NEVER,
            swapped: NEVER,
            equipped: NEVER,
            interacted: NEVER,
            threw: NEVER,
            thrown: 0,
            ignited: NEVER,
            sheared: NEVER,
            released: NEVER,
            hurled: NEVER,
            lunged: NEVER,
            cast: NEVER,
            reeled: NEVER,
            hurt: NEVER,
            hurt_from_front: true,
            idle_event: NEVER,
        }
    }
}

impl MotionSignals {
    /// A snapshot as a fresh tick begins: every per-tick field cleared, the
    /// event ticks left as they were.
    pub fn new() -> Self {
        Self::default()
    }

    /// `MotionSignals.since`: the age of `event` at `tick`, in ticks, or
    /// [`i32::MAX`] when the event never happened or is in the future.
    pub fn since(event: i32, tick: i32) -> i32 {
        if event != NEVER && tick >= event {
            tick - event
        } else {
            i32::MAX
        }
    }

    /// `MotionSignals.fresh`: whether `tick` is this snapshot's tick or the
    /// one right after it.
    pub fn fresh(&self, tick: i32) -> bool {
        self.tick != NEVER && tick >= self.tick && tick - self.tick <= 1
    }

    /// `MotionSignals.begin`: clears every field a new tick rebuilds, keeping
    /// the event ticks.
    pub fn begin(&mut self, tick: i32) {
        self.tick = tick;
        self.elytra = false;
        self.elytra_speed = 0.0;
        self.elytra_bank = 0.0;
        self.creative_flight = false;
        self.fast_flight = false;
        self.climbing = false;
        self.crawling = false;
        self.on_fence = false;
        self.running = false;
        self.run_seconds = 0.0;
        self.moving = false;
        self.walk_forward = 0.0;
        self.walk_left = 0.0;
        self.music_nearby = false;
        self.still_seconds = 0.0;
        self.freezing = false;
        self.fall_weight = 0;
        self.fall_distance = 0.0;
        self.water_below = false;
        self.lava_below = false;
        self.surface_swimming = false;
        self.submerged_swimming = false;
        self.swim_sprint = false;
        self.swim_vertical = 0;
        self.shallow_water = false;
        self.climb_direction = 0;
        self.mining = false;
        self.charging_crossbow = false;
        self.charging_spear = false;
        self.charging_trident = false;
        self.map_view = false;
    }

    /// `MotionSignals.landingWeight`: the weight a landing gets from two
    /// measures (a drop and a speed, say), each with its own medium and hard
    /// threshold: hard when either measure reaches its hard threshold, medium
    /// when either reaches its medium one, light otherwise.
    pub fn landing_weight(
        first: f32,
        second: f32,
        medium_first: f32,
        medium_second: f32,
        hard_first: f32,
        hard_second: f32,
    ) -> i32 {
        if first >= hard_first || second >= hard_second {
            HEAVY
        } else if first >= medium_first || second >= medium_second {
            MEDIUM
        } else {
            LIGHT
        }
    }

    /// `MotionSignals.fallWeight`: the same three-way measure, for a fall in
    /// progress.
    pub fn fall_weight(
        first: f32,
        second: f32,
        medium_first: f32,
        medium_second: f32,
        hard_first: f32,
        hard_second: f32,
    ) -> i32 {
        if first >= hard_first || second >= hard_second {
            HEAVY
        } else if first >= medium_first || second >= medium_second {
            MEDIUM
        } else {
            LIGHT
        }
    }
}

// ---------------------------------------------------------------------------
// GaitPhaseMatcher
// ---------------------------------------------------------------------------

/// `GaitPhaseMatcher`: the stride phase the gait settles on, matched by
/// walking a cost function round the cycle.
///
/// The phase is a fraction of the walk cycle, `0.0..1.0`. [`match_phase`]
/// scans the whole cycle in 64 steps, keeps the cheapest, then refines
/// ±8/512 of a cycle either side of that; a step is only kept when its cost
/// drops by more than `1e-4`, so the match never wanders on a flat cost.
pub mod gait_phase_matcher {
    /// The cost difference a step must beat to be kept.
    const IMPROVEMENT: f64 = 1.0e-4;
    /// The coarse pass's steps round the whole cycle.
    const COARSE_STEPS: i32 = 64;
    /// The fine pass's steps either side of the coarse best.
    const FINE_STEPS: i32 = 8;
    /// The fine pass's divisor.
    const FINE_DIVISOR: f32 = 512.0;

    /// `GaitPhaseMatcher.wrap`: the phase wrapped into `0.0..1.0`
    /// (`x - floor(x)`).
    pub fn wrap(phase: f32) -> f32 {
        phase - phase.floor()
    }

    /// `GaitPhaseMatcher.match`: the phase in `0.0..1.0` whose `cost` is
    /// cheapest, starting from `phase`. A cost that is not finite anywhere
    /// returns the phase unchanged.
    pub fn match_phase<F>(phase: f32, cost: F) -> f32
    where
        F: Fn(f32) -> f64,
    {
        let mut best = phase;
        let Some(mut best_cost) = finite(cost(phase)) else {
            return phase;
        };
        for i in 1..COARSE_STEPS {
            let candidate = wrap(phase + i as f32 / COARSE_STEPS as f32);
            if let Some(value) = finite(cost(candidate))
                && value < best_cost - IMPROVEMENT
            {
                best = candidate;
                best_cost = value;
            }
        }
        let coarse = best;
        for j in -FINE_STEPS..=FINE_STEPS {
            let candidate = wrap(coarse + j as f32 / FINE_DIVISOR);
            if let Some(value) = finite(cost(candidate))
                && value < best_cost - IMPROVEMENT
            {
                best = candidate;
                best_cost = value;
            }
        }
        best
    }

    /// The cost when it is finite.
    fn finite(cost: f64) -> Option<f64> {
        cost.is_finite().then_some(cost)
    }
}

pub use gait_phase_matcher::match_phase;

// ---------------------------------------------------------------------------
// BasePoseRetention
// ---------------------------------------------------------------------------

/// `BasePoseRetention`: how much of the base (vanilla) pose each bone keeps,
/// `0.0..=1.0` per bone.
///
/// A bone the set does not mention keeps nothing ([`BasePoseRetention::value`]
/// answers `NaN`, which the animation layer reads as "the animation owns the
/// bone"), except that an upper-body bone falls back to the `waist` value when
/// there is one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BasePoseRetention {
    bones: HashMap<String, f32>,
}

impl BasePoseRetention {
    /// `BasePoseRetention.EMPTY`.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets one bone's weight (the tests and the additive retention build
    /// their sets this way).
    pub fn set(&mut self, bone: &str, weight: f32) {
        self.bones.insert(bone.to_string(), weight);
    }

    /// `BasePoseRetention.BONES`: every bone the mod can retain.
    pub const BONES: [&str; 8] = [
        "root",
        "waist",
        "head",
        "body",
        "left_arm",
        "right_arm",
        "left_leg",
        "right_leg",
    ];

    /// `BasePoseRetention.UPPER_BODY`: the bones a waist value stands in for.
    pub const UPPER_BODY: [&str; 4] = ["body", "head", "left_arm", "right_arm"];

    /// Whether the set mentions no bone at all.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    /// `BasePoseRetention.parse`: the weights from a JSON object of bone
    /// names to numbers in `0.0..=1.0`. `None` is the empty set; anything
    /// else that is not an object, an unknown bone or a weight out of range
    /// is an error carrying the Java message (`Compat <context>...`).
    pub fn parse(value: Option<&Value>, context: &str) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::empty());
        };
        let Some(members) = value.as_object() else {
            return Err(invalid(context, "esperado um objeto por osso"));
        };
        let mut bones = HashMap::new();
        for (bone, weight) in members {
            let label = format!("{}.{}", context, bone);
            if !Self::BONES.contains(&bone.as_str()) {
                return Err(invalid(&label, "osso desconhecido"));
            }
            let Some(number) = weight.as_f32() else {
                return Err(invalid(&label, "esperado um número entre 0 e 1"));
            };
            if !number.is_finite() || number < 0.0 || number > 1.0 {
                return Err(invalid(&label, "esperado um número entre 0 e 1"));
            }
            bones.insert(bone.clone(), number);
        }
        Ok(Self { bones })
    }

    /// `BasePoseRetention.over`: this set with `other` laid under it — a bone
    /// both mention keeps this set's weight. An empty set on either side
    /// returns the other one.
    pub fn over(&self, other: &BasePoseRetention) -> BasePoseRetention {
        if self.bones.is_empty() {
            return other.clone();
        }
        if other.bones.is_empty() {
            return self.clone();
        }
        let mut merged = other.bones.clone();
        for (bone, weight) in &self.bones {
            merged.insert(bone.clone(), *weight);
        }
        BasePoseRetention { bones: merged }
    }

    /// `BasePoseRetention.value`: the weight `bone` keeps, or `NaN` when the
    /// set does not mention it. With `mirror`, `left_` and `right_` prefixes
    /// swap before the lookup; an upper-body bone with no value of its own
    /// falls back to the `waist` value.
    pub fn value(&self, bone: &str, mirror: bool) -> f32 {
        let mut bone = bone;
        let owned;
        if mirror {
            if let Some(rest) = bone.strip_prefix("left_") {
                owned = format!("right_{}", rest);
                bone = &owned;
            } else if let Some(rest) = bone.strip_prefix("right_") {
                owned = format!("left_{}", rest);
                bone = &owned;
            }
        }
        if let Some(weight) = self.bones.get(bone) {
            return *weight;
        }
        if Self::UPPER_BODY.contains(&bone)
            && let Some(waist) = self.bones.get("waist")
        {
            return *waist;
        }
        f32::NAN
    }
}

/// `BasePoseRetention.invalid`: the Java error message.
fn invalid(label: &str, message: &str) -> String {
    format!("Compat {}: {}", label, message)
}

// ---------------------------------------------------------------------------
// MovementTuning
// ---------------------------------------------------------------------------

/// `MovementTuning.Directions`: the sprint weights per direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Directions {
    pub forward: f32,
    pub backward: f32,
    pub left: f32,
    pub right: f32,
}

/// `MovementTuning`: the numbers the animation layer is tuned with, each a
/// small pure function of the pack's settings.
pub mod movement_tuning {
    use super::{Directions, Vec3};
    use crate::library::Quat;

    /// `MovementTuning.AIR_HOLD_STEPS`: how finely the jump's air hold is
    /// searched.
    pub const AIR_HOLD_STEPS: u32 = 96;
    /// The stride distance a pack may ask for at most.
    const STRIDE_MAX: f32 = 8.0;
    /// The gait amplitude a pack may ask for at most, when it sets no cap.
    const AMPLITUDE_CAP: f32 = 1.2;
    /// The fallback distance a full landing weighs in at.
    const LANDING_FULL: f64 = 5.0;
    /// The per-step growth of the landing's blend distance (`0.08`), damped
    /// by `0.98F` each step.
    const LANDING_STEP: f64 = 0.08;
    const LANDING_DAMP: f64 = 0.98_f32 as f64;
    /// The stride scale's floor.
    const USE_STRIDE_MIN: f32 = 0.1;
    /// The walk weight's stride divisor at scale 1.
    const WALK_DIVISOR: f32 = 0.11;
    /// The fade rate's bounds, in seconds.
    const FADE_MIN: f32 = 0.05;
    const FADE_MAX: f32 = 5.0;

    /// `MovementTuning.strideDistance`: the stride distance a pack asked
    /// for, clamped into `min..=8.0` and capped by `max`; `0.0` when the
    /// value is broken or out of range.
    pub fn stride_distance(value: f32, min: f32, max: f32) -> f32 {
        if value.is_finite() && value >= min && value <= STRIDE_MAX {
            value.min(max)
        } else {
            0.0
        }
    }

    /// `MovementTuning.sprintDirections`: the sprint direction weights with
    /// `blend` (`0.0..=1.0`) of the sideways and backward weights folded
    /// into the forward one.
    pub fn sprint_directions(
        forward: f32,
        backward: f32,
        left: f32,
        right: f32,
        blend: f32,
    ) -> Directions {
        let blend = if blend.is_finite() {
            blend.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Directions {
            forward: forward + (backward + left + right) * blend,
            backward: backward * (1.0 - blend),
            left: left * (1.0 - blend),
            right: right * (1.0 - blend),
        }
    }

    /// `MovementTuning.gaitAmplitude`: how far the legs swing at `speed`,
    /// relative to a full walk at `stride`, capped by `cap` (`1.2` when the
    /// pack sets none). A broken or non-positive stride leaves the swing at
    /// `1.0`; a broken speed flattens it.
    pub fn gait_amplitude(speed: f32, stride: f32, cap: f32) -> f32 {
        if !stride.is_finite() || stride <= 0.0 {
            return 1.0;
        }
        if !speed.is_finite() {
            return 0.0;
        }
        let cap = if cap.is_finite() {
            cap.max(0.0)
        } else {
            AMPLITUDE_CAP
        };
        (speed.max(0.0) / stride).clamp(0.0, cap)
    }

    /// `MovementTuning.earlyJumpLanding`: whether a landing `t` seconds into
    /// a jump counts as early (before either of the two windows closes).
    pub fn early_jump_landing(t: f32, a: f32, b: f32) -> bool {
        t.is_finite() && a.is_finite() && b.is_finite() && t >= 0.0 && t < a && t < b
    }

    /// `MovementTuning.landingAmplitude`: how much of the landing animation
    /// shows for a fall of `distance` blocks, against a full weight at
    /// `full` blocks (`5.0` when the pack sets none). The full distance is
    /// reached by summing a damped growth (`(d + 0.08) * 0.98F` a step) for
    /// at most 1024 steps, exactly as the Java does, so a pack's `full`
    /// lands on the same weight it always did.
    pub fn landing_amplitude(distance: f32, full: f32) -> f32 {
        if !distance.is_finite() || distance <= 0.0 {
            return 0.0;
        }
        let full = if full.is_finite() && full > 0.0 {
            full as f64
        } else {
            LANDING_FULL
        };
        let mut step = 0.0_f64;
        let mut total = 0.0_f64;
        let mut i = 0;
        while i < 1024 && total < full {
            step = (step + LANDING_STEP) * LANDING_DAMP;
            total += step;
            i += 1;
        }
        (distance as f64 / step).clamp(0.0, 1.0) as f32
    }

    /// `MovementTuning.jumpFamilies`: the clip-name prefixes a jump matches,
    /// by whether the player was running and whether they were moving.
    pub fn jump_families(running: bool, moving: bool) -> Vec<&'static str> {
        if !running {
            vec!["jump_idle_"]
        } else if moving {
            vec!["jump_running_", "jump_moving_"]
        } else {
            vec!["jump_moving_"]
        }
    }

    /// `MovementTuning.jumpContact`: the landing clip of a jump family, the
    /// family's own `landing` when it has one, the shared `jump_landing`
    /// otherwise.
    pub fn jump_contact<T>(lookup: impl Fn(&str) -> Option<T>, prefix: Option<&str>) -> Option<T> {
        let family = prefix.filter(|p| !p.is_empty());
        let own = family.and_then(|prefix| lookup(&format!("{}landing", prefix)));
        own.or_else(|| lookup("jump_landing"))
    }

    /// `MovementTuning.gaitPhase`: the cycle phase, reversed (`1 - phase`)
    /// when the clip runs backwards, and `0.0` when the phase is broken.
    pub fn gait_phase(phase: f32, reverse: bool) -> f32 {
        if !reverse {
            return phase;
        }
        if !phase.is_finite() {
            return 0.0;
        }
        let fraction = phase - phase.floor();
        if fraction == 0.0 { 0.0 } else { 1.0 - fraction }
    }

    /// `MovementTuning.useStrideScale`: the stride scale a use animation
    /// plays at, clamped into `0.1..=1.0`; `1.0` when broken.
    pub fn use_stride_scale(value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(USE_STRIDE_MIN, 1.0)
        } else {
            1.0
        }
    }

    /// `MovementTuning.walkWeight`: how much of the walk cycle shows at
    /// `speed`, against a full walk at `0.11 * scale` blocks a tick.
    pub fn walk_weight(speed: f32, scale: f32) -> f32 {
        let scale = if scale.is_finite() {
            scale.clamp(USE_STRIDE_MIN, 1.0)
        } else {
            1.0
        };
        if speed.is_finite() {
            (speed / (WALK_DIVISOR * scale)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// `MovementTuning.leanUnder`: the Euler angles (degrees) of `base`
    /// turned by `under` through `weight` (`0.0..=1.0`).
    pub fn lean_under(base: Vec3, under: Vec3, weight: f32) -> Vec3 {
        if weight <= 0.0 {
            return base;
        }
        let turned = quaternion(scaled(under, weight.min(1.0))) * quaternion(base);
        scaled(
            turned.get_euler_zyx(),
            (180.0 / std::f64::consts::PI) as f32,
        )
    }

    /// `MovementTuning.shoulderOnLean`: how the shoulder offset `a` moves
    /// when the model leans by `lean` about `lean_pivot` and shifts by
    /// `shift`, through `weight`.
    pub fn shoulder_on_lean(a: Vec3, b: Vec3, c: Vec3, d: Vec3, weight: f32) -> Vec3 {
        if weight <= 0.0 {
            return Vec3::ZERO;
        }
        let f = weight.min(1.0);
        let base = Vec3::new(a.x + b.x, a.y - b.y, a.z + b.z);
        let turned = quaternion(scaled(c, f)).transform_by(base);
        let moved = Vec3::new(turned.x + d.x * f, turned.y - d.y * f, turned.z + d.z * f);
        let delta = moved - base;
        Vec3::new(delta.x, -delta.y, delta.z)
    }

    /// `MovementTuning.armPivot`: the arms' rotation pivot, in model pixels,
    /// mirrored for the left arm.
    pub fn arm_pivot(right: bool) -> Vec3 {
        Vec3::new(if right { 5.0 } else { -5.0 }, 2.0, 0.0)
    }

    /// `MovementTuning.fadeRate`: how fast a weight moves per second,
    /// `1 / seconds` clamped into `0.05..5.0` seconds; `fallback` when the
    /// pack's value is broken or not positive.
    pub fn fade_rate(seconds: f32, fallback: f32) -> f32 {
        let seconds = if seconds.is_finite() && seconds > 0.0 {
            seconds
        } else {
            fallback
        };
        1.0 / seconds.clamp(FADE_MIN, FADE_MAX)
    }

    /// `MovementTuning.shortWay`: every angle wrapped into `-180..180`.
    pub fn short_way(angles: Vec3) -> Vec3 {
        Vec3::new(
            wrap_degrees(angles.x),
            wrap_degrees(angles.y),
            wrap_degrees(angles.z),
        )
    }

    /// `MovementTuning.wrapDegrees`: an angle wrapped into `-180..180`
    /// (`NaN` stays `NaN`).
    pub fn wrap_degrees(degrees: f32) -> f32 {
        let wrapped = degrees % 360.0;
        if wrapped >= 180.0 {
            wrapped - 360.0
        } else if wrapped < -180.0 {
            wrapped + 360.0
        } else {
            wrapped
        }
    }

    /// A vector scaled componentwise.
    fn scaled(v: Vec3, k: f32) -> Vec3 {
        Vec3::new(v.x * k, v.y * k, v.z * k)
    }

    /// A quaternion from Euler degrees: JOML's `rotationZYX`.
    fn quaternion(euler: Vec3) -> Quat {
        Quat::from_euler_zyx(euler.z, euler.y, euler.x)
    }
}

pub use movement_tuning as tuning;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn value(text: &str) -> Value {
        parse(text).unwrap_or_else(|e| panic!("parse failed: {}", e))
    }

    #[test]
    fn the_constants_hold_their_java_values() {
        assert_eq!(NEVER, i32::MIN);
        assert_eq!((LIGHT, MEDIUM, HEAVY), (1, 2, 3));
        assert_eq!((FALL, FREE_FALL, HARD_FALL), (1, 2, 3));
        assert_eq!((TOSS, PEARL, BREW), (1, 2, 3));
    }

    #[test]
    fn a_fresh_snapshot_starts_as_the_java_fields_do() {
        let signals = MotionSignals::new();
        assert_eq!(signals.tick, NEVER);
        assert!(signals.grounded);
        assert_eq!(signals.fall_weight, 0);
        assert_eq!(signals.fall_distance, 0.0);
        assert!(!signals.elytra);
        assert_eq!(signals.swim_vertical, 0);
        assert_eq!(signals.climb_direction, 0);
        assert_eq!(signals.walk_forward, 0.0);
        assert_eq!(signals.walk_left, 0.0);
        assert_eq!(signals.still_seconds, 0.0);
        // The event ticks have never happened, and the last hurt came from
        // the front.
        for event in [
            signals.jumped,
            signals.landed,
            signals.run_started,
            signals.run_stopped,
            signals.crouched,
            signals.stood_up,
            signals.splashed,
            signals.boosted,
            signals.swapped,
            signals.equipped,
            signals.interacted,
            signals.threw,
            signals.ignited,
            signals.sheared,
            signals.released,
            signals.hurled,
            signals.lunged,
            signals.cast,
            signals.reeled,
            signals.hurt,
            signals.idle_event,
        ] {
            assert_eq!(event, NEVER);
        }
        assert!(!signals.jump_moving);
        assert_eq!(signals.thrown, 0);
        assert_eq!(signals.landing_weight, 0);
        assert!(signals.hurt_from_front);
    }

    #[test]
    fn since_ages_an_event_and_fresh_spots_the_next_tick() {
        assert_eq!(MotionSignals::since(NEVER, 10), i32::MAX);
        assert_eq!(MotionSignals::since(7, 10), 3);
        assert_eq!(MotionSignals::since(7, 7), 0);
        // An event in the future is no age at all.
        assert_eq!(MotionSignals::since(11, 10), i32::MAX);

        let mut signals = MotionSignals::new();
        assert!(!signals.fresh(10));
        signals.tick = 10;
        assert!(signals.fresh(10));
        assert!(signals.fresh(11));
        assert!(!signals.fresh(12));
        assert!(!signals.fresh(9));
    }

    #[test]
    fn begin_clears_the_per_tick_fields_and_keeps_the_events() {
        let mut signals = MotionSignals {
            tick: 5,
            grounded: false,
            elytra: true,
            elytra_speed: 2.0,
            elytra_bank: 0.5,
            creative_flight: true,
            fast_flight: true,
            climbing: true,
            crawling: true,
            on_fence: true,
            running: true,
            run_seconds: 3.0,
            moving: true,
            walk_forward: 1.0,
            walk_left: -1.0,
            music_nearby: true,
            still_seconds: 4.0,
            freezing: true,
            fall_weight: HEAVY,
            fall_distance: 9.0,
            water_below: true,
            lava_below: true,
            surface_swimming: true,
            submerged_swimming: true,
            swim_sprint: true,
            swim_vertical: 1,
            shallow_water: true,
            climb_direction: -1,
            mining: true,
            charging_crossbow: true,
            charging_spear: true,
            charging_trident: true,
            map_view: true,
            jumped: 3,
            landed: 4,
            hurt: 2,
            ..MotionSignals::new()
        };
        signals.begin(6);
        assert_eq!(signals.tick, 6);
        // The events survive the tick, and so does the ground state: the
        // tick's collector sets it, `begin` only clears what it rebuilds.
        assert_eq!(signals.jumped, 3);
        assert_eq!(signals.landed, 4);
        assert_eq!(signals.hurt, 2);
        assert!(!signals.grounded);
        assert!(!signals.elytra);
        assert_eq!(signals.elytra_speed, 0.0);
        assert_eq!(signals.elytra_bank, 0.0);
        assert!(!signals.creative_flight);
        assert!(!signals.fast_flight);
        assert!(!signals.climbing);
        assert!(!signals.crawling);
        assert!(!signals.on_fence);
        assert!(!signals.running);
        assert_eq!(signals.run_seconds, 0.0);
        assert!(!signals.moving);
        assert_eq!(signals.walk_forward, 0.0);
        assert_eq!(signals.walk_left, 0.0);
        assert!(!signals.music_nearby);
        assert_eq!(signals.still_seconds, 0.0);
        assert!(!signals.freezing);
        assert_eq!(signals.fall_weight, 0);
        assert_eq!(signals.fall_distance, 0.0);
        assert!(!signals.water_below);
        assert!(!signals.lava_below);
        assert!(!signals.surface_swimming);
        assert!(!signals.submerged_swimming);
        assert!(!signals.swim_sprint);
        assert_eq!(signals.swim_vertical, 0);
        assert!(!signals.shallow_water);
        assert_eq!(signals.climb_direction, 0);
        assert!(!signals.mining);
        assert!(!signals.charging_crossbow);
        assert!(!signals.charging_spear);
        assert!(!signals.charging_trident);
        assert!(!signals.map_view);
    }

    #[test]
    fn the_landing_and_fall_weights_measure_the_same_way() {
        // Hard when either measure reaches its hard threshold.
        assert_eq!(
            MotionSignals::landing_weight(9.0, 0.0, 2.0, 3.0, 4.0, 5.0),
            HEAVY
        );
        assert_eq!(
            MotionSignals::landing_weight(0.0, 6.0, 2.0, 3.0, 4.0, 5.0),
            HEAVY
        );
        // Medium when only a medium threshold is reached.
        assert_eq!(
            MotionSignals::landing_weight(3.0, 0.0, 2.0, 3.0, 4.0, 5.0),
            MEDIUM
        );
        assert_eq!(
            MotionSignals::landing_weight(0.0, 3.5, 2.0, 3.0, 4.0, 5.0),
            MEDIUM
        );
        // Light otherwise, and exactly on a threshold is reached.
        assert_eq!(
            MotionSignals::landing_weight(1.0, 1.0, 2.0, 3.0, 4.0, 5.0),
            LIGHT
        );
        assert_eq!(
            MotionSignals::landing_weight(4.0, 0.0, 2.0, 3.0, 4.0, 5.0),
            HEAVY
        );
        assert_eq!(
            MotionSignals::landing_weight(2.0, 0.0, 2.0, 3.0, 4.0, 5.0),
            MEDIUM
        );
        // The fall weight is the same measure.
        for (a, b) in [(9.0, 0.0), (0.0, 6.0), (3.0, 0.0), (1.0, 1.0), (4.0, 0.0)] {
            assert_eq!(
                MotionSignals::fall_weight(a, b, 2.0, 3.0, 4.0, 5.0),
                MotionSignals::landing_weight(a, b, 2.0, 3.0, 4.0, 5.0)
            );
        }
    }

    #[test]
    fn the_phase_match_walks_the_cycle_for_the_cheapest_cost() {
        use gait_phase_matcher::match_phase;

        // A cost with one dip finds it from anywhere, to within the fine
        // pass's 1/512 of a cycle.
        let cost = |phase: f32| {
            let d = (phase - 0.75).abs();
            d.min(1.0 - d) as f64 // shortest way round the cycle
        };
        let matched = match_phase(0.1, cost);
        assert!((matched - 0.75).abs() < 1.0 / 512.0, "matched {}", matched);

        // A cost that is never finite leaves the phase alone.
        assert_eq!(match_phase(0.3, |_| f64::NAN), 0.3);
        // A flat cost keeps the starting phase (no step beats it).
        assert_eq!(match_phase(0.42, |_| 1.0), 0.42);
        // The result is always wrapped into 0.0..1.0.
        assert!((0.0..1.0).contains(&matched));

        // The wrap is `x - floor(x)`.
        assert_eq!(gait_phase_matcher::wrap(1.25), 0.25);
        assert_eq!(gait_phase_matcher::wrap(-0.25), 0.75);
        assert_eq!(gait_phase_matcher::wrap(3.0), 0.0);
    }

    #[test]
    fn retention_parses_bones_and_names_the_bad_ones() {
        // Nothing to parse is the empty set.
        assert!(BasePoseRetention::parse(None, "test").unwrap().is_empty());

        let parsed = BasePoseRetention::parse(
            Some(&value("{\"head\":1.0,\"body\":0.5,\"left_arm\":0.0}")),
            "test",
        )
        .unwrap();
        assert!((parsed.value("head", false) - 1.0).abs() < 1e-6);
        assert!((parsed.value("body", false) - 0.5).abs() < 1e-6);
        assert!((parsed.value("left_arm", false) - 0.0).abs() < 1e-6);
        // A bone the set does not mention is NaN.
        assert!(parsed.value("right_leg", false).is_nan());

        // The errors carry the Java messages.
        assert_eq!(
            BasePoseRetention::parse(Some(&value("[]")), "ctx").unwrap_err(),
            "Compat ctx: esperado um objeto por osso"
        );
        assert_eq!(
            BasePoseRetention::parse(Some(&value("{\"tail\":1.0}")), "ctx").unwrap_err(),
            "Compat ctx.tail: osso desconhecido"
        );
        assert_eq!(
            BasePoseRetention::parse(Some(&value("{\"head\":2.0}")), "ctx").unwrap_err(),
            "Compat ctx.head: esperado um número entre 0 e 1"
        );
        assert_eq!(
            BasePoseRetention::parse(Some(&value("{\"head\":-0.5}")), "ctx").unwrap_err(),
            "Compat ctx.head: esperado um número entre 0 e 1"
        );
        assert_eq!(
            BasePoseRetention::parse(Some(&value("{\"head\":\"lots\"}")), "ctx").unwrap_err(),
            "Compat ctx.head: esperado um número entre 0 e 1"
        );
        // A JSON null is not an object either.
        assert_eq!(
            BasePoseRetention::parse(Some(&value("null")), "ctx").unwrap_err(),
            "Compat ctx: esperado um objeto por osso"
        );
    }

    #[test]
    fn retention_mirrors_and_falls_back_to_the_waist() {
        let parsed =
            BasePoseRetention::parse(Some(&value("{\"right_arm\":0.25,\"waist\":0.75}")), "test")
                .unwrap();

        // Mirroring swaps the left and right prefixes: the stored right arm
        // answers for the left, and the mirrored right arm finds nothing, so
        // it takes the waist's value.
        assert!((parsed.value("left_arm", true) - 0.25).abs() < 1e-6);
        assert!((parsed.value("right_arm", true) - 0.75).abs() < 1e-6);
        // Without mirroring, only the stored side answers.
        assert!((parsed.value("right_arm", false) - 0.25).abs() < 1e-6);

        // An upper-body bone with nothing of its own takes the waist's value.
        assert!((parsed.value("head", false) - 0.75).abs() < 1e-6);
        assert!((parsed.value("body", false) - 0.75).abs() < 1e-6);
        // The other arm is upper body too, so it also takes the waist.
        assert!((parsed.value("left_arm", false) - 0.75).abs() < 1e-6);
        // A leg is not upper body, so it stays NaN.
        assert!(parsed.value("left_leg", false).is_nan());
    }

    #[test]
    fn retention_over_keeps_the_base_where_both_speak() {
        let base =
            BasePoseRetention::parse(Some(&value("{\"head\":1.0,\"body\":0.5}")), "a").unwrap();
        let overlay =
            BasePoseRetention::parse(Some(&value("{\"body\":0.25,\"left_leg\":1.0}")), "b")
                .unwrap();

        // `base.over(overlay)`: the base's weights win, the overlay's new
        // bones join.
        let merged = base.over(&overlay);
        assert!((merged.value("head", false) - 1.0).abs() < 1e-6);
        assert!((merged.value("body", false) - 0.5).abs() < 1e-6);
        assert!((merged.value("left_leg", false) - 1.0).abs() < 1e-6);

        // An empty side returns the other whole.
        let empty = BasePoseRetention::empty();
        assert_eq!(empty.over(&base), base);
        assert_eq!(base.over(&empty), base);
        assert_eq!(BasePoseRetention::BONES.len(), 8);
        assert_eq!(BasePoseRetention::UPPER_BODY.len(), 4);
    }

    #[test]
    fn tuning_clamps_the_stride_distance() {
        use tuning::stride_distance;
        // In range: capped by `max`, never past 8.
        assert!((stride_distance(2.0, 0.5, 3.0) - 2.0).abs() < 1e-6);
        assert!((stride_distance(5.0, 0.5, 3.0) - 3.0).abs() < 1e-6);
        assert!((stride_distance(9.0, 0.5, 3.0) - 0.0).abs() < 1e-6);
        // Below the minimum or broken: nothing.
        assert!((stride_distance(0.4, 0.5, 3.0) - 0.0).abs() < 1e-6);
        assert!((stride_distance(f32::NAN, 0.5, 3.0) - 0.0).abs() < 1e-6);
        assert!((stride_distance(f32::INFINITY, 0.5, 3.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tuning_folds_the_sprint_into_the_forward_weight() {
        use tuning::sprint_directions;
        // No blend leaves the four weights alone.
        let none = sprint_directions(1.0, 0.7, 0.85, 0.85, 0.0);
        assert!((none.forward - 1.0).abs() < 1e-6);
        assert!((none.backward - 0.7).abs() < 1e-6);
        // A full blend folds the other three into the forward weight.
        let all = sprint_directions(1.0, 0.7, 0.85, 0.85, 1.0);
        assert!((all.forward - (1.0 + 0.7 + 0.85 + 0.85)).abs() < 1e-6);
        assert_eq!(all.backward, 0.0);
        assert_eq!(all.left, 0.0);
        assert_eq!(all.right, 0.0);
        // Halfway blends both ways.
        let half = sprint_directions(1.0, 0.7, 0.85, 0.85, 0.5);
        assert!((half.forward - (1.0 + 1.2)).abs() < 1e-6);
        assert!((half.backward - 0.35).abs() < 1e-6);
        // A broken blend reads as none.
        let broken = sprint_directions(1.0, 0.7, 0.85, 0.85, f32::NAN);
        assert!((broken.forward - 1.0).abs() < 1e-6);
        // A blend past 1 clamps.
        let over = sprint_directions(1.0, 0.7, 0.85, 0.85, 4.0);
        assert_eq!(over.backward, 0.0);
    }

    #[test]
    fn tuning_scales_the_gait_and_the_walk_weight() {
        use tuning::{gait_amplitude, walk_weight};
        // The swing grows with speed up to the cap.
        assert!((gait_amplitude(0.5, 1.0, 1.2) - 0.5).abs() < 1e-6);
        assert!((gait_amplitude(2.0, 1.0, 1.2) - 1.2).abs() < 1e-6);
        // No cap means the Java default of 1.2.
        assert!((gait_amplitude(2.0, 1.0, f32::NAN) - 1.2).abs() < 1e-6);
        // A broken stride leaves the swing at 1; a broken speed flattens it.
        assert!((gait_amplitude(0.5, 0.0, 1.2) - 1.0).abs() < 1e-6);
        assert!((gait_amplitude(f32::NAN, 1.0, 1.2) - 0.0).abs() < 1e-6);
        // A negative speed is no swing at all.
        assert!((gait_amplitude(-1.0, 1.0, 1.2) - 0.0).abs() < 1e-6);

        // The walk weight fills in by 0.11 blocks a tick at scale 1.
        assert!((walk_weight(0.11, 1.0) - 1.0).abs() < 1e-6);
        assert!((walk_weight(0.05, 1.0) - 0.45454547).abs() < 1e-6);
        assert!((walk_weight(1.0, 1.0) - 1.0).abs() < 1e-6);
        // A smaller scale fills sooner; a broken speed is nothing.
        assert!((walk_weight(0.05, 0.5) - 0.909_090_94).abs() < 1e-6);
        assert!((walk_weight(f32::NAN, 1.0) - 0.0).abs() < 1e-6);
        // A broken scale reads as 1.
        assert!((walk_weight(0.11, f32::NAN) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tuning_lands_on_the_java_blend_distance() {
        use tuning::landing_amplitude;
        // Nothing below the onset, full at and past the Java blend distance.
        assert_eq!(landing_amplitude(0.0, 5.0), 0.0);
        assert_eq!(landing_amplitude(-1.0, 5.0), 0.0);
        assert_eq!(landing_amplitude(f32::NAN, 5.0), 0.0);
        assert!((landing_amplitude(0.843_910_6, 5.0) - 1.0).abs() < 1e-6);
        assert!((landing_amplitude(9.0, 5.0) - 1.0).abs() < 1e-6);
        // In between, the share of the blend distance the fall reached.
        assert!((landing_amplitude(0.4, 5.0) - 0.47398388).abs() < 1e-6);
        assert!((landing_amplitude(0.25, 5.0) - 0.29623994).abs() < 1e-6);
        // A pack's own full distance scales the same way.
        assert!((landing_amplitude(0.4, 0.4) - 1.0).abs() < 1e-6);
        // A broken full distance reads as the Java default of 5.
        assert!((landing_amplitude(0.4, 0.0) - 0.47398388).abs() < 1e-6);
    }

    #[test]
    fn tuning_names_the_jump_families_and_their_landing() {
        use tuning::{jump_contact, jump_families};
        assert_eq!(jump_families(false, true), vec!["jump_idle_"]);
        assert_eq!(
            jump_families(true, true),
            vec!["jump_running_", "jump_moving_"]
        );
        assert_eq!(jump_families(true, false), vec!["jump_moving_"]);

        // A family with its own landing keeps it; one without falls back to
        // the shared landing.
        let clips = |name: &str| match name {
            "jump_running_landing" => Some("running landing"),
            "jump_landing" => Some("shared landing"),
            _ => None,
        };
        assert_eq!(
            jump_contact(clips, Some("jump_running_")),
            Some("running landing")
        );
        assert_eq!(
            jump_contact(clips, Some("jump_idle_")),
            Some("shared landing")
        );
        // An empty family name goes straight to the shared landing.
        assert_eq!(jump_contact(clips, Some("")), Some("shared landing"));
        assert_eq!(jump_contact(clips, None), Some("shared landing"));
        // Nothing anywhere is nothing.
        assert_eq!(jump_contact(|_| None::<&str>, Some("jump_idle_")), None);
    }

    #[test]
    fn tuning_phases_scales_and_fades() {
        use tuning::{early_jump_landing, fade_rate, gait_phase, use_stride_scale};
        // The phase runs backwards only when asked.
        assert!((gait_phase(0.3, false) - 0.3).abs() < 1e-6);
        assert!((gait_phase(0.3, true) - 0.7).abs() < 1e-6);
        assert!((gait_phase(1.3, true) - 0.7).abs() < 1e-6);
        assert_eq!(gait_phase(2.0, true), 0.0);
        assert_eq!(gait_phase(f32::NAN, true), 0.0);
        assert!(gait_phase(f32::NAN, false).is_nan());

        // A landing inside both windows is early.
        assert!(early_jump_landing(0.1, 0.2, 0.3));
        assert!(!early_jump_landing(0.25, 0.2, 0.3));
        assert!(!early_jump_landing(-0.1, 0.2, 0.3));
        assert!(!early_jump_landing(f32::NAN, 0.2, 0.3));

        // The stride scale clamps into 0.1..1; broken reads as 1.
        assert!((use_stride_scale(0.5) - 0.5).abs() < 1e-6);
        assert!((use_stride_scale(0.01) - 0.1).abs() < 1e-6);
        assert!((use_stride_scale(4.0) - 1.0).abs() < 1e-6);
        assert!((use_stride_scale(f32::NAN) - 1.0).abs() < 1e-6);

        // The fade rate is one over the seconds, clamped into 0.05..5.
        assert!((fade_rate(0.2, 6.0) - 5.0).abs() < 1e-6);
        assert!((fade_rate(0.5, 6.0) - 2.0).abs() < 1e-6);
        assert!((fade_rate(0.01, 6.0) - 20.0).abs() < 1e-6);
        assert!((fade_rate(50.0, 6.0) - 0.2).abs() < 1e-6);
        // A broken value takes the fallback.
        assert!((fade_rate(0.0, 4.0) - 0.25).abs() < 1e-6);
        assert!((fade_rate(f32::NAN, 4.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn tuning_wraps_degrees_the_java_way() {
        use tuning::{short_way, wrap_degrees};
        assert!((wrap_degrees(450.0) - 90.0).abs() < 1e-6);
        assert!((wrap_degrees(-90.0) - (-90.0)).abs() < 1e-6);
        assert!((wrap_degrees(-270.0) - 90.0).abs() < 1e-6);
        assert!((wrap_degrees(180.0) - (-180.0)).abs() < 1e-6);
        assert!((wrap_degrees(-180.0) - (-180.0)).abs() < 1e-6);
        assert_eq!(wrap_degrees(0.0), 0.0);
        // NaN stays NaN, like Java's `%`.
        assert!(wrap_degrees(f32::NAN).is_nan());

        let wrapped = short_way(Vec3::new(450.0, -270.0, 30.0));
        assert!((wrapped.x - 90.0).abs() < 1e-6);
        assert!((wrapped.y - 90.0).abs() < 1e-6);
        assert!((wrapped.z - 30.0).abs() < 1e-6);
    }

    #[test]
    fn tuning_leans_and_pivots_like_joml() {
        use tuning::{arm_pivot, lean_under, shoulder_on_lean};
        // No weight leaves the angles alone.
        let base = Vec3::new(10.0, 20.0, 30.0);
        assert_eq!(lean_under(base, Vec3::new(5.0, 5.0, 5.0), 0.0), base);

        // A z quarter turn composed with an x quarter turn reads back as
        // x 90, z 90 (the JOML Euler ZYX of the product).
        let leaned = lean_under(Vec3::new(90.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 90.0), 1.0);
        assert!((leaned.x - 90.0).abs() < 1e-4, "x was {}", leaned.x);
        assert!(leaned.y.abs() < 1e-4, "y was {}", leaned.y);
        assert!((leaned.z - 90.0).abs() < 1e-4, "z was {}", leaned.z);

        // Half the under-turn halves the z angle.
        let half = lean_under(Vec3::new(90.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 90.0), 0.5);
        assert!((half.z - 45.0).abs() < 1e-4, "z was {}", half.z);

        // No weight leaves the shoulder where it was.
        assert_eq!(
            shoulder_on_lean(
                Vec3::new(1.0, 2.0, 3.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(0.0, 0.0, 90.0),
                Vec3::new(1.0, 1.0, 1.0),
                0.0
            ),
            Vec3::ZERO
        );

        // The arm pivots mirror each other, 2 pixels above the middle.
        let right = arm_pivot(true);
        let left = arm_pivot(false);
        assert_eq!(right, Vec3::new(5.0, 2.0, 0.0));
        assert_eq!(left, Vec3::new(-5.0, 2.0, 0.0));
    }
}

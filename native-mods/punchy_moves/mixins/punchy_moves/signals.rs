//! Motion signals and gait maths for Punchy Moves.
//!
//! Everything the animation layer needs to decide *what the player is doing*,
//! *how far the legs have swung* and *how much of the vanilla pose to keep*,
//! written from scratch and kept free of engine types so the tests run without
//! a Horizon build:
//!
//! - [`MotionSignals`]: one snapshot of the player's movement state per tick,
//!   plus the [`FallWeight`] scale and the [`action`] bits a snapshot reports.
//! - [`GaitPhaseMatcher`]: the walk-cycle phase, its wrapping and the
//!   nearest-phase matching used to pick a clip without popping.
//! - [`BasePoseRetention`]: per-bone weights in `0.0..=1.0` saying how much of
//!   the base pose a bone keeps, with the whole-body and upper-body presets.
//! - [`MovementTuning`]: the numbers those three read (stride length, gait and
//!   landing amplitudes, fade rate, short-way blending, a configurable turn
//!   and the sprint direction weights).

use crate::json::Value;

// ---------------------------------------------------------------------------
// MotionSignals
// ---------------------------------------------------------------------------

/// The weight a fall or a landing puts into the animation mix, in
/// `0.0..=1.0`. Absent faults it is [`FallWeight::NONE`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct FallWeight(f32);

impl FallWeight {
    /// Touching the ground: no fall animation at all.
    pub const NONE: FallWeight = FallWeight(0.0);
    /// A one-block drop: the softest landing the animator reacts to.
    pub const LIGHT: FallWeight = FallWeight(0.25);
    /// A fall of a little over two blocks.
    pub const HEAVY: FallWeight = FallWeight(0.75);
    /// The full landing animation.
    pub const LANDING: FallWeight = FallWeight(1.0);

    /// A drop below this many blocks is not worth animating.
    pub const ONSET_BLOCKS: f32 = 0.5;
    /// A drop of this many blocks or more plays the landing at full weight.
    pub const FULL_BLOCKS: f32 = 4.0;

    /// Clamps the weight into `0.0..=1.0`; a NaN becomes `0.0`.
    pub fn new(weight: f32) -> Self {
        if weight.is_nan() {
            return FallWeight::NONE;
        }
        FallWeight(weight.clamp(0.0, 1.0))
    }

    /// The weight a drop of `distance` blocks earns: nothing below
    /// [`FallWeight::ONSET_BLOCKS`], rising linearly to full weight at
    /// [`FallWeight::FULL_BLOCKS`]. Negative and NaN distances earn nothing.
    pub fn from_distance(distance: f32) -> Self {
        if !distance.is_finite() || distance <= Self::ONSET_BLOCKS {
            return FallWeight::NONE;
        }
        let span = Self::FULL_BLOCKS - Self::ONSET_BLOCKS;
        FallWeight::new((distance - Self::ONSET_BLOCKS) / span)
    }

    /// The weight as a plain number.
    pub fn get(self) -> f32 {
        self.0
    }

    /// Whether the weight is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

/// Bit values for the discrete actions a [`MotionSignals`] snapshot can
/// report. A snapshot reports them through [`MotionSignals::actions`].
pub mod action {
    /// Breaking a block.
    pub const MINING: u32 = 1 << 0;
    /// Drawing a crossbow.
    pub const CHARGING_CROSSBOW: u32 = 1 << 1;
    /// Winding up a spear throw.
    pub const CHARGING_SPEAR: u32 = 1 << 2;
    /// Holding a trident ready to throw.
    pub const CHARGING_TRIDENT: u32 = 1 << 3;
    /// Looking at a held map.
    pub const MAP_VIEW: u32 = 1 << 4;
    /// A jukebox or note block is playing within earshot.
    pub const MUSIC_NEARBY: u32 = 1 << 5;

    /// Every action bit.
    pub const ALL: u32 =
        MINING | CHARGING_CROSSBOW | CHARGING_SPEAR | CHARGING_TRIDENT | MAP_VIEW | MUSIC_NEARBY;
}

/// One tick's worth of movement state: what the player touches, how they
/// move, and which discrete action they are in the middle of.
///
/// The fields are plain data on purpose: the animation layer reads them
/// field by field, and the capture code fills them in one place per tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionSignals {
    /// The tick this snapshot was taken on; monotonically non-decreasing.
    pub tick: u64,
    /// Standing on something solid.
    pub grounded: bool,
    /// How much a fall or landing weighs in the mix, `0.0..=1.0`.
    pub fall_weight: f32,
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
    /// Vertical swim input, `-1.0` (down) .. `1.0` (up).
    pub swim_vertical: f32,
    /// Standing in water shallow enough to wade through.
    pub shallow_water: bool,
    /// Holding a ladder, vine or scaffolding.
    pub climbing: bool,
    /// Vertical climb input, `-1.0` (down) .. `1.0` (up).
    pub climb_direction: f32,
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
    /// Seconds the player has been running, `0.0` when not.
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
}

impl MotionSignals {
    /// A snapshot with every flag off and every number zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Water or lava directly below the player.
    pub fn in_fluid(&self) -> bool {
        self.water_below || self.lava_below
    }

    /// Swimming on or under the surface.
    pub fn swimming(&self) -> bool {
        self.surface_swimming || self.submerged_swimming
    }

    /// Any kind of flight: creative, fast or an elytra glide.
    pub fn flying(&self) -> bool {
        self.creative_flight || self.fast_flight || self.elytra
    }

    /// Gliding on an elytra.
    pub fn gliding(&self) -> bool {
        self.elytra
    }

    /// Off the ground, out of every fluid, not flying and not on a ladder:
    /// the state a jump or a fall animation answers to.
    pub fn airborne(&self) -> bool {
        !self.grounded && !self.in_fluid() && !self.flying() && !self.climbing
    }

    /// The horizontal input as `(forward, left)`, each clamped to
    /// `-1.0..=1.0`.
    pub fn input(&self) -> (f32, f32) {
        (clamp_axis(self.walk_forward), clamp_axis(self.walk_left))
    }

    /// The length of the input vector, capped at `1.0`: `1.0` for a full
    /// walk in any direction, `0.0` when standing still.
    pub fn locomotion(&self) -> f32 {
        let (forward, left) = self.input();
        (forward * forward + left * left).sqrt().min(1.0)
    }

    /// The action bits of the discrete actions this snapshot reports.
    pub fn actions(&self) -> u32 {
        let mut bits = 0;
        if self.mining {
            bits |= action::MINING;
        }
        if self.charging_crossbow {
            bits |= action::CHARGING_CROSSBOW;
        }
        if self.charging_spear {
            bits |= action::CHARGING_SPEAR;
        }
        if self.charging_trident {
            bits |= action::CHARGING_TRIDENT;
        }
        if self.map_view {
            bits |= action::MAP_VIEW;
        }
        if self.music_nearby {
            bits |= action::MUSIC_NEARBY;
        }
        bits
    }

    /// Whether any discrete action is in progress.
    pub fn is_acting(&self) -> bool {
        self.actions() != 0
    }

    /// Whether either hand is busy with a charge that an arm pose overrides.
    pub fn is_charging(&self) -> bool {
        self.charging_crossbow || self.charging_spear || self.charging_trident
    }
}

/// Clamps one input axis into `-1.0..=1.0`; NaN becomes `0.0`.
fn clamp_axis(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-1.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Gait phase
// ---------------------------------------------------------------------------

/// Wraps an angle in degrees into `0.0..360.0`. A non-finite angle wraps to
/// `0.0` so a broken input can never poison the phase.
pub fn wrap_degrees(degrees: f32) -> f32 {
    wrap_turn(degrees, 360.0)
}

/// Wraps an angle in degrees into `0.0..turn`, where `turn` is the full
/// revolution (`360.0` for a normal circle).
pub fn wrap_turn(degrees: f32, turn: f32) -> f32 {
    if !degrees.is_finite() || !turn.is_finite() || turn <= 0.0 {
        return 0.0;
    }
    let mut wrapped = degrees % turn;
    if wrapped < 0.0 {
        wrapped += turn;
    }
    // `-0.0 % turn` is `-0.0`; fold it so the range really is `0.0..turn`.
    if wrapped == 0.0 { 0.0 } else { wrapped }
}

/// The signed gap from `from` to `to` in degrees, taking the short way round:
/// always in `-180.0..=180.0`.
pub fn shortest_delta(from: f32, to: f32) -> f32 {
    let turn = 360.0;
    let mut delta = wrap_degrees(to) - wrap_degrees(from);
    if delta > turn * 0.5 {
        delta -= turn;
    } else if delta < -turn * 0.5 {
        delta += turn;
    }
    delta
}

/// The value equivalent to `target` that lies closest to `reference`:
/// `target + k * 360` for the smallest `|k|`.
pub fn nearest_equivalent(target: f32, reference: f32) -> f32 {
    reference + shortest_delta(reference, target)
}

/// Blends between two phases the short way, `t` in `0.0..=1.0`, and wraps the
/// result.
pub fn lerp_phase(from: f32, to: f32, t: f32) -> f32 {
    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    wrap_degrees(from + shortest_delta(from, to) * t)
}

/// The walk-cycle phase of a moving player, in degrees.
///
/// The phase advances one full turn per stride: `stride_distance` blocks of
/// travel is one complete left-right cycle of the legs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitPhaseMatcher {
    /// The current phase, always in `0.0..360.0`.
    phase: f32,
    /// How far a candidate phase may be from the current one, in degrees, and
    /// still count as a match. Never negative.
    tolerance: f32,
}

impl GaitPhaseMatcher {
    /// The tolerance a fresh matcher starts with, in degrees: a quarter turn,
    /// so any clip whose stance lands within a quarter cycle still matches.
    pub const DEFAULT_TOLERANCE: f32 = 90.0;

    /// A matcher at phase zero with [`GaitPhaseMatcher::DEFAULT_TOLERANCE`].
    pub fn new() -> Self {
        Self::default()
    }

    /// A matcher at phase zero with an explicit tolerance.
    pub fn with_tolerance(tolerance: f32) -> Self {
        let mut matcher = Self::new();
        matcher.set_tolerance(tolerance);
        matcher
    }

    /// The current phase in `0.0..360.0`.
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Sets the phase, wrapped into `0.0..360.0`.
    pub fn set_phase(&mut self, degrees: f32) -> f32 {
        self.phase = wrap_degrees(degrees);
        self.phase
    }

    /// The match tolerance in degrees.
    pub fn tolerance(&self) -> f32 {
        self.tolerance
    }

    /// Sets the match tolerance; a negative or non-finite value becomes `0.0`.
    pub fn set_tolerance(&mut self, degrees: f32) {
        self.tolerance = if degrees.is_finite() {
            degrees.max(0.0)
        } else {
            0.0
        };
    }

    /// Returns the matcher to phase zero, keeping its tolerance.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Advances the phase by `distance` blocks of travel over a stride of
    /// `stride_distance` blocks, and returns the new phase.
    ///
    /// A stride of zero or less, a NaN distance or an infinite one leaves the
    /// phase untouched.
    pub fn advance(&mut self, distance: f32, stride_distance: f32) -> f32 {
        if !distance.is_finite() || !stride_distance.is_finite() || stride_distance <= 0.0 {
            return self.phase;
        }
        self.phase = wrap_degrees(self.phase + distance / stride_distance * 360.0);
        self.phase
    }

    /// Advances the phase from a speed in blocks per second over `dt` seconds.
    pub fn advance_at_speed(&mut self, speed: f32, dt: f32, stride_distance: f32) -> f32 {
        if !speed.is_finite() || !dt.is_finite() {
            return self.phase;
        }
        self.advance(speed * dt, stride_distance)
    }

    /// The absolute angular distance between the current phase and
    /// `candidate`, in `0.0..=180.0`.
    pub fn distance_to(&self, candidate: f32) -> f32 {
        shortest_delta(self.phase, candidate).abs()
    }

    /// Whether `candidate` lies within the tolerance of the current phase,
    /// measured the short way round.
    pub fn matches(&self, candidate: f32) -> bool {
        self.distance_to(candidate) <= self.tolerance
    }

    /// The index of the candidate phase nearest the current one, or `None`
    /// when `candidates` is empty.
    pub fn best_match(&self, candidates: &[f32]) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (index, candidate) in candidates.iter().enumerate() {
            let distance = self.distance_to(*candidate);
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, _)| index)
    }

    /// The phase equivalent to `target` nearest the current one, unwrapped so
    /// a blend from [`GaitPhaseMatcher::phase`] never jumps a full turn.
    pub fn unwrapped_towards(&self, target: f32) -> f32 {
        nearest_equivalent(target, self.phase)
    }

    /// Steps the phase towards `target` the short way by at most `max_step`
    /// degrees, and returns the new phase.
    pub fn blend_towards(&mut self, target: f32, max_step: f32) -> f32 {
        let limit = if max_step.is_finite() {
            max_step.abs()
        } else {
            0.0
        };
        let delta = shortest_delta(self.phase, target).clamp(-limit, limit);
        self.phase = wrap_degrees(self.phase + delta);
        self.phase
    }
}

impl Default for GaitPhaseMatcher {
    fn default() -> Self {
        Self {
            phase: 0.0,
            tolerance: Self::DEFAULT_TOLERANCE,
        }
    }
}

// ---------------------------------------------------------------------------
// BasePoseRetention
// ---------------------------------------------------------------------------

/// How much of the base (vanilla) pose each bone keeps, in `0.0..=1.0`.
///
/// `0.0` lets the animation own the bone completely, `1.0` leaves the vanilla
/// pose in place; in between the two are blended. Bones missing from the set
/// keep nothing (`0.0`), so a fresh set is empty rather than full.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BasePoseRetention {
    /// Bone name to weight, in the order the bones were first set.
    bones: Vec<(String, f32)>,
}

impl BasePoseRetention {
    /// Every player-model bone the mod can retain, in a stable order.
    pub const BONES: [&'static str; 14] = [
        "head",
        "hat",
        "body",
        "jacket",
        "right_arm",
        "right_sleeve",
        "left_arm",
        "left_sleeve",
        "right_leg",
        "right_pants",
        "left_leg",
        "left_pants",
        "cloak",
        "ear",
    ];

    /// The bones a seated or hand-held upper-body animation leaves alone.
    pub const UPPER_BODY: [&'static str; 9] = [
        "head",
        "hat",
        "body",
        "jacket",
        "right_arm",
        "right_sleeve",
        "left_arm",
        "left_sleeve",
        "cloak",
    ];

    /// An empty set: every bone gives up the base pose.
    pub fn new() -> Self {
        Self::default()
    }

    /// A set holding `bones` at `weight`, clamped into `0.0..=1.0`.
    pub fn from_bones(bones: &[&str], weight: f32) -> Self {
        let mut retention = Self::new();
        for bone in bones {
            retention.set(bone, weight);
        }
        retention
    }

    /// Every bone of [`BasePoseRetention::BONES`] at full weight.
    pub fn whole_body() -> Self {
        Self::from_bones(&Self::BONES, 1.0)
    }

    /// The bones of [`BasePoseRetention::UPPER_BODY`] at full weight.
    pub fn upper_body() -> Self {
        Self::from_bones(&Self::UPPER_BODY, 1.0)
    }

    /// The preset with this name, or `None` when the name is unknown.
    pub fn preset(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "bones" | "whole_body" | "whole-body" | "all" => Some(Self::whole_body()),
            "upper_body" | "upper-body" | "upper" => Some(Self::upper_body()),
            "none" | "empty" => Some(Self::new()),
            _ => None,
        }
    }

    /// The weight this bone keeps, or `0.0` when the bone is absent.
    pub fn get(&self, bone: &str) -> f32 {
        self.bones
            .iter()
            .find(|(name, _)| name == bone)
            .map(|(_, weight)| *weight)
            .unwrap_or(0.0)
    }

    /// Sets a bone's weight, clamped into `0.0..=1.0` (NaN becomes `0.0`),
    /// and returns the stored weight.
    pub fn set(&mut self, bone: &str, weight: f32) -> f32 {
        let weight = if weight.is_nan() {
            0.0
        } else {
            weight.clamp(0.0, 1.0)
        };
        match self.bones.iter_mut().find(|(name, _)| name == bone) {
            Some((_, stored)) => *stored = weight,
            None => self.bones.push((bone.to_string(), weight)),
        }
        weight
    }

    /// Removes a bone, returning the weight it had.
    pub fn remove(&mut self, bone: &str) -> Option<f32> {
        let position = self.bones.iter().position(|(name, _)| name == bone)?;
        Some(self.bones.remove(position).1)
    }

    /// The bones and their weights, in insertion order.
    pub fn bones(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bones
            .iter()
            .map(|(name, weight)| (name.as_str(), *weight))
    }

    /// How many bones the set mentions.
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    /// Whether the set mentions no bone at all.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    /// This set with `overlay` laid over it: a bone in both keeps the
    /// overlay's weight, a bone only in the overlay is added, and the order of
    /// this set is preserved with the new bones appended.
    pub fn over(&self, overlay: &BasePoseRetention) -> BasePoseRetention {
        let mut merged = self.clone();
        for (bone, weight) in overlay.bones() {
            merged.set(bone, weight);
        }
        merged
    }

    /// Reads retention from JSON. Accepted forms:
    ///
    /// - a preset name: `"bones"`, `"upper_body"` or `"none"`;
    /// - a bare object of bone weights: `{"head": 1.0, "body": 0.5}`;
    /// - a preset with overrides:
    ///   `{"preset": "upper_body", "bones": {"head": 0.25}}`.
    ///
    /// An unknown preset or a non-numeric weight is an error.
    pub fn parse(value: &Value) -> Result<Self, String> {
        if let Some(name) = value.as_str() {
            return Self::preset(name)
                .ok_or_else(|| format!("unknown base-pose retention preset {:?}", name));
        }
        if !value.is_object() {
            return Err("base-pose retention must be a preset name or an object".to_string());
        }
        let mut retention = match value.get("preset").and_then(Value::as_str) {
            Some(name) => Self::preset(name)
                .ok_or_else(|| format!("unknown base-pose retention preset {:?}", name))?,
            None => Self::new(),
        };
        if let Some(weights) = value.get("bones") {
            retention.apply_weights(weights)?;
        } else if !value.has("preset") {
            retention.apply_weights(value)?;
        }
        Ok(retention)
    }

    /// Applies an object of bone weights onto this set.
    fn apply_weights(&mut self, value: &Value) -> Result<(), String> {
        let members = value
            .as_object()
            .ok_or_else(|| "base-pose retention weights must be an object".to_string())?;
        for (bone, weight) in members {
            let number = weight
                .as_f32()
                .ok_or_else(|| format!("base-pose weight for {:?} must be a number", bone))?;
            self.set(bone, number);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MovementTuning
// ---------------------------------------------------------------------------

/// The sprint direction weights: how much of the gait survives a sprint in
/// each direction. `1.0` keeps the walk swing, `0.0` flattens it out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SprintWeights {
    /// Sprinting straight ahead.
    pub forward: f32,
    /// Sprinting backwards.
    pub back: f32,
    /// Sprinting to the left.
    pub left: f32,
    /// Sprinting to the right.
    pub right: f32,
}

impl SprintWeights {
    /// The weight for an input direction, `forward` and `left` in
    /// `-1.0..=1.0`: pure input picks one weight, a diagonal blends the two in
    /// proportion to the input's components. No input keeps the forward
    /// weight.
    pub fn weight(&self, forward: f32, left: f32) -> f32 {
        let forward = clamp_axis(forward);
        let left = clamp_axis(left);
        let forward_weight = if forward < 0.0 {
            self.back
        } else {
            self.forward
        };
        let side_weight = if left < 0.0 { self.right } else { self.left };
        let forward_share = forward.abs();
        let side_share = left.abs();
        let total = forward_share + side_share;
        if total <= f32::EPSILON {
            return self.forward;
        }
        let blended = (forward_weight * forward_share + side_weight * side_share) / total;
        if blended.is_nan() { 0.0 } else { blended }
    }
}

impl Default for SprintWeights {
    fn default() -> Self {
        Self {
            forward: 1.0,
            back: 0.7,
            left: 0.85,
            right: 0.85,
        }
    }
}

/// The numbers the animation layer is tuned with: one stride's length, the
/// swing and landing amplitudes, how quickly weights fade, how much of the
/// short way a phase step closes, the turn used for wrapping, and the sprint
/// direction weights.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementTuning {
    /// Blocks of travel per full gait cycle.
    pub stride_distance: f32,
    /// Swing scale of the gait at walking speed.
    pub gait_amplitude: f32,
    /// Dip scale when a landing plays.
    pub landing_amplitude: f32,
    /// Weight change per second when easing between poses.
    pub fade_rate: f32,
    /// The share (`0.0..=1.0`) of the shortest angular gap one phase step
    /// closes.
    pub short_way: f32,
    /// The full turn used for wrapping, in degrees.
    pub wrap_degrees: f32,
    /// The weights a sprint keeps per direction.
    pub sprint: SprintWeights,
}

impl Default for MovementTuning {
    fn default() -> Self {
        Self {
            stride_distance: 2.0,
            gait_amplitude: 1.0,
            landing_amplitude: 1.0,
            fade_rate: 6.0,
            short_way: 0.35,
            wrap_degrees: 360.0,
            sprint: SprintWeights::default(),
        }
    }
}

impl MovementTuning {
    /// The default tuning.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wraps `degrees` into `0.0..wrap_degrees`; a bad turn or a non-finite
    /// angle wraps to `0.0`.
    pub fn wrap(&self, degrees: f32) -> f32 {
        wrap_turn(degrees, self.wrap_degrees)
    }

    /// The signed gap from `from` to `to`, the short way round, always in
    /// `-turn/2..=turn/2`.
    pub fn shortest_delta(&self, from: f32, to: f32) -> f32 {
        let turn = self.wrap_degrees;
        if !turn.is_finite() || turn <= 0.0 {
            return 0.0;
        }
        let mut delta = self.wrap(to) - self.wrap(from);
        if delta > turn * 0.5 {
            delta -= turn;
        } else if delta < -turn * 0.5 {
            delta += turn;
        }
        delta
    }

    /// The gait phase after `distance` blocks of travel, in `0.0..turn`.
    /// A stride of zero or less leaves the phase at `0.0`.
    pub fn gait_phase(&self, distance: f32) -> f32 {
        if !distance.is_finite() || self.stride_distance <= 0.0 {
            return 0.0;
        }
        self.wrap(distance / self.stride_distance * self.wrap_degrees)
    }

    /// The phase advanced from a speed in blocks per second over `dt` seconds.
    pub fn gait_phase_at(&self, speed: f32, dt: f32) -> f32 {
        if !speed.is_finite() || !dt.is_finite() {
            return 0.0;
        }
        self.gait_phase(speed * dt)
    }

    /// Moves `current` towards `target` by at most `fade_rate * dt`, stopping
    /// exactly on the target.
    pub fn fade(&self, current: f32, target: f32, dt: f32) -> f32 {
        let rate = if self.fade_rate.is_finite() {
            self.fade_rate.max(0.0)
        } else {
            0.0
        };
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let step = rate * dt;
        let gap = target - current;
        if !gap.is_finite() {
            return target;
        }
        if gap.abs() <= step {
            target
        } else {
            current + step * gap.signum()
        }
    }

    /// One phase step towards `to` from `from`, closing `short_way` of the
    /// shortest gap and wrapping the result.
    pub fn short_way_step(&self, from: f32, to: f32) -> f32 {
        let share = if self.short_way.is_finite() {
            self.short_way.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.wrap(from + self.shortest_delta(from, to) * share)
    }

    /// The gait amplitude for an input direction: [`MovementTuning::gait_amplitude`]
    /// while walking, scaled by the sprint weight while sprinting.
    pub fn stride_amplitude(&self, forward: f32, left: f32, sprinting: bool) -> f32 {
        let scale = if sprinting {
            self.sprint.weight(forward, left)
        } else {
            1.0
        };
        let amplitude = self.gait_amplitude * scale;
        if amplitude.is_nan() { 0.0 } else { amplitude }
    }

    /// The landing dip for a fall of `fall_distance` blocks: the landing
    /// amplitude scaled by the fall's weight.
    pub fn landing_dip(&self, fall_distance: f32) -> f32 {
        let dip = self.landing_amplitude * FallWeight::from_distance(fall_distance).get();
        if dip.is_nan() { 0.0 } else { dip }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn value(text: &str) -> Value {
        parse(text).unwrap_or_else(|e| panic!("parse failed: {}", e))
    }

    #[test]
    fn a_fresh_snapshot_is_inert() {
        let signals = MotionSignals::new();
        assert_eq!(signals.tick, 0);
        assert!(!signals.grounded);
        assert_eq!(signals.fall_weight, 0.0);
        assert_eq!(signals.input(), (0.0, 0.0));
        assert_eq!(signals.locomotion(), 0.0);
        assert_eq!(signals.actions(), 0);
        assert!(!signals.is_acting());
        assert!(!signals.in_fluid());
        assert!(!signals.swimming());
        assert!(!signals.flying());
        // Neither in the air nor anywhere else: a resting snapshot is grounded
        // only once the game says so.
        assert!(signals.airborne());
    }

    #[test]
    fn predicates_read_the_state_they_name() {
        let mut signals = MotionSignals {
            grounded: true,
            water_below: true,
            ..MotionSignals::new()
        };
        assert!(!signals.airborne());
        assert!(signals.in_fluid());

        signals.water_below = false;
        signals.submerged_swimming = true;
        assert!(signals.swimming());
        assert!(!signals.airborne());

        signals.submerged_swimming = false;
        signals.grounded = false;
        signals.elytra = true;
        assert!(signals.flying());
        assert!(signals.gliding());
        assert!(!signals.airborne());

        signals.elytra = false;
        signals.climbing = true;
        assert!(!signals.airborne());

        signals.climbing = false;
        assert!(signals.airborne());
    }

    #[test]
    fn actions_mask_every_flag_once() {
        let mut signals = MotionSignals {
            mining: true,
            map_view: true,
            ..MotionSignals::new()
        };
        assert_eq!(signals.actions(), action::MINING | action::MAP_VIEW);
        assert!(signals.is_acting());
        assert!(!signals.is_charging());

        signals.charging_crossbow = true;
        assert!(signals.is_charging());
        assert_eq!(
            signals.actions() & action::CHARGING_CROSSBOW,
            action::CHARGING_CROSSBOW
        );

        // Every action bit is distinct and inside `ALL`.
        let all = [
            action::MINING,
            action::CHARGING_CROSSBOW,
            action::CHARGING_SPEAR,
            action::CHARGING_TRIDENT,
            action::MAP_VIEW,
            action::MUSIC_NEARBY,
        ];
        let mut union = 0;
        for bit in all {
            assert_eq!(union & bit, 0, "bit {:x} repeats", bit);
            union |= bit;
        }
        assert_eq!(union, action::ALL);

        let full = MotionSignals {
            mining: true,
            charging_crossbow: true,
            charging_spear: true,
            charging_trident: true,
            map_view: true,
            music_nearby: true,
            ..MotionSignals::new()
        };
        assert_eq!(full.actions(), action::ALL);
    }

    #[test]
    fn input_and_locomotion_stay_in_range() {
        let signals = MotionSignals {
            walk_forward: 1.0,
            walk_left: 0.0,
            ..MotionSignals::new()
        };
        assert_eq!(signals.locomotion(), 1.0);

        let diagonal = MotionSignals {
            walk_forward: 1.0,
            walk_left: 1.0,
            ..MotionSignals::new()
        };
        assert_eq!(diagonal.locomotion(), 1.0);

        let tiny = MotionSignals {
            walk_forward: 0.6,
            walk_left: 0.8,
            ..MotionSignals::new()
        };
        assert!((tiny.locomotion() - 1.0).abs() < 1e-6);

        let out_of_range = MotionSignals {
            walk_forward: 4.0,
            walk_left: f32::NAN,
            ..MotionSignals::new()
        };
        assert_eq!(out_of_range.input(), (1.0, 0.0));
    }

    #[test]
    fn fall_weight_clamps_and_ramps_with_distance() {
        assert_eq!(FallWeight::new(-1.0), FallWeight::NONE);
        assert_eq!(FallWeight::new(2.0), FallWeight::LANDING);
        assert_eq!(FallWeight::new(f32::NAN), FallWeight::NONE);
        assert!(FallWeight::NONE.is_zero());

        // Nothing below the onset, full at the full distance, linear between.
        assert_eq!(FallWeight::from_distance(0.0), FallWeight::NONE);
        assert_eq!(FallWeight::from_distance(-3.0), FallWeight::NONE);
        assert_eq!(FallWeight::from_distance(f32::NAN), FallWeight::NONE);
        assert_eq!(FallWeight::from_distance(4.0), FallWeight::LANDING);
        assert_eq!(FallWeight::from_distance(40.0), FallWeight::LANDING);

        let half = FallWeight::from_distance(2.25).get();
        assert!((half - 0.5).abs() < 1e-6, "half weight was {}", half);

        // Monotonic in the distance.
        let mut previous = 0.0;
        let mut distance = 0.0;
        while distance <= 5.0 {
            let weight = FallWeight::from_distance(distance).get();
            assert!(weight >= previous, "weight fell at {}", distance);
            previous = weight;
            distance += 0.05;
        }
    }

    #[test]
    fn degrees_wrap_into_one_turn() {
        assert_eq!(wrap_degrees(0.0), 0.0);
        assert_eq!(wrap_degrees(360.0), 0.0);
        assert_eq!(wrap_degrees(450.0), 90.0);
        assert_eq!(wrap_degrees(-90.0), 270.0);
        assert_eq!(wrap_degrees(-720.0), 0.0);
        assert_eq!(wrap_degrees(f32::NAN), 0.0);
        assert_eq!(wrap_degrees(f32::INFINITY), 0.0);
        // A custom turn.
        assert_eq!(wrap_turn(7.0, 4.0), 3.0);
        assert_eq!(wrap_turn(-1.0, 4.0), 3.0);
        assert_eq!(wrap_turn(2.0, 0.0), 0.0);
    }

    #[test]
    fn shortest_delta_takes_the_short_way() {
        assert_eq!(shortest_delta(10.0, 20.0), 10.0);
        assert_eq!(shortest_delta(20.0, 10.0), -10.0);
        // Across the wrap: 350 -> 10 is +20, not -340.
        assert_eq!(shortest_delta(350.0, 10.0), 20.0);
        assert_eq!(shortest_delta(10.0, 350.0), -20.0);
        // Exactly half a turn is a full turn back, so the sign stays `-`.
        assert_eq!(shortest_delta(0.0, 180.0), 180.0);
        assert_eq!(shortest_delta(180.0, 0.0), -180.0);
        assert!(shortest_delta(0.0, 90.0).abs() <= 180.0);

        // The equivalent nearest a reference, unwrapped.
        assert_eq!(nearest_equivalent(10.0, 350.0), 370.0);
        assert_eq!(nearest_equivalent(350.0, 10.0), -10.0);
        assert_eq!(nearest_equivalent(45.0, 0.0), 45.0);

        // A short-way blend never crosses the far side.
        assert_eq!(lerp_phase(350.0, 10.0, 0.5), 0.0);
        assert_eq!(lerp_phase(0.0, 90.0, 0.5), 45.0);
        assert_eq!(lerp_phase(0.0, 90.0, 4.0), 90.0);
    }

    #[test]
    fn gait_phase_advances_one_turn_per_stride() {
        let mut matcher = GaitPhaseMatcher::new();
        assert_eq!(matcher.phase(), 0.0);

        // Half a stride is half a turn.
        assert_eq!(matcher.advance(1.0, 2.0), 180.0);
        // Another half brings it back to zero.
        assert_eq!(matcher.advance(1.0, 2.0), 0.0);
        // Three strides wrap twice and land on zero again.
        assert_eq!(matcher.advance(6.0, 2.0), 0.0);

        // A quarter stride is a quarter turn, and negative travel runs the
        // cycle backwards.
        matcher.reset();
        assert_eq!(matcher.advance(0.5, 2.0), 90.0);
        assert_eq!(matcher.advance(-1.5, 2.0), 180.0);

        // Speed and time are the same thing.
        matcher.reset();
        assert_eq!(matcher.advance_at_speed(2.0, 0.25, 2.0), 90.0);
    }

    #[test]
    fn a_bad_stride_leaves_the_phase_alone() {
        let mut matcher = GaitPhaseMatcher::new();
        matcher.set_phase(45.0);
        assert_eq!(matcher.advance(4.0, 0.0), 45.0);
        assert_eq!(matcher.advance(4.0, -2.0), 45.0);
        assert_eq!(matcher.advance(f32::NAN, 2.0), 45.0);
        assert_eq!(matcher.advance(1.0, f32::INFINITY), 45.0);
        assert_eq!(matcher.advance_at_speed(f32::NAN, 1.0, 2.0), 45.0);
    }

    #[test]
    fn phase_matching_uses_the_short_way_and_the_tolerance() {
        let mut matcher = GaitPhaseMatcher::with_tolerance(20.0);
        matcher.set_phase(350.0);

        // 10 degrees away the short way, not 340 the long way.
        assert_eq!(matcher.distance_to(10.0), 20.0);
        assert_eq!(matcher.distance_to(340.0), 10.0);
        assert!(matcher.matches(10.0));
        assert!(matcher.matches(340.0));

        matcher.set_tolerance(5.0);
        assert!(!matcher.matches(10.0));
        assert!(matcher.matches(352.0));

        // A negative tolerance clamps to zero, so only an exact phase matches.
        matcher.set_tolerance(-30.0);
        assert_eq!(matcher.tolerance(), 0.0);
        assert!(!matcher.matches(350.0 + 0.5));

        // Tolerance survives a reset; the phase does not.
        matcher.set_tolerance(30.0);
        matcher.reset();
        assert_eq!(matcher.phase(), 0.0);
        assert_eq!(matcher.tolerance(), 30.0);
    }

    #[test]
    fn best_match_picks_the_nearest_candidate() {
        let mut matcher = GaitPhaseMatcher::new();
        matcher.set_phase(100.0);
        assert_eq!(matcher.best_match(&[10.0, 95.0, 300.0]), Some(1));
        assert_eq!(matcher.best_match(&[]), None);
        // A candidate on the far side of the wrap is nearest via the short way.
        matcher.set_phase(355.0);
        assert_eq!(matcher.best_match(&[180.0, 5.0]), Some(1));
    }

    #[test]
    fn blending_steps_at_most_the_max_step_the_short_way() {
        let mut matcher = GaitPhaseMatcher::new();
        matcher.set_phase(350.0);
        assert_eq!(matcher.blend_towards(10.0, 5.0), 355.0);
        assert_eq!(matcher.blend_towards(10.0, 5.0), 0.0);
        // The last step lands exactly on the target when the step is large.
        assert_eq!(matcher.blend_towards(90.0, 500.0), 90.0);
        // A negative step is treated as its magnitude; NaN as zero.
        matcher.set_phase(10.0);
        assert_eq!(matcher.blend_towards(40.0, -10.0), 20.0);
        assert_eq!(matcher.blend_towards(40.0, f32::NAN), 20.0);

        // The unwrapped target is the equivalent nearest the current phase.
        matcher.set_phase(350.0);
        assert_eq!(matcher.unwrapped_towards(10.0), 370.0);
    }

    #[test]
    fn retention_presets_hold_the_bones_they_name() {
        let whole = BasePoseRetention::whole_body();
        assert_eq!(whole.len(), BasePoseRetention::BONES.len());
        for bone in BasePoseRetention::BONES {
            assert_eq!(whole.get(bone), 1.0);
        }

        let upper = BasePoseRetention::upper_body();
        assert_eq!(upper.len(), BasePoseRetention::UPPER_BODY.len());
        assert_eq!(upper.get("head"), 1.0);
        assert_eq!(upper.get("left_leg"), 0.0);

        // The upper body is a subset of the whole body, and a fresh set keeps
        // nothing.
        for bone in BasePoseRetention::UPPER_BODY {
            assert!(
                BasePoseRetention::BONES.contains(&bone),
                "{} is not a bone",
                bone
            );
        }
        assert!(BasePoseRetention::new().is_empty());
        assert_eq!(BasePoseRetention::new().get("head"), 0.0);
    }

    #[test]
    fn retention_weights_clamp_and_stay_in_insertion_order() {
        let mut retention = BasePoseRetention::new();
        assert_eq!(retention.set("body", 0.5), 0.5);
        assert_eq!(retention.set("head", 1.5), 1.0);
        assert_eq!(retention.set("left_leg", -0.5), 0.0);
        assert_eq!(retention.set("cloak", f32::NAN), 0.0);
        assert_eq!(retention.len(), 4);

        let names: Vec<&str> = retention.bones().map(|(bone, _)| bone).collect();
        assert_eq!(names, ["body", "head", "left_leg", "cloak"]);

        // Setting an existing bone keeps its position.
        retention.set("body", 0.25);
        assert_eq!(retention.get("body"), 0.25);
        assert_eq!(retention.len(), 4);

        assert_eq!(retention.remove("head"), Some(1.0));
        assert_eq!(retention.remove("head"), None);
        assert_eq!(retention.len(), 3);
        assert_eq!(retention.get("head"), 0.0);
    }

    #[test]
    fn retention_parses_presets_weights_and_overrides() {
        let preset = BasePoseRetention::parse(&value("\"upper_body\"")).unwrap();
        assert_eq!(preset, BasePoseRetention::upper_body());

        let whole = BasePoseRetention::parse(&value("\"bones\"")).unwrap();
        assert_eq!(whole, BasePoseRetention::whole_body());

        let none = BasePoseRetention::parse(&value("\"none\"")).unwrap();
        assert!(none.is_empty());

        let bare = BasePoseRetention::parse(&value("{\"head\":1.0,\"body\":0.5}")).unwrap();
        assert_eq!(bare.get("head"), 1.0);
        assert_eq!(bare.get("body"), 0.5);
        assert_eq!(bare.len(), 2);

        // A preset with an override keeps the rest of the preset.
        let tuned = BasePoseRetention::parse(&value(
            "{\"preset\":\"upper_body\",\"bones\":{\"head\":0.25}}",
        ))
        .unwrap();
        assert_eq!(tuned.get("head"), 0.25);
        assert_eq!(tuned.get("left_arm"), 1.0);
        assert_eq!(tuned.get("left_leg"), 0.0);

        // Out-of-range weights clamp; unknown presets and bad weights fail.
        let clamped = BasePoseRetention::parse(&value("\"bones\"")).unwrap();
        assert!(clamped.get("ear") <= 1.0);
        assert!(BasePoseRetention::parse(&value("\"every_bone\"")).is_err());
        assert!(BasePoseRetention::parse(&value("[]")).is_err());
        assert!(BasePoseRetention::parse(&value("{\"head\":\"lots\"}")).is_err());
        assert!(BasePoseRetention::parse(&value("{\"bones\":7}")).is_err());
    }

    #[test]
    fn over_merges_with_the_overlay_winning() {
        let base = BasePoseRetention::from_bones(&["head", "body"], 0.5);
        let overlay = BasePoseRetention::from_bones(&["body", "cloak"], 1.0);
        let merged = base.over(&overlay);

        assert_eq!(merged.get("head"), 0.5);
        assert_eq!(merged.get("body"), 1.0);
        assert_eq!(merged.get("cloak"), 1.0);
        assert_eq!(merged.len(), 3);

        // The order is the base's, then the overlay's new bones.
        let names: Vec<&str> = merged.bones().map(|(bone, _)| bone).collect();
        assert_eq!(names, ["head", "body", "cloak"]);

        // Neither input is changed, and merging nothing changes nothing.
        assert_eq!(base.get("body"), 0.5);
        assert_eq!(overlay.len(), 2);
        assert_eq!(base.over(&BasePoseRetention::new()), base);
    }

    #[test]
    fn movement_tuning_defaults_are_usable() {
        let tuning = MovementTuning::new();
        assert!(tuning.stride_distance > 0.0);
        assert!(tuning.gait_amplitude > 0.0);
        assert!(tuning.landing_amplitude > 0.0);
        assert!(tuning.fade_rate > 0.0);
        assert!((0.0..=1.0).contains(&tuning.short_way));
        assert_eq!(tuning.wrap_degrees, 360.0);
        assert_eq!(tuning.sprint.forward, 1.0);
    }

    #[test]
    fn tuning_wraps_and_measures_the_short_way() {
        let tuning = MovementTuning::new();
        assert_eq!(tuning.wrap(370.0), 10.0);
        assert_eq!(tuning.wrap(-10.0), 350.0);
        assert_eq!(tuning.shortest_delta(350.0, 10.0), 20.0);
        assert_eq!(tuning.shortest_delta(10.0, 350.0), -20.0);

        // A custom turn wraps and measures against itself.
        let quarter = MovementTuning {
            wrap_degrees: 4.0,
            ..MovementTuning::new()
        };
        assert_eq!(quarter.wrap(5.0), 1.0);
        assert_eq!(quarter.shortest_delta(0.0, 3.0), -1.0);

        // A broken turn yields no gap at all.
        let broken = MovementTuning {
            wrap_degrees: 0.0,
            ..MovementTuning::new()
        };
        assert_eq!(broken.wrap(90.0), 0.0);
        assert_eq!(broken.shortest_delta(0.0, 90.0), 0.0);
    }

    #[test]
    fn tuning_phase_and_amplitude_follow_the_numbers() {
        let tuning = MovementTuning::default();
        assert_eq!(tuning.gait_phase(0.0), 0.0);
        assert_eq!(tuning.gait_phase(1.0), 180.0);
        assert_eq!(tuning.gait_phase(2.0), 0.0);
        assert_eq!(tuning.gait_phase_at(2.0, 0.25), 90.0);
        assert_eq!(tuning.gait_phase(f32::NAN), 0.0);

        // A walk keeps the amplitude; a sprint scales it by the direction.
        assert_eq!(
            tuning.stride_amplitude(1.0, 0.0, false),
            tuning.gait_amplitude
        );
        assert_eq!(
            tuning.stride_amplitude(1.0, 0.0, true),
            tuning.gait_amplitude * tuning.sprint.forward
        );
        assert_eq!(
            tuning.stride_amplitude(0.0, 1.0, true),
            tuning.gait_amplitude * tuning.sprint.left
        );

        // A fall only dips above the onset and at most by the landing scale.
        assert_eq!(tuning.landing_dip(0.0), 0.0);
        assert_eq!(tuning.landing_dip(10.0), tuning.landing_amplitude);
    }

    #[test]
    fn tuning_fades_towards_the_target_and_closes_the_short_way() {
        let tuning = MovementTuning {
            fade_rate: 2.0,
            ..MovementTuning::default()
        };
        assert_eq!(tuning.fade(0.0, 1.0, 0.25), 0.5);
        assert_eq!(tuning.fade(0.0, 1.0, 1.0), 1.0);
        assert_eq!(tuning.fade(1.0, 0.0, 0.1), 0.8);
        // A zero-length step changes nothing; a NaN step is treated as zero.
        assert_eq!(tuning.fade(0.25, 1.0, 0.0), 0.25);
        assert_eq!(tuning.fade(0.25, 1.0, f32::NAN), 0.25);

        // `short_way` is the share of the shortest gap one step closes.
        let step = MovementTuning {
            short_way: 0.5,
            ..MovementTuning::default()
        };
        assert_eq!(step.short_way_step(350.0, 10.0), 0.0);
        assert_eq!(step.short_way_step(0.0, 90.0), 45.0);
        assert_eq!(step.short_way_step(0.0, 720.0), 0.0);
    }

    #[test]
    fn sprint_weights_blend_by_direction() {
        let weights = SprintWeights::default();
        assert_eq!(weights.weight(1.0, 0.0), weights.forward);
        assert_eq!(weights.weight(-1.0, 0.0), weights.back);
        assert_eq!(weights.weight(0.0, 1.0), weights.left);
        assert_eq!(weights.weight(0.0, -1.0), weights.right);
        // Standing still keeps the forward weight.
        assert_eq!(weights.weight(0.0, 0.0), weights.forward);

        // A diagonal is the share-weighted blend of the two.
        let diagonal = weights.weight(1.0, 1.0);
        let expected = (weights.forward + weights.left) * 0.5;
        assert!((diagonal - expected).abs() < 1e-6);

        // Out-of-range input clamps, and NaN counts as still.
        assert_eq!(weights.weight(9.0, 0.0), weights.forward);
        assert_eq!(weights.weight(f32::NAN, f32::NAN), weights.forward);
    }
}

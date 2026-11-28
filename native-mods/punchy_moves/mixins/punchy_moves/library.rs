//! Punchy Moves' animation library, ported from the Fabric mod's
//! `punchymoves.client.animation.AnimationLibrary` and its nested records,
//! plus `CompatFormat` (legacy pack normalization) and
//! `ItemAnimationBindings` (per-item binding resolution).
//!
//! Pure Rust and std-only: vectors and quaternions are local f32
//! implementations whose formulas match JOML 1.10.8 exactly
//! (`rotationZYX`, `mul` = `this * other`, `transform`, `getEulerAnglesZYX`);
//! JSON loading runs on the mod's own `json` module; curves come from
//! `easing` and variable expressions from `expressions`.

use std::collections::HashMap;

use crate::signals::BasePoseRetention;

/// One 3-component f32 vector: the JOML `Vector3f` surface the library uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Vec3 = Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// `lengthSquared`.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// `Vector3f.lerp(other, t)`: `this + (other - this) * t`.
    pub fn lerp(self, other: Vec3, t: f32) -> Vec3 {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// f32 quaternion: the JOML `Quaternionf` surface the library uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    /// `Quaternionf.rotationZYX(angleZ, angleY, angleX)`: rotation about Z,
    /// then Y, then X — the quaternion product `qz * qy * qx`.
    pub fn from_euler_zyx(angle_z_deg: f32, angle_y_deg: f32, angle_x_deg: f32) -> Self {
        let sx = (angle_x_deg.to_radians() * 0.5).sin();
        let cx = (angle_x_deg.to_radians() * 0.5).cos();
        let sy = (angle_y_deg.to_radians() * 0.5).sin();
        let cy = (angle_y_deg.to_radians() * 0.5).cos();
        let sz = (angle_z_deg.to_radians() * 0.5).sin();
        let cz = (angle_z_deg.to_radians() * 0.5).cos();
        let cycz = cy * cz;
        let sysz = sy * sz;
        let sycz = sy * cz;
        let cysz = cy * sz;
        Self {
            x: sx * cycz - cx * sysz,
            y: cx * sycz + sx * cysz,
            z: cx * cysz - sx * sycz,
            w: cx * cycz + sx * sysz,
        }
    }

    /// `Quaternionf.transform(Vector3f)`: rotates a vector by this
    /// quaternion (JOML's normalized formula).
    pub fn transform_by(self, v: Vec3) -> Vec3 {
        let xx = self.x * self.x;
        let yy = self.y * self.y;
        let zz = self.z * self.z;
        let ww = self.w * self.w;
        let xy = self.x * self.y;
        let xz = self.x * self.z;
        let yz = self.y * self.z;
        let xw = self.x * self.w;
        let zw = self.z * self.w;
        let yw = self.y * self.w;
        let k = 1.0 / (xx + yy + zz + ww);
        Vec3 {
            x: (xx - yy - zz + ww) * k * v.x
                + 2.0 * (xy - zw) * k * v.y
                + (2.0 * (xz + yw) * k) * v.z,
            y: 2.0 * (xy + zw) * k * v.x
                + (yy - xx - zz + ww) * k * v.y
                + 2.0 * (yz - xw) * k * v.z,
            z: 2.0 * (xz - yw) * k * v.x
                + 2.0 * (yz + xw) * k * v.y
                + ((zz - xx - yy + ww) * k) * v.z,
        }
    }

    /// `Quaternionf.getEulerAnglesZYX`, radians, with JOML's `safeAsin`
    /// (asin of a clamped argument).
    pub fn get_euler_zyx(self) -> Vec3 {
        let xx = self.x * self.x;
        let yy = self.y * self.y;
        let zz = self.z * self.z;
        Vec3 {
            x: (self.y * self.z + self.w * self.x).atan2(0.5 - xx - yy),
            y: (-2.0 * (self.x * self.z - self.w * self.y))
                .clamp(-1.0, 1.0)
                .asin(),
            z: (self.x * self.y + self.w * self.z).atan2(0.5 - yy - zz),
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Componentwise product, like `Vector3f* Vector3fc`.
impl std::ops::Mul for Vec3 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

/// `Vector3f.mul(float)`: every component times a scalar.
impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f32) -> Vec3 {
        Vec3 {
            x: self.x * k,
            y: self.y * k,
            z: self.z * k,
        }
    }
}

impl std::ops::Mul for Quat {
    type Output = Quat;
    /// Post-multiplication, `R = self * other`: `other` applies first.
    fn mul(self, other: Quat) -> Quat {
        Self {
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        }
    }
}

/// One sampled bone pose: the JOML `Pose` record (position, rotation in
/// degrees, scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Pose {
    pub fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

/// One keyframe: the `Key` record (pre/post vectors, interpolation name,
/// optional easing curve).
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub pre: Vec3,
    pub post: Vec3,
    pub interpolation: String,
    pub easing: Option<crate::easing::Easing>,
}

/// One animated channel: the `Channel` record — a time-ordered list of keys
/// plus the vector returned when the channel has no keys at all. The keys
/// are kept sorted by time, like the Java `TreeMap`.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    keys: Vec<(f32, Key)>,
    fallback: Vec3,
}

impl Channel {
    /// Builds a channel from `(time, key)` pairs; they are sorted by time.
    pub fn new(mut keys: Vec<(f32, Key)>, fallback: Vec3) -> Self {
        keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Self { keys, fallback }
    }

    /// Draws the channel at `t` with no look-behind: the Java `sample(float)`.
    pub fn sample(&self, t: f32) -> Vec3 {
        self.sample_before(t, 0.0)
    }

    /// Draws the channel at `t`. `before` is the clip's total length: when a
    /// Catmull-Rom segment at either end has no neighbouring key, the lookup
    /// may reach that far for the missing control point.
    pub fn sample_before(&self, t: f32, before: f32) -> Vec3 {
        if self.keys.is_empty() {
            return self.fallback;
        }
        let floor = self.floor_idx(t);
        let ceiling = self.ceiling_idx(t);
        let Some(ceil_idx) = ceiling else {
            // After the last key: its `post` value (the floor key is there).
            return match floor {
                Some(i) => self.keys[i].1.post,
                None => self.fallback,
            };
        };
        let Some(floor_idx) = floor else {
            // Before the first key: its `pre` value.
            return self.keys[ceil_idx].1.pre;
        };
        let floor_time = self.keys[floor_idx].0;
        let ceil_time = self.keys[ceil_idx].0;
        if floor_time == ceil_time {
            return self.keys[floor_idx].1.post;
        }
        let mut alpha = (t - floor_time) / (ceil_time - floor_time);
        if let Some(easing) = self.keys[ceil_idx].1.easing {
            alpha = easing.apply(alpha);
        }
        let a = self.keys[floor_idx].1.post;
        let b = self.keys[ceil_idx].1.pre;
        if is_catmull(&self.keys[floor_idx].1) || is_catmull(&self.keys[ceil_idx].1) {
            let mut lower = self.lower_idx(floor_time);
            let mut higher = self.higher_idx(ceil_time);
            if before > 0.0 && lower.is_none() {
                lower = self.lower_idx(before);
            }
            if before > 0.0 && higher.is_none() && self.keys.len() > 2 {
                higher = self.higher_idx(self.keys[0].0);
            }
            let p0 = match lower {
                Some(i) => self.keys[i].1.post,
                None => a,
            };
            let p3 = match higher {
                Some(i) => self.keys[i].1.pre,
                None => b,
            };
            return catmull(p0, a, b, p3, alpha);
        }
        a.lerp(b, alpha)
    }

    /// The last key's time, or 0.0 when the channel has no keys.
    pub fn last_key_time(&self) -> f32 {
        self.keys.last().map_or(0.0, |(t, _)| *t)
    }

    /// Index of the last key at or before `t` (`floorEntry`).
    fn floor_idx(&self, t: f32) -> Option<usize> {
        let n = self.keys.partition_point(|(k, _)| *k <= t);
        if n == 0 { None } else { Some(n - 1) }
    }

    /// Index of the first key at or after `t` (`ceilingEntry`).
    fn ceiling_idx(&self, t: f32) -> Option<usize> {
        let n = self.keys.partition_point(|(k, _)| *k < t);
        if n == self.keys.len() { None } else { Some(n) }
    }

    /// Index of the last key strictly before `t` (`lowerEntry`).
    fn lower_idx(&self, t: f32) -> Option<usize> {
        let n = self.keys.partition_point(|(k, _)| *k < t);
        if n == 0 { None } else { Some(n - 1) }
    }

    /// Index of the first key strictly after `t` (`higherEntry`).
    fn higher_idx(&self, t: f32) -> Option<usize> {
        let n = self.keys.partition_point(|(k, _)| *k <= t);
        if n == self.keys.len() { None } else { Some(n) }
    }
}

/// `catmullrom` interpolation, matched case-insensitively.
fn is_catmull(key: &Key) -> bool {
    key.interpolation.eq_ignore_ascii_case("catmullrom")
}

/// Catmull-Rom spline through `p1 -> p2` with control points `p0`/`p3`.
fn catmull(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    Vec3 {
        x: curve(p0.x, p1.x, p2.x, p3.x, t, t2, t3),
        y: curve(p0.y, p1.y, p2.y, p3.y, t, t2, t3),
        z: curve(p0.z, p1.z, p2.z, p3.z, t, t2, t3),
    }
}

/// The Catmull-Rom basis for one component.
fn curve(p0: f32, p1: f32, p2: f32, p3: f32, t: f32, t2: f32, t3: f32) -> f32 {
    0.5 * ((2.0 * p1)
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// One bone's animated channels: the `BoneTrack` record.
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTrack {
    pub position: Channel,
    pub rotation: Channel,
    pub scale: Channel,
}

impl BoneTrack {
    pub fn new(position: Channel, rotation: Channel, scale: Channel) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// Samples all three channels at `t`.
    pub fn sample(&self, t: f32, before: f32) -> Pose {
        Pose {
            position: self.position.sample_before(t, before),
            rotation: self.rotation.sample_before(t, before),
            scale: self.scale.sample_before(t, before),
        }
    }

    /// The last key's time across all three channels.
    pub fn last_key_time(&self) -> f32 {
        self.position
            .last_key_time()
            .max(self.rotation.last_key_time())
            .max(self.scale.last_key_time())
    }
}

/// One animated clip: the `Clip` record — a named, timed set of bone tracks.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub name: String,
    pub length: f32,
    pub loop_: bool,
    pub bones: std::collections::HashMap<String, BoneTrack>,
}

impl Clip {
    pub fn new(
        name: String,
        length: f32,
        loop_: bool,
        bones: std::collections::HashMap<String, BoneTrack>,
    ) -> Self {
        Self {
            name,
            length,
            loop_,
            bones,
        }
    }

    /// A copy without the local arm tracks (`left_arm`, `right_arm`).
    pub fn without_local_arms(&self) -> Clip {
        let mut bones = self.bones.clone();
        bones.remove("left_arm");
        bones.remove("right_arm");
        Clip::new(self.name.clone(), self.length, self.loop_, bones)
    }

    /// Samples one bone at `t`, in model space: the Java `sample`.
    pub fn sample(&self, bone: &str, t: f32) -> Pose {
        let bone = bone.to_lowercase();
        let track = self.bones.get(&bone);
        let time = self.clamp_time(t);
        let loop_length = if self.loop_ { self.length } else { 0.0 };
        let pose = match track {
            None => Pose::identity(),
            Some(track) => track.sample(time, loop_length),
        };
        self.to_model_space(&bone, pose, time, loop_length)
    }

    /// Samples the carry pose of a bone: identity transformed only by the
    /// virtual waist, like the Java `sampleCarry`.
    pub fn sample_carry(&self, bone: &str, t: f32) -> Pose {
        let time = self.clamp_time(t);
        let loop_length = if self.loop_ { self.length } else { 0.0 };
        self.inherit_virtual_waist(&bone.to_lowercase(), Pose::identity(), time, loop_length)
    }

    /// Samples one bone once (no looping, no look-behind): `sampleOnce`.
    pub fn sample_once(&self, bone: &str, t: f32) -> Pose {
        let bone = bone.to_lowercase();
        let track = self.bones.get(&bone);
        let time = 0.0f32.max(t.min(self.length));
        let pose = match track {
            None => Pose::identity(),
            Some(track) => track.sample(time, 0.0),
        };
        self.to_model_space(&bone, pose, time, 0.0)
    }

    fn clamp_time(&self, t: f32) -> f32 {
        if self.loop_ {
            positive_modulo(t, self.length)
        } else {
            0.0f32.max(t.min(self.length))
        }
    }

    /// Model-space adjustment: the idle root pivot, then the virtual waist.
    fn to_model_space(&self, bone: &str, pose: Pose, time: f32, loop_length: f32) -> Pose {
        if bone == "root" && self.name == "idle" {
            let pivot = Vec3::new(0.0, 12.0, 0.0);
            let q = quaternion(pose.rotation);
            let offset = q.transform_by(Vec3::new(pivot.x, pivot.y, pivot.z) * pose.scale) - pivot;
            let position = Vec3::new(
                pose.position.x + offset.x,
                pose.position.y - offset.y,
                pose.position.z + offset.z,
            );
            return Pose::new(position, pose.rotation, pose.scale);
        }
        self.inherit_virtual_waist(bone, pose, time, loop_length)
    }

    fn inherit_virtual_waist(&self, bone: &str, pose: Pose, time: f32, loop_length: f32) -> Pose {
        if !is_virtual_waist_child(bone) {
            return pose;
        }
        let Some(waist) = self.bones.get("waist") else {
            return pose;
        };
        let waist_pose = waist.sample(time, loop_length);
        let q = quaternion(waist_pose.rotation);
        let combined = q * quaternion(pose.rotation);
        let pivot = virtual_waist_child_pivot(bone);
        let waist_scale = waist_pose.scale;
        let rotated_pivot = q.transform_by(Vec3::new(pivot.x, pivot.y, pivot.z) * waist_scale);
        let rotated_offset = q.transform_by(
            Vec3::new(pose.position.x, -pose.position.y, pose.position.z) * waist_scale,
        );
        let mut position = rotated_pivot - pivot
            + rotated_offset
            + Vec3::new(
                waist_pose.position.x,
                -waist_pose.position.y,
                waist_pose.position.z,
            );
        position.y = -position.y;
        Pose::new(
            position,
            // JOML's getEulerAnglesZYX is radians; the pose carries degrees.
            combined.get_euler_zyx()
                * Vec3::new(
                    (180.0 / std::f64::consts::PI) as f32,
                    (180.0 / std::f64::consts::PI) as f32,
                    (180.0 / std::f64::consts::PI) as f32,
                ),
            waist_pose.scale * pose.scale,
        )
    }
}

/// `MovementTuning.jumpAirHold`: how far into the jump clip the air pose
/// holds before the landing clip's first frame takes over — the time whose
/// pose is closest to the landing's, searched at `AIR_HOLD_STEPS` + 1 points
/// across the jump (or across `hold` seconds of it when the pack set one).
/// A landing with no bones holds forever (`INFINITY`).
pub fn jump_air_hold(jump: &Clip, landing: &Clip, hold: f32) -> f32 {
    if landing.bones.is_empty() {
        return f32::INFINITY;
    }
    let span = if hold.is_finite() && hold > 0.0 {
        jump.length.min(hold)
    } else {
        jump.length
    };
    let mut best = span;
    let mut best_cost = f32::MAX;
    for i in 0..=96u32 {
        let t = span * i as f32 / 96.0;
        let mut cost = 0.0_f32;
        for bone in landing.bones.keys() {
            let air = jump.sample_once(bone, t);
            let landed = landing.sample_once(bone, 0.0);
            cost += distance_squared(air.rotation, landed.rotation)
                + 16.0 * distance_squared(air.position, landed.position);
        }
        if cost < best_cost - 1.0e-4 {
            best_cost = cost;
            best = t;
        }
    }
    best
}

/// `Vector3f.distanceSquared`: the squared straight distance.
fn distance_squared(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y + d.z * d.z
}

/// The pivot a virtual-waist child rotates around, in the clip's own space.
fn virtual_waist_child_pivot(bone: &str) -> Vec3 {
    match bone {
        "right_arm" => Vec3::new(-5.0, 2.0, 0.0),
        "left_arm" => Vec3::new(5.0, 2.0, 0.0),
        _ => Vec3::ZERO,
    }
}

/// Builds the quaternion from an Euler vector: JOML `rotationZYX`.
fn quaternion(rotation: Vec3) -> Quat {
    Quat::from_euler_zyx(rotation.z, rotation.y, rotation.x)
}

/// Bones that inherit the virtual waist's motion.
fn is_virtual_waist_child(bone: &str) -> bool {
    matches!(bone, "body" | "head" | "right_arm" | "left_arm")
}

/// Java `Float.remed`-style positive modulo: the remainder with the
/// divisor's sign made positive.
fn positive_modulo(value: f32, divisor: f32) -> f32 {
    let r = value % divisor;
    if r < 0.0 { r + divisor } else { r }
}

/// Camera profile: the `CameraProfile` record (shoulder camera offsets and
/// field of view).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraProfile {
    pub horizontal: f32,
    pub vertical: f32,
    pub depth: f32,
    pub fov: f32,
    pub response: f32,
}

impl CameraProfile {
    pub fn is_complete(&self) -> bool {
        self.horizontal.is_finite()
            && self.vertical.is_finite()
            && self.depth.is_finite()
            && self.fov.is_finite()
            && self.response.is_finite()
    }
}

/// One locomotion clip with its weight and overlays: the `LocomotionClip`
/// record.
#[derive(Debug, Clone, PartialEq)]
pub struct LocomotionClip {
    pub clip: Clip,
    pub multiplier: f32,
    pub overlays: Vec<ResolvedOverlay>,
    pub reverse: bool,
    pub add: Option<Clip>,
}

impl LocomotionClip {
    pub fn new(clip: Clip, multiplier: f32, overlays: Vec<ResolvedOverlay>) -> Self {
        Self {
            clip,
            multiplier,
            overlays,
            reverse: false,
            add: None,
        }
    }
}

/// One locomotion influence: the `LocomotionInfluence` record — the weights
/// per locomotion state and the transition share.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocomotionInfluence {
    pub walk: f32,
    pub sprint: f32,
    pub jump: f32,
    pub transition: f32,
    pub hand_only: bool,
}

impl LocomotionInfluence {
    pub fn new(walk: f32, sprint: f32, jump: f32) -> Self {
        Self {
            walk,
            sprint,
            jump,
            transition: 0.0,
            hand_only: false,
        }
    }

    /// Picks the weight for the current locomotion state: jumping wins over
    /// sprinting, sprinting over walking.
    pub fn select(&self, sprinting: bool, jumping: bool) -> f32 {
        if jumping {
            self.jump
        } else if sprinting {
            self.sprint
        } else {
            self.walk
        }
    }
}

/// One emote: the `Emote` record.
#[derive(Debug, Clone, PartialEq)]
pub struct Emote {
    pub id: String,
    pub display_name: String,
    pub animation: String,
    pub default_key: String,
    pub face: Option<String>,
}

/// The emote menu: the `EmoteCatalog` record.
#[derive(Debug, Clone, PartialEq)]
pub struct EmoteCatalog {
    pub menu_key: String,
    pub entries: Vec<Emote>,
}

/// One handed animation: the `HandedAnimation` record.
#[derive(Debug, Clone, PartialEq)]
pub struct HandedAnimation {
    pub clip: Clip,
    pub mirror: bool,
}

/// One resolved overlay: the `ResolvedOverlay` record — a clip plus the
/// bones it applies to.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedOverlay {
    pub clip: Clip,
    pub mode: OverlayMode,
    pub bones: Vec<String>,
}

impl ResolvedOverlay {
    /// Whether the overlay applies to `bone`: no bone list means every bone.
    pub fn applies_to(&self, bone: &str) -> bool {
        self.bones.is_empty() || self.bones.iter().any(|b| b == bone)
    }
}

/// How an overlay combines with the base pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    Additive,
    Override,
    Exclusive,
}

impl OverlayMode {
    /// The Java `fromString`: null or anything unknown is `ADDITIVE`.
    pub fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("override") => OverlayMode::Override,
            Some("exclusive") => OverlayMode::Exclusive,
            _ => OverlayMode::Additive,
        }
    }
}

/// The bone-name prefix the mod's face shorthand keys use (`FACE_PREFIX`).
pub const FACE_PREFIX: &str = "face_";
/// The mix bone the face shorthand installs (`face_mix`).
pub const FACE_MIX_BONE: &str = "face_mix";

/// Looks a member up in an object's member slice.
fn member<'a>(
    obj: &'a [(String, crate::json::Value)],
    key: &str,
) -> Option<&'a crate::json::Value> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `safeFloat`: the element as an f32, or `fallback` on null or any parse
/// failure (the Java helper catches every `RuntimeException`).
fn safe_float(elem: Option<&crate::json::Value>, fallback: f32) -> f32 {
    match elem {
        Some(crate::json::Value::Number(n)) => *n as f32,
        Some(crate::json::Value::String(s)) => s.trim().parse::<f32>().unwrap_or(fallback),
        _ => fallback,
    }
}

/// `vector`: a scalar becomes a uniform vector, an array of three or more
/// numbers becomes a vector (missing components fall back per component),
/// anything else is the fallback.
fn vector(elem: Option<&crate::json::Value>, fallback: Vec3) -> Vec3 {
    match elem {
        Some(crate::json::Value::Number(n)) => {
            let f = *n as f32;
            if f.is_finite() {
                Vec3::new(f, f, f)
            } else {
                fallback
            }
        }
        Some(crate::json::Value::String(s)) => match s.trim().parse::<f32>() {
            Ok(f) if f.is_finite() => Vec3::new(f, f, f),
            _ => fallback,
        },
        Some(crate::json::Value::Array(items)) if items.len() >= 3 => Vec3::new(
            safe_float(items.first(), fallback.x),
            safe_float(items.get(1), fallback.y),
            safe_float(items.get(2), fallback.z),
        ),
        _ => fallback,
    }
}

/// `parseChannel`: a channel from a JSON element. An array is one key at
/// time 0.0; an object maps time strings to keys (a bare array value is a
/// symmetric key, an object carries `pre`/`post`/`vector`, `lerp_mode` and
/// `easing`); anything else is an empty channel on the fallback.
pub fn parse_channel(elem: Option<&crate::json::Value>, fallback: Vec3) -> Channel {
    let mut keys: Vec<(f32, Key)> = Vec::new();
    let Some(elem) = elem.filter(|e| !e.is_null()) else {
        return Channel::new(keys, fallback);
    };
    match elem {
        crate::json::Value::Array(_) => {
            let v = vector(Some(elem), fallback);
            keys.push((
                0.0,
                Key {
                    pre: v,
                    post: v,
                    interpolation: "linear".to_string(),
                    easing: None,
                },
            ));
        }
        crate::json::Value::Object(members) => {
            for (key, value) in members {
                // A time that does not parse skips the entry, like the Java
                // catch around the whole entry.
                let Ok(time) = key.parse::<f32>() else {
                    continue;
                };
                match value {
                    crate::json::Value::Array(_) => {
                        let v = vector(Some(value), fallback);
                        keys.push((
                            time,
                            Key {
                                pre: v,
                                post: v,
                                interpolation: "linear".to_string(),
                                easing: None,
                            },
                        ));
                    }
                    crate::json::Value::Object(obj) => {
                        let post_elem = member(obj, "post")
                            .or_else(|| member(obj, "vector"))
                            .or_else(|| member(obj, "pre"));
                        let pre_elem = member(obj, "pre").or(post_elem);
                        let pre = vector(pre_elem, fallback);
                        let post = vector(post_elem, pre);
                        let lerp_mode = member(obj, "lerp_mode")
                            .and_then(crate::json::Value::as_str)
                            .unwrap_or("linear")
                            .to_string();
                        let easing = member(obj, "easing")
                            .filter(|e| matches!(e, crate::json::Value::String(_)))
                            .and_then(|e| e.as_str())
                            .and_then(crate::easing::Easing::by_name);
                        keys.push((
                            time,
                            Key {
                                pre,
                                post,
                                interpolation: lerp_mode,
                                easing,
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Channel::new(keys, fallback)
}

/// `heldY`: a one-key position track that holds the bone at `y`, with an
/// empty rotation and a unit scale.
pub fn held_y(y: f32) -> BoneTrack {
    let v = Vec3::new(0.0, y, 0.0);
    let key = Key {
        pre: v,
        post: v,
        interpolation: "linear".to_string(),
        easing: None,
    };
    BoneTrack {
        position: Channel::new(vec![(0.0, key)], Vec3::ZERO),
        rotation: Channel::new(Vec::new(), Vec3::ZERO),
        scale: Channel::new(Vec::new(), Vec3::ONE),
    }
}

/// The bone the face shorthand installs for the mix weight (`face_mix`).
fn face_shorthand(
    obj: &crate::json::Value,
    bones: &mut std::collections::HashMap<String, BoneTrack>,
) {
    let face_elem = obj.get("face");
    let mut name: Option<String> = None;
    let mut mix = 1.0f32;
    match face_elem {
        Some(crate::json::Value::String(s)) => name = Some(s.clone()),
        Some(crate::json::Value::Object(obj2)) => {
            name = member(obj2, "look")
                .and_then(crate::json::Value::as_str)
                .map(str::to_string);
            mix = member(obj2, "mix")
                .map(|e| safe_float(Some(e), 1.0))
                .unwrap_or(1.0);
        }
        _ => {}
    }
    let Some(name) = name.filter(|n| !n.trim().is_empty()) else {
        return;
    };
    bones
        .entry(FACE_MIX_BONE.to_string())
        .or_insert_with(|| held_y(mix));
    let normalized = name.trim().to_lowercase().replace(' ', "_");
    bones
        .entry(format!("{}{}", FACE_PREFIX, normalized))
        .or_insert_with(|| held_y(1.0));
}

/// `readClip`: parses one clip from its JSON object (the Java public entry).
pub fn read_clip(name: &str, obj: &crate::json::Value) -> Clip {
    parse_clip(name, obj)
}

/// `parseClip`: a clip from its JSON object. The length comes from
/// `animation_length`; a non-looping clip stretches to the last key's time;
/// the length never drops below 1e-4.
pub fn parse_clip(name: &str, obj: &crate::json::Value) -> Clip {
    let mut length = match obj.get("animation_length") {
        Some(elem) => safe_float(Some(elem), 0.0),
        None => 0.0,
    };
    let looping = obj
        .get("loop")
        .map(|e| matches!(e, crate::json::Value::Bool(true)))
        .unwrap_or(false);
    let mut bones: std::collections::HashMap<String, BoneTrack> = std::collections::HashMap::new();
    if let Some(crate::json::Value::Object(members)) = obj.get("bones") {
        for (bone_name, value) in members {
            let crate::json::Value::Object(track_obj) = value else {
                continue;
            };
            let track = BoneTrack {
                position: parse_channel(member(track_obj, "position"), Vec3::ZERO),
                rotation: parse_channel(member(track_obj, "rotation"), Vec3::ZERO),
                scale: parse_channel(member(track_obj, "scale"), Vec3::ONE),
            };
            bones.insert(bone_name.to_lowercase(), track.clone());
            if !looping {
                length = length.max(track.last_key_time());
            }
        }
    }
    face_shorthand(obj, &mut bones);
    Clip::new(name.to_string(), 1.0e-4f32.max(length), looping, bones)
}

/// The mod's animation library: the Java `AnimationLibrary`'s storage and
/// lookup, without the engine-coupled resolution (item stacks, biomes and
/// passengers stay with the game layer; `groupFor` is supplied by the game
/// layer too, since it needs the item registry).
pub struct AnimationLibrary {
    clips: std::collections::HashMap<String, Clip>,
    bindings: ItemAnimationBindings,
    face_bones: std::collections::HashSet<String>,
    hands: Option<Hands>,
}

impl AnimationLibrary {
    /// Builds a library; the face bones are derived from the clips, like the
    /// Java `faceBones(clips)` helper.
    pub fn new(
        clips: std::collections::HashMap<String, Clip>,
        bindings: ItemAnimationBindings,
    ) -> Self {
        Self::with_hands(clips, bindings, None)
    }

    /// Builds a library with the hands state set.
    pub fn with_hands(
        clips: std::collections::HashMap<String, Clip>,
        bindings: ItemAnimationBindings,
        hands: Option<Hands>,
    ) -> Self {
        let mut face_bones = std::collections::HashSet::new();
        for clip in clips.values() {
            for bone in clip.bones.keys() {
                if bone.starts_with(FACE_PREFIX) {
                    face_bones.insert(bone.clone());
                }
            }
        }
        // A face bone with a side gains its opposite, so a blink of one eye
        // can find the other.
        for name in std::mem::take(&mut face_bones) {
            face_bones.insert(name.clone());
            if let Some(rest) = name.strip_suffix("_right") {
                face_bones.insert(format!("{}_left", rest));
            } else if let Some(rest) = name.strip_suffix("_left") {
                face_bones.insert(format!("{}_right", rest));
            }
        }
        Self {
            clips,
            bindings,
            face_bones,
            hands,
        }
    }

    /// Whether any clip loaded.
    pub fn has_animations(&self) -> bool {
        !self.clips.is_empty()
    }

    /// A clip by its exact name.
    pub fn clip(&self, name: &str) -> Option<&Clip> {
        self.clips.get(name)
    }

    /// Every clip whose name starts with `prefix`, sorted by name.
    pub fn clips_with_prefix(&self, prefix: &str) -> Vec<&Clip> {
        let mut found: Vec<&Clip> = self
            .clips
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(_, clip)| clip)
            .collect();
        found.sort_by(|a, b| a.name.cmp(&b.name));
        found
    }

    /// The bones the face shorthand installed across every loaded clip.
    pub fn face_bones(&self) -> &std::collections::HashSet<String> {
        &self.face_bones
    }

    /// The bindings the library loads.
    pub fn bindings(&self) -> &ItemAnimationBindings {
        &self.bindings
    }
}

// ---------------------------------------------------------------------------
// The lookups the animation controllers read (the Java `AnimationLibrary`
// handed-clip and binding chains)
// ---------------------------------------------------------------------------

impl AnimationLibrary {
    /// The hands state the per-frame glue sets, the Java `hands` field: which
    /// arm is the main one and whether each hand carries something.
    pub fn set_hands(&mut self, main_left: bool, main_occupied: bool, off_occupied: bool) {
        self.hands = Some(Hands {
            main_left,
            main_occupied,
            off_occupied,
        });
    }

    /// `otherHandOccupied`.
    pub fn other_hand_occupied(&self, left: bool) -> bool {
        match &self.hands {
            Some(hands) => {
                if left == hands.main_left {
                    hands.off_occupied
                } else {
                    hands.main_occupied
                }
            }
            None => false,
        }
    }

    /// `namedClip`: the clip under `name_left`/`name_right`, then the
    /// `_alt` forms, then the bare name.
    pub fn named_clip(&self, name: Option<&str>, left: Option<bool>) -> Option<Clip> {
        let name = name.filter(|n| !n.trim().is_empty())?;
        if let Some(left) = left {
            if let Some(clip) =
                self.clips
                    .get(&format!("{}_{}", name, if left { "left" } else { "right" }))
            {
                return Some(clip.clone());
            }
            if let Some(base) = name.strip_suffix("_alt")
                && let Some(clip) = self.clips.get(&format!(
                    "{}_{}_alt",
                    base,
                    if left { "left" } else { "right" }
                ))
            {
                return Some(clip.clone());
            }
        }
        self.clips.get(name).cloned()
    }

    /// `oneHandedName`: the first action prefix carrying a `one_handed`
    /// setting, walking up the `_` separators.
    pub fn one_handed_name(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<String> {
        let mut action = action.to_string();
        loop {
            let name =
                self.bindings
                    .string_setting(item, group, &format!("{}_one_handed", action), "");
            if !name.is_empty() {
                return Some(name);
            }
            let cut = action.rfind('_')?;
            action.truncate(cut);
        }
    }

    /// `resolve`: the clip name an action resolves to, layered over the
    /// item, its group, the defaults and the empty hand.
    pub fn resolve_clip_name(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<String> {
        let action = normalize_id(action);
        let bindings = &self.bindings;
        if let Some(id) = item {
            if let Some(name) = bindings
                .specific_items
                .get(id)
                .and_then(|map| map.get(&action))
            {
                return Some(name.clone());
            }
            if let Some(group) = group
                && let Some(name) = bindings.groups.get(group).and_then(|map| map.get(&action))
            {
                return Some(name.clone());
            }
            return bindings.defaults.get(&action).cloned();
        }
        bindings
            .empty_hand
            .get(&action)
            .or_else(|| bindings.defaults.get(&action))
            .cloned()
    }

    /// `resolveClip`: the one-handed clip wins when the other hand is full.
    pub fn resolve_clip(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        left: Option<bool>,
        other_occupied: bool,
    ) -> Option<Clip> {
        if other_occupied {
            let one_handed = self.one_handed_name(item, group, action);
            if let Some(clip) = self.named_clip(
                one_handed
                    .as_deref()
                    .map(|_| {
                        self.one_handed_name(item, group, action)
                            .unwrap_or_default()
                    })
                    .as_deref()
                    .or(one_handed.as_deref()),
                left,
            ) {
                return Some(clip);
            }
        }
        let name = self.resolve_clip_name(item, group, action);
        self.named_clip(name.as_deref(), left)
    }

    /// `handedClipFor`: the clip for one action on one hand.
    pub fn handed_clip_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        left: bool,
        other_occupied: bool,
    ) -> Option<Clip> {
        let action = action.to_string();
        if let Some(base) = action.strip_suffix("_one_handed") {
            let base = base.to_string();
            let name =
                self.bindings
                    .string_setting(item, group, &format!("{}_one_handed", base), "");
            let name = if name.is_empty() {
                self.resolve_clip_name(item, group, &action)
            } else {
                Some(name)
            };
            return self.named_clip(name.as_deref(), Some(left));
        }
        // The bindings' own resolution may name a different clip.
        let resolved = self
            .resolve_clip_name(item, group, &action)
            .unwrap_or(action.clone());
        self.resolve_clip(item, group, &action, Some(left), other_occupied)
            .or_else(|| self.named_clip(Some(&resolved), Some(left)))
    }

    /// `handedClipsFor`: the sequence's clips, `action_1`, `action_2`, ...
    pub fn handed_clips_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        left: bool,
        other_occupied: bool,
    ) -> Vec<Clip> {
        if other_occupied {
            let one_handed = self.one_handed_name(item, group, action);
            if let Some(clip) = self.named_clip(one_handed.as_deref(), Some(left)) {
                return vec![clip];
            }
        }
        let mut out = Vec::new();
        for (index, name) in self
            .bindings
            .sequences_for(item, group, action)
            .iter()
            .enumerate()
        {
            let named = format!("{}_{}", action, index + 1);
            if let Some(clip) = self
                .resolve_clip(item, group, &named, Some(left), other_occupied)
                .or_else(|| self.named_clip(Some(name), Some(left)))
            {
                out.push(clip);
            }
        }
        out
    }

    /// `overlaysFor`: the overlays for an action, resolved to clips, with
    /// the handed name preferred and the one-handed variant honoured.
    pub fn overlays_resolved(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        left: bool,
        other_occupied: bool,
    ) -> Vec<ResolvedOverlay> {
        let mut out = Vec::new();
        for binding in self.bindings().overlays_for(item, group, action) {
            let clip = self
                .named_clip(
                    Some(&format!(
                        "{}_{}",
                        binding.name,
                        if left { "left" } else { "right" }
                    )),
                    Some(left),
                )
                .or_else(|| self.named_clip(Some(&binding.name), Some(left)));
            if let Some(clip) = clip {
                out.push(ResolvedOverlay {
                    clip,
                    mode: binding.mode,
                    bones: binding.bones,
                });
            }
        }
        let _ = other_occupied;
        out
    }

    /// `hasItemBinding`.
    pub fn has_item_binding(&self, item: Option<&str>, action: &str) -> bool {
        let action = normalize_id(action);
        let bindings = &self.bindings;
        match item {
            Some(id) => {
                bindings
                    .specific_items
                    .get(id)
                    .is_some_and(|map| map.contains_key(&action))
                    || bindings.defaults.contains_key(&action)
            }
            None => bindings.empty_hand.contains_key(&action),
        }
    }

    /// `retriggerProgress`: the stored percent over 100, clamped into a
    /// fraction.
    pub fn retrigger_progress(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        default: f32,
    ) -> f32 {
        self.bindings.retrigger_for(item, group, action, default)
    }

    /// `basePoseRetention`: the specific over the group over the default
    /// (or the empty hand over the default).
    pub fn base_pose_retention_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> BasePoseRetention {
        let action = normalize_id(action);
        let bindings = &self.bindings;
        let base = bindings
            .default_retentions
            .get(&action)
            .cloned()
            .unwrap_or_default();
        match item {
            Some(id) => {
                let specific = bindings
                    .specific_retentions
                    .get(id)
                    .and_then(|map| map.get(&action))
                    .cloned()
                    .unwrap_or_default();
                let group = group
                    .and_then(|g| bindings.group_retentions.get(g))
                    .and_then(|map| map.get(&action))
                    .cloned()
                    .unwrap_or_default();
                specific.over(&group.over(&base))
            }
            None => bindings
                .empty_hand_retentions
                .get(&action)
                .cloned()
                .unwrap_or_default()
                .over(&base),
        }
    }

    /// `configuredLocomotionInfluence`.
    pub fn configured_locomotion_influence(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<LocomotionInfluence> {
        self.bindings
            .influence_for(item, group, action)
            .map(|influence| LocomotionInfluence {
                walk: influence.walk,
                sprint: influence.sprint,
                jump: influence.jump,
                transition: influence.transition,
                hand_only: influence.hand_only,
            })
    }
}

/// The per-frame hands state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hands {
    /// The main hand is the left one.
    pub main_left: bool,
    /// The main hand carries something.
    pub main_occupied: bool,
    /// The off hand carries something.
    pub off_occupied: bool,
}

/// One overlay binding: the `OverlayBinding` record — a clip plus the bones
/// it applies to.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayBinding {
    pub name: String,
    pub mode: OverlayMode,
    pub bones: Vec<String>,
}

/// One blend binding: the `BlendBinding` record — a from/to progress window
/// with an optional boolean condition and a numeric modifier, each side
/// optionally driven by a compiled expression.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendBinding {
    pub from: f32,
    pub to: f32,
    pub duration_seconds: f32,
    pub condition: bool,
    pub condition_expression: Option<crate::expressions::Expression>,
    pub modifier: f32,
    pub modifier_expression: Option<crate::expressions::Expression>,
}

/// One timer binding: the `TimerBinding` record — a numeric modifier and an
/// optional boolean condition, each side optionally expression-driven.
#[derive(Debug, Clone, PartialEq)]
pub struct TimerBinding {
    pub modifier: f32,
    pub condition: bool,
    pub modifier_expression: Option<crate::expressions::Expression>,
    pub condition_expression: Option<crate::expressions::Expression>,
}

/// One variable binding: the `VariableBinding` record — a named variable an
/// expression can read, boolean or numeric, with the names it references.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBinding {
    pub name: String,
    pub is_bool: bool,
    pub expression: Option<crate::expressions::Expression>,
    pub references: Vec<String>,
}

/// One locomotion binding: the `LocomotionBinding` record — a clip with an
/// optional condition string, an effect, overlays, a reverse flag and an
/// optional add clip.
#[derive(Debug, Clone, PartialEq)]
pub struct LocomotionBinding {
    pub clip: String,
    pub condition: Option<String>,
    pub effect: Option<String>,
    pub overlays: Vec<OverlayBinding>,
    pub reverse: bool,
    pub add: Option<String>,
}

/// One locomotion influence: the bindings' `LocomotionInfluence` record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BindLocomotionInfluence {
    pub walk: f32,
    pub sprint: f32,
    pub jump: f32,
    pub transition: f32,
    pub hand_only: bool,
}

/// One emote binding: the `EmoteBinding` record.
#[derive(Debug, Clone, PartialEq)]
pub struct EmoteBinding {
    pub id: String,
    pub display_name: String,
    pub animation: String,
    pub key: Option<String>,
    pub face: Option<String>,
}

/// The item bindings: the Java `ItemAnimationBindings`' storage and lookup.
/// The layering is specific item → group → default, with the empty-hand
/// layer standing in for a missing item. `group_for` is supplied by the
/// caller (the game layer owns the item registry), so resolution takes the
/// group explicitly.
pub struct ItemAnimationBindings {
    defaults: HashMap<String, String>,
    empty_hand: HashMap<String, String>,
    groups: HashMap<String, HashMap<String, String>>,
    specific_items: HashMap<String, HashMap<String, String>>,
    default_sequences: HashMap<String, Vec<String>>,
    empty_hand_sequences: HashMap<String, Vec<String>>,
    group_sequences: HashMap<String, HashMap<String, Vec<String>>>,
    specific_sequences: HashMap<String, HashMap<String, Vec<String>>>,
    pose_idles: HashMap<String, String>,
    timer_effects: HashMap<String, String>,
    timer_scales: HashMap<String, f64>,
    numeric_settings: HashMap<String, f64>,
    string_settings: HashMap<String, String>,
    empty_hand_string_settings: HashMap<String, String>,
    variants: HashMap<String, Vec<String>>,
    locomotion: HashMap<String, Vec<LocomotionBinding>>,
    default_retrigger: HashMap<String, f64>,
    empty_hand_retrigger: HashMap<String, f64>,
    group_retrigger: HashMap<String, HashMap<String, f64>>,
    specific_retrigger: HashMap<String, HashMap<String, f64>>,
    default_influences: HashMap<String, BindLocomotionInfluence>,
    empty_hand_influences: HashMap<String, BindLocomotionInfluence>,
    group_influences: HashMap<String, HashMap<String, BindLocomotionInfluence>>,
    specific_influences: HashMap<String, HashMap<String, BindLocomotionInfluence>>,
    default_overlays: HashMap<String, Vec<OverlayBinding>>,
    group_overlays: HashMap<String, HashMap<String, Vec<OverlayBinding>>>,
    specific_overlays: HashMap<String, HashMap<String, Vec<OverlayBinding>>>,
    default_blends: HashMap<String, BlendBinding>,
    empty_hand_blends: HashMap<String, BlendBinding>,
    group_blends: HashMap<String, HashMap<String, BlendBinding>>,
    specific_blends: HashMap<String, HashMap<String, BlendBinding>>,
    default_timers: HashMap<String, TimerBinding>,
    empty_hand_timers: HashMap<String, TimerBinding>,
    group_timers: HashMap<String, HashMap<String, TimerBinding>>,
    specific_timers: HashMap<String, HashMap<String, TimerBinding>>,
    group_numeric_settings: HashMap<String, HashMap<String, f64>>,
    specific_numeric_settings: HashMap<String, HashMap<String, f64>>,
    group_string_settings: HashMap<String, HashMap<String, String>>,
    specific_string_settings: HashMap<String, HashMap<String, String>>,
    default_static_when_using: Option<bool>,
    empty_hand_static_when_using: Option<bool>,
    group_static_when_using: HashMap<String, bool>,
    specific_static_when_using: HashMap<String, bool>,
    variables: Vec<VariableBinding>,
    emote_menu_key: Option<String>,
    emote_bindings: Vec<EmoteBinding>,
    default_one_handed: HashMap<String, String>,
    default_animation_types: std::collections::HashSet<String>,
    default_retentions: HashMap<String, BasePoseRetention>,
    empty_hand_retentions: HashMap<String, BasePoseRetention>,
    group_retentions: HashMap<String, HashMap<String, BasePoseRetention>>,
    specific_retentions: HashMap<String, HashMap<String, BasePoseRetention>>,
}

impl Default for ItemAnimationBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemAnimationBindings {
    /// `numericSetting(stack, action, key, default)`: the composite
    /// `action + field` key, layered, falling back to the bare field.
    pub fn numeric_action_setting(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        key: &str,
        default: f32,
    ) -> f32 {
        let composite = format!("{}{}", normalize_id(action), normalize_id(key));
        let flat = normalize_id(key);
        let pick =
            |map: &HashMap<String, f64>| map.get(&composite).or_else(|| map.get(&flat)).copied();
        let value = match item {
            Some(id) => self
                .specific_numeric_settings
                .get(id)
                .and_then(pick)
                .or_else(|| {
                    group
                        .and_then(|g| self.group_numeric_settings.get(g))
                        .and_then(pick)
                })
                .or_else(|| pick(&self.numeric_settings)),
            None => pick(&self.numeric_settings),
        };
        value.map(|v| v as f32).unwrap_or(default)
    }

    /// `stringSetting(stack, action, key, default)`: the composite key,
    /// falling back to the bare field.
    pub fn string_action_setting(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        key: &str,
        default: &str,
    ) -> String {
        let composite = format!("{}{}", normalize_id(action), normalize_id(key));
        let flat = normalize_id(key);
        let pick =
            |map: &HashMap<String, String>| map.get(&composite).or_else(|| map.get(&flat)).cloned();
        match item {
            Some(id) => self
                .specific_string_settings
                .get(id)
                .and_then(pick)
                .or_else(|| {
                    group
                        .and_then(|g| self.group_string_settings.get(g))
                        .and_then(pick)
                })
                .or_else(|| pick(&self.string_settings))
                .unwrap_or_else(|| default.to_string()),
            None => pick(&self.empty_hand_string_settings)
                .or_else(|| pick(&self.string_settings))
                .unwrap_or_else(|| default.to_string()),
        }
    }

    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
            empty_hand: HashMap::new(),
            groups: HashMap::new(),
            specific_items: HashMap::new(),
            default_sequences: HashMap::new(),
            empty_hand_sequences: HashMap::new(),
            group_sequences: HashMap::new(),
            specific_sequences: HashMap::new(),
            pose_idles: HashMap::new(),
            timer_effects: HashMap::new(),
            timer_scales: HashMap::new(),
            numeric_settings: HashMap::new(),
            string_settings: HashMap::new(),
            empty_hand_string_settings: HashMap::new(),
            variants: HashMap::new(),
            locomotion: HashMap::new(),
            default_retrigger: HashMap::new(),
            empty_hand_retrigger: HashMap::new(),
            group_retrigger: HashMap::new(),
            specific_retrigger: HashMap::new(),
            default_influences: HashMap::new(),
            empty_hand_influences: HashMap::new(),
            group_influences: HashMap::new(),
            specific_influences: HashMap::new(),
            default_overlays: HashMap::new(),
            group_overlays: HashMap::new(),
            specific_overlays: HashMap::new(),
            default_blends: HashMap::new(),
            empty_hand_blends: HashMap::new(),
            group_blends: HashMap::new(),
            specific_blends: HashMap::new(),
            default_timers: HashMap::new(),
            empty_hand_timers: HashMap::new(),
            group_timers: HashMap::new(),
            specific_timers: HashMap::new(),
            group_numeric_settings: HashMap::new(),
            specific_numeric_settings: HashMap::new(),
            group_string_settings: HashMap::new(),
            specific_string_settings: HashMap::new(),
            default_static_when_using: None,
            empty_hand_static_when_using: None,
            group_static_when_using: HashMap::new(),
            specific_static_when_using: HashMap::new(),
            variables: Vec::new(),
            emote_menu_key: None,
            emote_bindings: Vec::new(),
            default_one_handed: HashMap::new(),
            default_animation_types: std::collections::HashSet::new(),
            default_retentions: HashMap::new(),
            empty_hand_retentions: HashMap::new(),
            group_retentions: HashMap::new(),
            specific_retentions: HashMap::new(),
        }
    }

    /// Ingests one normalized (format 2) bindings JSON object: the Java
    /// `add`. The four layers are `default_animations`, `emptyHand`,
    /// `item` (groups) and `itemSpecific`.
    pub fn add(&mut self, root: &crate::json::Value) {
        if !root.is_object() {
            return;
        }
        if let Some(v) = root.get("static_when_using") {
            self.default_static_when_using = optional_boolean(v);
        }
        if let Some(v) = root.get("variables") {
            self.add_variables(v);
        }
        if let Some(elem) = root.get("default_animations") {
            self.add_entries(elem, Layer::Default, EntryKinds::all());
            self.add_sequences(elem, Layer::Default);
            self.add_retrigger(elem, Layer::Default);
            self.add_influences(elem, Layer::Default);
            self.add_overlays(elem, Layer::Default);
            self.add_blends(elem, Layer::Default);
            self.add_retentions(elem, Layer::Default);
            self.add_timers(elem, Layer::Default);
            self.add_emote_entries(elem);
            self.index_one_handed_clips(elem);
        }
        if let Some(empty_obj) = root.get("emptyHand").and_then(|e| e.as_object()) {
            if let Some(v) = member(empty_obj, "static_when_using") {
                self.empty_hand_static_when_using = optional_boolean(v);
            }
            if let Some(custom) = member(empty_obj, "customAnimation") {
                self.add_entries(
                    custom,
                    Layer::EmptyHand,
                    EntryKinds {
                        strings_to_empty_hand: true,
                        ..EntryKinds::default()
                    },
                );
                self.add_sequences(custom, Layer::EmptyHand);
                self.add_retrigger(custom, Layer::EmptyHand);
                self.add_influences(custom, Layer::EmptyHand);
                self.add_blends(custom, Layer::EmptyHand);
                self.add_retentions(custom, Layer::EmptyHand);
                self.add_timers(custom, Layer::EmptyHand);
            }
        }
        if let Some(groups) = root.get("item").and_then(|e| e.as_object()) {
            for (group_name, group_obj) in groups {
                let group = normalize_id(group_name);
                if let Some(v) = group_obj.get("static_when_using") {
                    self.group_static_when_using
                        .insert(group.clone(), optional_boolean(v).unwrap_or(false));
                }
                if let Some(camera) = group_obj.get("throwable_camera") {
                    let map = self
                        .group_numeric_settings
                        .entry(group.clone())
                        .or_default();
                    Self::add_camera_settings(camera, map);
                }
                if let Some(custom) = group_obj.get("customAnimation") {
                    self.add_entries(
                        custom,
                        Layer::Group(group.clone()),
                        EntryKinds {
                            numeric: true,
                            ..EntryKinds::default()
                        },
                    );
                    self.add_sequences(custom, Layer::Group(group.clone()));
                    self.add_retrigger(custom, Layer::Group(group.clone()));
                    self.add_influences(custom, Layer::Group(group.clone()));
                    self.add_overlays(custom, Layer::Group(group.clone()));
                    self.add_blends(custom, Layer::Group(group.clone()));
                    self.add_retentions(custom, Layer::Group(group.clone()));
                    self.add_timers(custom, Layer::Group(group.clone()));
                }
            }
        }
        if let Some(specific) = root.get("itemSpecific").and_then(|e| e.as_object()) {
            for (item_name, item_obj) in specific {
                let item = normalize_id(item_name);
                if let Some(v) = item_obj.get("static_when_using") {
                    self.specific_static_when_using
                        .insert(item.clone(), optional_boolean(v).unwrap_or(false));
                }
                if let Some(camera) = item_obj.get("throwable_camera") {
                    let map = self
                        .specific_numeric_settings
                        .entry(item.clone())
                        .or_default();
                    Self::add_camera_settings(camera, map);
                }
                if let Some(custom) = item_obj.get("customAnimation") {
                    self.add_entries(
                        custom,
                        Layer::Specific(item.clone()),
                        EntryKinds {
                            numeric: true,
                            ..EntryKinds::default()
                        },
                    );
                    self.add_sequences(custom, Layer::Specific(item.clone()));
                    self.add_retrigger(custom, Layer::Specific(item.clone()));
                    self.add_influences(custom, Layer::Specific(item.clone()));
                    self.add_overlays(custom, Layer::Specific(item.clone()));
                    self.add_blends(custom, Layer::Specific(item.clone()));
                    self.add_retentions(custom, Layer::Specific(item.clone()));
                    self.add_timers(custom, Layer::Specific(item.clone()));
                }
            }
        }
    }

    /// The clip an action resolves to: specific item → group → default,
    /// then the empty-hand layer for a missing item.
    pub fn resolve(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<&String> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(map) = self.specific_items.get(id)
                && let Some(clip) = map.get(&action)
            {
                return Some(clip);
            }
            if let Some(group) = group
                && let Some(map) = self.groups.get(group)
                && let Some(clip) = map.get(&action)
            {
                return Some(clip);
            }
            return self.defaults.get(&action);
        }
        if let Some(map) = self.empty_hand.get(&action) {
            return Some(map);
        }
        self.defaults.get(&action)
    }

    /// `first`: the layered string lookup — specific map, then group map,
    /// then the default map.
    fn first<'a>(
        specific: Option<&'a HashMap<String, String>>,
        group: Option<&'a HashMap<String, String>>,
        defaults: &'a HashMap<String, String>,
        action: &str,
    ) -> Option<&'a String> {
        if let Some(map) = specific
            && let Some(v) = map.get(action)
        {
            return Some(v);
        }
        if let Some(map) = group
            && let Some(v) = map.get(action)
        {
            return Some(v);
        }
        defaults.get(action)
    }
}

impl EntryKinds {
    /// Every sub-target filled: the Java defaults call passes all nine maps.
    pub fn all() -> Self {
        Self {
            pose_idles: true,
            timer_effects: true,
            timer_scales: true,
            numeric: true,
            strings_to_empty_hand: false,
            variants: true,
            locomotion: true,
        }
    }
}

impl ItemAnimationBindings {
    /// Ingests one action-to-descriptor pass into the layer's maps. The
    /// per-action descriptor array carries the clip (`name` or `clip`),
    /// optional `variants`, and an optional `one_handed` clip.
    fn add_entries(&mut self, elem: &crate::json::Value, layer: Layer, kinds: EntryKinds) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(desc) = descriptor.as_object() else {
                continue;
            };
            // The entry's `type` names the action it poses.
            let Some(action) = member(desc, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            let clip_name = member(desc, "name")
                .and_then(crate::json::Value::as_str)
                .or_else(|| member(desc, "clip").and_then(crate::json::Value::as_str));
            if let Some(clip_name) = clip_name
                && let Some(entries) = self.entries_map(&layer)
            {
                entries.insert(action.clone(), clip_name.to_string());
                self.default_animation_types.insert(action.clone());
            }
            if kinds.variants
                && let Some(vs) = member(desc, "variants").and_then(|v| v.as_array())
            {
                // A variant is a clip name, or an object with `name`/`clip`.
                let names: Vec<String> = vs
                    .iter()
                    .filter_map(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .or_else(|| value_of(v.as_object()?, &["name", "clip"]))
                    })
                    .collect();
                if !names.is_empty() {
                    self.variants.insert(action.clone(), names);
                }
            }
            if let Some(one_handed) = member(desc, "one_handed") {
                let clip = match one_handed {
                    crate::json::Value::String(s) => Some(s.clone()),
                    crate::json::Value::Object(o) => value_of(o, &["clip", "name"]),
                    _ => None,
                };
                if let Some(clip) = clip {
                    self.default_one_handed.insert(action.clone(), clip);
                }
            }
            if kinds.pose_idles
                && let Some(idle) = member(desc, "pose_idle").and_then(|v| v.as_str())
            {
                self.pose_idles.insert(action.clone(), idle.to_string());
            }
            if kinds.timer_effects
                && let Some(effect) = member(desc, "timer_effect").and_then(|v| v.as_str())
            {
                self.timer_effects
                    .insert(action.clone(), effect.to_string());
            }
            if kinds.timer_scales
                && let Some(scale) = member(desc, "timer_scale")
                && let Some(f) = scale.as_f64()
            {
                self.timer_scales.insert(action.clone(), f);
            }
            // The Java chain reads its setting fields straight off the
            // descriptor and stores them under `action + field`.
            for (field, value) in desc {
                if let Some(f) = value.as_f64()
                    && NUMERIC_SETTING_FIELDS.contains(&field.as_str())
                {
                    let target = self.numeric_map(&layer);
                    target.insert(format!("{}{}", action, normalize_id(field)), f);
                }
                if let Some(text) = value.as_str()
                    && STRING_SETTING_FIELDS.contains(&field.as_str())
                    && let Some(target) = self.string_map(&layer)
                {
                    target.insert(
                        format!("{}{}", action, normalize_id(field)),
                        text.to_string(),
                    );
                }
            }
            if kinds.numeric
                && let Some(numeric) = member(desc, "settings").and_then(|v| v.as_object())
            {
                let target = self.numeric_map(&layer);
                for (k, v) in numeric {
                    if let Some(f) = v.as_f64() {
                        target.insert(normalize_id(k), f);
                    }
                }
            }
            if kinds.strings_to_empty_hand {
                if let Some(strings) = member(desc, "settings").and_then(|v| v.as_object()) {
                    for (k, v) in strings {
                        if let Some(s) = v.as_str() {
                            self.empty_hand_string_settings
                                .insert(normalize_id(k), s.to_string());
                        }
                    }
                }
            } else if let Some(strings) = member(desc, "settings").and_then(|v| v.as_object())
                && let Some(target) = self.string_map(&layer)
            {
                for (k, v) in strings {
                    if let Some(s) = v.as_str() {
                        target.insert(normalize_id(k), s.to_string());
                    }
                }
            }
            if kinds.locomotion
                && let Some(loc) = member(desc, "locomotion")
            {
                let binding = parse_locomotion_binding(loc);
                self.locomotion
                    .entry(action.clone())
                    .or_default()
                    .push(binding);
            }
        }
    }

    fn entries_map(&mut self, layer: &Layer) -> Option<&mut HashMap<String, String>> {
        match layer {
            Layer::Default => Some(&mut self.defaults),
            Layer::EmptyHand => Some(&mut self.empty_hand),
            Layer::Group(g) => Some(self.groups.entry(g.clone()).or_default()),
            Layer::Specific(i) => Some(self.specific_items.entry(i.clone()).or_default()),
        }
    }

    fn numeric_map(&mut self, layer: &Layer) -> &mut HashMap<String, f64> {
        match layer {
            Layer::Default => &mut self.numeric_settings,
            Layer::EmptyHand => &mut self.numeric_settings,
            Layer::Group(g) => self.group_numeric_settings.entry(g.clone()).or_default(),
            Layer::Specific(i) => self.specific_numeric_settings.entry(i.clone()).or_default(),
        }
    }

    fn string_map(&mut self, layer: &Layer) -> Option<&mut HashMap<String, String>> {
        match layer {
            Layer::Default => Some(&mut self.string_settings),
            Layer::EmptyHand => Some(&mut self.empty_hand_string_settings),
            Layer::Group(g) => Some(self.group_string_settings.entry(g.clone()).or_default()),
            Layer::Specific(i) => Some(self.specific_string_settings.entry(i.clone()).or_default()),
        }
    }

    fn add_sequences(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(action) = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            // The sequence is the entry's `var_1`, `var_2`, ... clips.
            let mut names = Vec::new();
            let mut index = 1;
            while let Some(name) = value_of(obj, &[&format!("var_{}", index)]) {
                names.push(name);
                index += 1;
            }
            if names.is_empty() {
                continue;
            }
            match &layer {
                Layer::Default => {
                    self.default_sequences.insert(action, names);
                }
                Layer::EmptyHand => {
                    self.empty_hand_sequences.insert(action, names);
                }
                Layer::Group(g) => {
                    self.group_sequences
                        .entry(g.clone())
                        .or_default()
                        .insert(action, names);
                }
                Layer::Specific(i) => {
                    self.specific_sequences
                        .entry(i.clone())
                        .or_default()
                        .insert(action, names);
                }
            }
        }
    }

    fn add_retrigger(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(action) = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            let Some(f) = member(obj, "retrigger_progress").and_then(|v| v.as_f64()) else {
                continue;
            };
            match &layer {
                Layer::Default => {
                    self.default_retrigger.insert(action, f);
                }
                Layer::EmptyHand => {
                    self.empty_hand_retrigger.insert(action, f);
                }
                Layer::Group(g) => {
                    self.group_retrigger
                        .entry(g.clone())
                        .or_default()
                        .insert(action, f);
                }
                Layer::Specific(i) => {
                    self.specific_retrigger
                        .entry(i.clone())
                        .or_default()
                        .insert(action, f);
                }
            }
        }
    }

    fn add_influences(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(action) = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            let Some(influence) = locomotion_influence(member(obj, "locomotion_influence")) else {
                continue;
            };
            match &layer {
                Layer::Default => {
                    self.default_influences.insert(action, influence);
                }
                Layer::EmptyHand => {
                    self.empty_hand_influences.insert(action, influence);
                }
                Layer::Group(g) => {
                    self.group_influences
                        .entry(g.clone())
                        .or_default()
                        .insert(action, influence);
                }
                Layer::Specific(i) => {
                    self.specific_influences
                        .entry(i.clone())
                        .or_default()
                        .insert(action, influence);
                }
            }
        }
    }

    fn add_overlays(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(ty) = member(obj, "type").and_then(crate::json::Value::as_str) else {
                continue;
            };
            let action = normalize_id(ty);
            for (field, suffix) in [("overlays", ""), ("one_handed_overlays", "onehanded")] {
                let Some(list) = member(obj, field).and_then(|v| v.as_array()) else {
                    continue;
                };
                let bindings: Vec<OverlayBinding> =
                    list.iter().filter_map(parse_overlay_binding).collect();
                if bindings.is_empty() {
                    continue;
                }
                let key = format!("{}{}", action, suffix);
                match &layer {
                    Layer::Default => {
                        self.default_overlays.insert(key, bindings);
                    }
                    Layer::EmptyHand => {}
                    Layer::Group(g) => {
                        self.group_overlays
                            .entry(g.clone())
                            .or_default()
                            .insert(key, bindings);
                    }
                    Layer::Specific(i) => {
                        self.specific_overlays
                            .entry(i.clone())
                            .or_default()
                            .insert(key, bindings);
                    }
                }
            }
        }
    }

    fn add_blends(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(action) = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            let Some(blending) = member(obj, "blending").and_then(|v| v.as_object()) else {
                continue;
            };
            let binding = BlendBinding {
                from: bounded_float(blending, "from", 0.0),
                to: bounded_float(blending, "to", 1.0),
                duration_seconds: non_negative_float(obj, "blending_progression", 0.0),
                condition: literal_blend_condition(member(obj, "blending_condition")),
                condition_expression: expression(obj, "blending_condition", "true"),
                modifier: non_negative_float(obj, "blending_modifier", 1.0),
                modifier_expression: expression(obj, "blending_modifier", "1"),
            };
            match &layer {
                Layer::Default => {
                    self.default_blends.insert(action, binding);
                }
                Layer::EmptyHand => {
                    self.empty_hand_blends.insert(action, binding);
                }
                Layer::Group(g) => {
                    self.group_blends
                        .entry(g.clone())
                        .or_default()
                        .insert(action, binding);
                }
                Layer::Specific(i) => {
                    self.specific_blends
                        .entry(i.clone())
                        .or_default()
                        .insert(action, binding);
                }
            }
        }
    }

    fn add_timers(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(action) = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .map(normalize_id)
            else {
                continue;
            };
            if member(obj, "timer_modifier").is_none() && member(obj, "timer_condition").is_none() {
                continue;
            }
            let binding = TimerBinding {
                modifier: non_negative_float(obj, "timer_modifier", 1.0),
                condition: literal_blend_condition(member(obj, "timer_condition")),
                modifier_expression: expression(obj, "timer_modifier", "1"),
                condition_expression: expression(obj, "timer_condition", "true"),
            };
            match &layer {
                Layer::Default => {
                    self.default_timers.insert(action, binding);
                }
                Layer::EmptyHand => {
                    self.empty_hand_timers.insert(action, binding);
                }
                Layer::Group(g) => {
                    self.group_timers
                        .entry(g.clone())
                        .or_default()
                        .insert(action, binding);
                }
                Layer::Specific(i) => {
                    self.specific_timers
                        .entry(i.clone())
                        .or_default()
                        .insert(action, binding);
                }
            }
        }
    }

    /// `addRetentions`: the entry's `base_pose_retain`, parsed with the
    /// entry's type as the error context.
    fn add_retentions(&mut self, elem: &crate::json::Value, layer: Layer) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let Some(ty) = member(obj, "type").and_then(crate::json::Value::as_str) else {
                continue;
            };
            let Some(retain) = member(obj, "base_pose_retain") else {
                continue;
            };
            let Ok(retention) =
                BasePoseRetention::parse(Some(retain), &format!("{}.base_pose_retain", ty))
            else {
                continue;
            };
            let action = normalize_id(ty);
            match &layer {
                Layer::Default => {
                    self.default_retentions.insert(action, retention);
                }
                Layer::EmptyHand => {
                    self.empty_hand_retentions.insert(action, retention);
                }
                Layer::Group(g) => {
                    self.group_retentions
                        .entry(g.clone())
                        .or_default()
                        .insert(action, retention);
                }
                Layer::Specific(i) => {
                    self.specific_retentions
                        .entry(i.clone())
                        .or_default()
                        .insert(action, retention);
                }
            }
        }
    }

    /// `addEmoteEntries`: the `emotes` entry's menu key and its emote list.
    fn add_emote_entries(&mut self, elem: &crate::json::Value) {
        let Some(descriptors) = elem.as_array() else {
            return;
        };
        for descriptor in descriptors {
            let Some(obj) = descriptor.as_object() else {
                continue;
            };
            let is_emotes = member(obj, "type")
                .and_then(crate::json::Value::as_str)
                .is_some_and(|ty| normalize_id(ty) == "emotes");
            if !is_emotes {
                continue;
            }
            if let Some(key) = value_of(obj, &["menu_key", "keybind", "key"]) {
                self.emote_menu_key = Some(key);
            }
            let entries = member(obj, "entries")
                .or_else(|| member(obj, "emotes"))
                .and_then(|v| v.as_array());
            let Some(entries) = entries else {
                continue;
            };
            for entry in entries {
                let Some(entry) = entry.as_object() else {
                    continue;
                };
                let Some(animation) = value_of(entry, &["animation", "name", "clip"]) else {
                    continue;
                };
                let id = value_of(entry, &["id"]).unwrap_or_else(|| animation.clone());
                let display_name = value_of(entry, &["display_name", "label", "title"])
                    .unwrap_or_else(|| id.clone());
                let key = value_of(entry, &["keybind", "key"]).unwrap_or_else(|| "UNKNOWN".into());
                let face = value_of(entry, &["face", "expression"]);
                self.emote_bindings.push(EmoteBinding {
                    id,
                    display_name,
                    animation,
                    key: Some(key),
                    face,
                });
            }
        }
    }

    fn add_camera_settings(camera: &crate::json::Value, map: &mut HashMap<String, f64>) {
        let Some(obj) = camera.as_object() else {
            return;
        };
        for (k, v) in obj {
            if let Some(f) = v.as_f64() {
                map.insert(normalize_id(k), f);
            }
        }
    }

    fn add_variables(&mut self, elem: &crate::json::Value) {
        let Some(items) = elem.as_array() else {
            return;
        };
        for item in items {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let Some(name) = member(obj, "name").and_then(crate::json::Value::as_str) else {
                continue;
            };
            let is_bool = member(obj, "bool")
                .map(|v| matches!(v, crate::json::Value::Bool(true)))
                .unwrap_or(false);
            let expression = member(obj, "expression")
                .and_then(crate::json::Value::as_str)
                .and_then(crate::expressions::compile);
            let references = expression
                .as_ref()
                .map(|_| vec![name.to_string()])
                .unwrap_or_default();
            self.variables.push(VariableBinding {
                name: name.to_string(),
                is_bool,
                expression,
                references,
            });
        }
    }

    /// `indexOneHandedClips`: the entry's `one_handed` clip, or the first
    /// one found on a nested entry under `subanimations`.
    fn index_one_handed_clips(&mut self, elem: &crate::json::Value) {
        self.index_one_handed(elem, None);
    }

    fn index_one_handed(&mut self, elem: &crate::json::Value, action: Option<&str>) {
        if let Some(items) = elem.as_array() {
            for item in items {
                self.index_one_handed(item, action);
            }
            return;
        }
        let Some(obj) = elem.as_object() else {
            return;
        };
        let action = member(obj, "type")
            .and_then(crate::json::Value::as_str)
            .map(normalize_id)
            .or_else(|| action.map(str::to_string));
        if let Some(one_handed) = member(obj, "one_handed") {
            let clip = match one_handed {
                crate::json::Value::String(s) => Some(s.clone()),
                crate::json::Value::Object(o) => value_of(o, &["clip", "name"]),
                _ => None,
            };
            if let (Some(clip), Some(action)) = (clip, action.as_deref()) {
                self.default_one_handed
                    .entry(action.to_string())
                    .or_insert(clip);
            }
        }
        if let Some(children) = member(obj, "subanimations") {
            self.index_one_handed(children, action.as_deref());
        }
    }

    /// `firstRetention`: the specific, then group, then empty-hand, then
    /// default retention for an action.
    #[allow(dead_code)] // the game layer calls this once retention lookups land
    fn first_retention(&self, item: &str, group: &str, action: &str) -> Option<BasePoseRetention> {
        self.specific_retentions
            .get(item)
            .and_then(|map| map.get(action))
            .or_else(|| {
                self.group_retentions
                    .get(group)
                    .and_then(|map| map.get(action))
            })
            .or_else(|| self.empty_hand_retentions.get(action))
            .or_else(|| self.default_retentions.get(action))
            .cloned()
    }
}

/// `optional_boolean`: a JSON `true`/`false`, else `None`.
fn optional_boolean(v: &crate::json::Value) -> Option<bool> {
    match v {
        crate::json::Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// `value`: the first member of `keys` holding a non-blank string.
fn value_of(obj: &[(String, crate::json::Value)], keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = member(obj, key)
            && let Some(text) = value.as_str()
            && !text.trim().is_empty()
        {
            return Some(text.to_string());
        }
    }
    None
}

/// `boundedFloat`: a number clamped into `0.0..=1.0`, `fallback` when the
/// member is missing or not a number.
fn bounded_float(obj: &[(String, crate::json::Value)], key: &str, fallback: f32) -> f32 {
    match member(obj, key).and_then(|v| v.as_f64()) {
        Some(f) => (f as f32).clamp(0.0, 1.0),
        None => fallback,
    }
}

/// `nonNegativeFloat` (and `positiveFloat`, the same body): a number floored
/// at `0.0`, `fallback` when the member is missing or not a number.
fn non_negative_float(obj: &[(String, crate::json::Value)], key: &str, fallback: f32) -> f32 {
    match member(obj, key).and_then(|v| v.as_f64()) {
        Some(f) => (f as f32).max(0.0),
        None => fallback,
    }
}

/// `literalBlendCondition`: the condition as a plain value — absent is true,
/// a boolean is itself, a string is anything but `false`, `never` and `0`
/// (normalized), and anything else is true.
fn literal_blend_condition(value: Option<&crate::json::Value>) -> bool {
    let Some(value) = value else {
        return true;
    };
    if value.is_null() {
        return true;
    }
    match value {
        crate::json::Value::Bool(b) => *b,
        crate::json::Value::String(s) => {
            let normalized = normalize_id(s);
            normalized != "false" && normalized != "never" && normalized != "0"
        }
        _ => true,
    }
}

/// `expression`: the field compiled, the fallback compiled when the field is
/// missing or does not compile.
fn expression(
    obj: &[(String, crate::json::Value)],
    field: &str,
    fallback: &str,
) -> Option<crate::expressions::Expression> {
    let source = value_of(obj, &[field]).unwrap_or_else(|| fallback.to_string());
    crate::expressions::compile(&source).or_else(|| crate::expressions::compile(fallback))
}

/// `locomotionInfluence`: a number is all three weights at once; an object
/// carries `default`, `walk`/`walking`, `sprint`/`run`, `jump`,
/// `transition` and `hand_only`.
fn locomotion_influence(value: Option<&crate::json::Value>) -> Option<BindLocomotionInfluence> {
    let value = value.filter(|v| !v.is_null())?;
    if let Some(f) = value.as_f64() {
        let f = normalize_locomotion_influence(f as f32);
        return Some(BindLocomotionInfluence {
            walk: f,
            sprint: f,
            jump: f,
            transition: 1.0,
            hand_only: false,
        });
    }
    let obj = value.as_object()?;
    let default = normalize_locomotion_influence(number(obj, "default", 1.0));
    let walk = normalize_locomotion_influence(number(obj, "walk", number(obj, "walking", default)));
    let sprint = normalize_locomotion_influence(number(obj, "sprint", number(obj, "run", default)));
    let jump = normalize_locomotion_influence(number(obj, "jump", default));
    let transition = normalize_locomotion_influence(number(obj, "transition", 1.0));
    let hand_only = member(obj, "hand_only")
        .map(|v| matches!(v, crate::json::Value::Bool(true)))
        .unwrap_or(false);
    Some(BindLocomotionInfluence {
        walk,
        sprint,
        jump,
        transition,
        hand_only,
    })
}

/// `number`: the member as an f32, `fallback` when missing or not a number.
fn number(obj: &[(String, crate::json::Value)], key: &str, fallback: f32) -> f32 {
    member(obj, key)
        .and_then(|v| v.as_f64())
        .map(|f| f as f32)
        .unwrap_or(fallback)
}

/// The Java `normalize`: lowercase, then strip `_`, `-` and spaces.
fn normalize_id(name: &str) -> String {
    name.to_lowercase().replace(['_', '-', ' '], "")
}

/// `parse_overlay_binding`: an overlay binding from one JSON object — a clip
/// name, a mode and an optional bone list.
fn parse_overlay_binding(value: &crate::json::Value) -> Option<OverlayBinding> {
    let obj = value.as_object()?;
    let name = member(obj, "name")
        .or_else(|| member(obj, "clip"))
        .and_then(crate::json::Value::as_str)?;
    let mode = OverlayMode::from_name(member(obj, "mode").and_then(crate::json::Value::as_str));
    let bones = member(obj, "bones")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(OverlayBinding {
        name: name.to_string(),
        mode,
        bones,
    })
}

/// `parse_locomotion_binding`: a locomotion binding from one JSON object.
fn parse_locomotion_binding(value: &crate::json::Value) -> LocomotionBinding {
    let Some(obj) = value.as_object() else {
        return LocomotionBinding {
            clip: String::new(),
            condition: None,
            effect: None,
            overlays: Vec::new(),
            reverse: false,
            add: None,
        };
    };
    LocomotionBinding {
        clip: member(obj, "clip")
            .and_then(crate::json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        condition: member(obj, "condition")
            .and_then(crate::json::Value::as_str)
            .map(str::to_string),
        effect: member(obj, "effect")
            .and_then(crate::json::Value::as_str)
            .map(str::to_string),
        overlays: member(obj, "overlays")
            .and_then(|v| v.as_array())
            .map(|items| items.iter().filter_map(parse_overlay_binding).collect())
            .unwrap_or_default(),
        reverse: member(obj, "reverse")
            .map(|v| matches!(v, crate::json::Value::Bool(true)))
            .unwrap_or(false),
        add: member(obj, "add")
            .and_then(crate::json::Value::as_str)
            .map(str::to_string),
    }
}

/// `normalizeLocomotionInfluence`: a weight over `1.0` reads as a percentage
/// and comes down by a hundred; everything clamps into `0.0..=1.0`, and a
/// broken weight reads as `1.0`.
fn normalize_locomotion_influence(value: f32) -> f32 {
    if !value.is_finite() {
        return 1.0;
    }
    let value = if value > 1.0 { value / 100.0 } else { value };
    value.clamp(0.0, 1.0)
}

impl ItemAnimationBindings {
    /// `resolveSequence`: the sequence's clip names, layered.
    pub fn sequences_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Vec<String> {
        let action = normalize_id(action);
        let at = |sequences: Option<&HashMap<String, Vec<String>>>,
                  entries: Option<&HashMap<String, String>>|
         -> Option<Vec<String>> {
            if let Some(map) = sequences
                && let Some(list) = map.get(&action)
            {
                return Some(list.clone());
            }
            entries
                .and_then(|map| map.get(&action))
                .map(|n| vec![n.clone()])
        };
        match item {
            Some(id) => at(self.specific_sequences.get(id), self.specific_items.get(id))
                .or_else(|| group.and_then(|g| at(self.group_sequences.get(g), self.groups.get(g))))
                .or_else(|| at(Some(&self.default_sequences), Some(&self.defaults)))
                .unwrap_or_default(),
            None => at(Some(&self.empty_hand_sequences), Some(&self.empty_hand))
                .or_else(|| at(Some(&self.default_sequences), Some(&self.defaults)))
                .unwrap_or_default(),
        }
    }
}

/// The numeric setting fields the Java per-action chain reads off a
/// descriptor.
pub(crate) const NUMERIC_SETTING_FIELDS: &[&str] = &[
    "transition_seconds",
    "locomotion_retain",
    "start_at_end",
    "moving_legs_influence",
    "legs_locomotion_retain",
    "over_idle",
    "fence_balance_influence",
    "charge_ticks",
    "minimum_charge_ticks",
    "charge_height",
    "smash_distance",
    "additive",
    "disable_locomotion_arm_swing",
    "stride_blocks_per_cycle",
    "sprint_stride_blocks_per_cycle",
    "crouch_stride_blocks_per_cycle",
    "max_cycles_per_second",
    "sprint_max_cycles_per_second",
    "crouch_max_cycles_per_second",
    "minimum_displacement",
    "maximum_displacement",
    "reverse_backward",
    "amplitude_reference_speed",
    "max_amplitude",
    "horizontal",
    "vertical",
    "depth",
    "fov",
    "response",
    "timer_scale",
];

/// The string setting fields the Java chain reads off a descriptor.
pub(crate) const STRING_SETTING_FIELDS: &[&str] =
    &["behavior", "one_handed", "sprint_overlay", "timer_effect"];

/// Which bindings layer an ingest pass writes to.
#[derive(Clone)]
enum Layer {
    Default,
    EmptyHand,
    Group(String),
    Specific(String),
}

/// Which sub-targets one `addEntries` pass fills; unset kinds are skipped,
/// like the Java nulls in the nine-argument call.
#[derive(Clone, Copy, Default)]
struct EntryKinds {
    pose_idles: bool,
    timer_effects: bool,
    timer_scales: bool,
    numeric: bool,
    strings_to_empty_hand: bool,
    variants: bool,
    locomotion: bool,
}

/// The bindings' lookup side: every layer follows specific item → group →
/// default, with the empty-hand layer standing in for a missing item.
impl ItemAnimationBindings {
    /// `firstFloat`: the layered numeric lookup.
    fn first_f64_layered(
        specific: Option<&HashMap<String, HashMap<String, f64>>>,
        group: Option<&HashMap<String, HashMap<String, f64>>>,
        defaults: &HashMap<String, f64>,
        item: Option<&str>,
        group_key: Option<&str>,
        action: &str,
    ) -> Option<f64> {
        if let (Some(map), Some(id)) = (specific, item)
            && let Some(inner) = map.get(id)
            && let Some(v) = inner.get(action)
        {
            return Some(*v);
        }
        if let (Some(map), Some(g)) = (group, group_key)
            && let Some(inner) = map.get(g)
            && let Some(v) = inner.get(action)
        {
            return Some(*v);
        }
        defaults.get(action).copied()
    }

    /// `resolveSequence`: the layered sequence lookup — specific sequences
    /// and items, then group sequences and groups, then the defaults.
    pub fn resolve_sequence(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Vec<String> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(found) = sequence_at(
                self.specific_sequences.get(id),
                self.specific_items.get(id),
                &action,
            ) {
                return found.clone();
            }
            if let Some(group) = group
                && let Some(found) = sequence_at(
                    self.group_sequences.get(group),
                    self.groups.get(group),
                    &action,
                )
            {
                return found.clone();
            }
            return sequence_at(Some(&self.default_sequences), Some(&self.defaults), &action)
                .unwrap_or_default();
        }
        if let Some(found) = sequence_at(
            Some(&self.empty_hand_sequences),
            Some(&self.empty_hand),
            &action,
        ) {
            return found;
        }
        sequence_at(Some(&self.default_sequences), Some(&self.defaults), &action)
            .unwrap_or_default()
    }

    /// The layered overlay lookup for one item.
    pub fn overlays_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Vec<OverlayBinding> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(map) = self.specific_overlays.get(id)
                && let Some(bindings) = map.get(&action)
            {
                return bindings.clone();
            }
            if let Some(group) = group
                && let Some(map) = self.group_overlays.get(group)
                && let Some(bindings) = map.get(&action)
            {
                return bindings.clone();
            }
            return self
                .default_overlays
                .get(&action)
                .cloned()
                .unwrap_or_default();
        }
        self.default_overlays
            .get(&action)
            .cloned()
            .unwrap_or_default()
    }

    /// The layered blend lookup.
    pub fn blending_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<BlendBinding> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(map) = self.specific_blends.get(id)
                && let Some(binding) = map.get(&action)
            {
                return Some(binding.clone());
            }
            if let Some(group) = group
                && let Some(map) = self.group_blends.get(group)
                && let Some(binding) = map.get(&action)
            {
                return Some(binding.clone());
            }
            return self.default_blends.get(&action).cloned();
        }
        self.empty_hand_blends.get(&action).cloned()
    }

    /// The layered timer lookup.
    pub fn timer_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<TimerBinding> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(map) = self.specific_timers.get(id)
                && let Some(binding) = map.get(&action)
            {
                return Some(binding.clone());
            }
            if let Some(group) = group
                && let Some(map) = self.group_timers.get(group)
                && let Some(binding) = map.get(&action)
            {
                return Some(binding.clone());
            }
            return self.default_timers.get(&action).cloned();
        }
        self.empty_hand_timers.get(&action).cloned()
    }

    /// `retriggerProgress`: the layered percentage divided by 100 and
    /// clamped to `[0, 1]`.
    pub fn retrigger_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
        default: f32,
    ) -> f32 {
        let action = normalize_id(action);
        let value = if let Some(id) = item {
            Self::first_f64_layered(
                Some(&self.specific_retrigger),
                group.map(|_| &self.group_retrigger),
                &self.default_retrigger,
                Some(id),
                group,
                &action,
            )
        } else {
            self.empty_hand_retrigger
                .get(&action)
                .copied()
                .or_else(|| self.default_retrigger.get(&action).copied())
        }
        .unwrap_or(f64::from(default));
        (value / 100.0) as f32
    }

    /// The layered influence lookup.
    pub fn influence_for(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        action: &str,
    ) -> Option<BindLocomotionInfluence> {
        let action = normalize_id(action);
        if let Some(id) = item {
            if let Some(map) = self.specific_influences.get(id)
                && let Some(influence) = map.get(&action)
            {
                return Some(*influence);
            }
            if let Some(group) = group
                && let Some(map) = self.group_influences.get(group)
                && let Some(influence) = map.get(&action)
            {
                return Some(*influence);
            }
            return self.default_influences.get(&action).copied();
        }
        self.empty_hand_influences
            .get(&action)
            .copied()
            .or_else(|| self.default_influences.get(&action).copied())
    }

    /// The layered numeric setting.
    pub fn numeric_setting(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        key: &str,
        default: f32,
    ) -> f32 {
        let key = normalize_id(key);
        if let Some(id) = item {
            return Self::first_f64_layered(
                Some(&self.specific_numeric_settings),
                group.map(|_| &self.group_numeric_settings),
                &self.numeric_settings,
                Some(id),
                group,
                &key,
            )
            .unwrap_or(f64::from(default)) as f32;
        }
        self.numeric_settings
            .get(&key)
            .copied()
            .unwrap_or(f64::from(default)) as f32
    }

    /// The layered string setting.
    pub fn string_setting(
        &self,
        item: Option<&str>,
        group: Option<&str>,
        key: &str,
        default: &str,
    ) -> String {
        let key = normalize_id(key);
        if let Some(id) = item {
            return Self::first(
                self.specific_string_settings.get(id),
                group.and_then(|g| self.group_string_settings.get(g)),
                &self.string_settings,
                &key,
            )
            .cloned()
            .unwrap_or_else(|| default.to_string());
        }
        self.empty_hand_string_settings
            .get(&key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// The variant clip names an action carries.
    pub fn variants_for(&self, action: &str) -> Vec<String> {
        self.variants
            .get(&normalize_id(action))
            .cloned()
            .unwrap_or_default()
    }

    /// The locomotion bindings an action carries.
    pub fn locomotion_for(&self, action: &str) -> Vec<LocomotionBinding> {
        self.locomotion
            .get(&normalize_id(action))
            .cloned()
            .unwrap_or_default()
    }

    /// The one-handed clip an action carries.
    pub fn one_handed_clip(&self, action: &str) -> Option<&String> {
        self.default_one_handed.get(&normalize_id(action))
    }

    /// Whether the action has any default binding at all.
    pub fn has_default_type(&self, action: &str) -> bool {
        self.default_animation_types.contains(&normalize_id(action))
    }

    /// The registered variable bindings.
    pub fn variables(&self) -> &[VariableBinding] {
        &self.variables
    }

    /// The emote menu the bindings carry, if any.
    pub fn emote_catalog(&self) -> Option<EmoteCatalog> {
        if self.emote_bindings.is_empty() {
            return None;
        }
        Some(EmoteCatalog {
            menu_key: self.emote_menu_key.clone().unwrap_or_default(),
            entries: self
                .emote_bindings
                .iter()
                .map(|b| Emote {
                    id: b.id.clone(),
                    display_name: b.display_name.clone(),
                    animation: b.animation.clone(),
                    default_key: b.key.clone().unwrap_or_default(),
                    face: b.face.clone(),
                })
                .collect(),
        })
    }

    /// The emote menu key, if any.
    pub fn emote_menu_key(&self) -> Option<&str> {
        self.emote_menu_key.as_deref()
    }

    /// The emote bindings.
    pub fn emote_bindings(&self) -> &[EmoteBinding] {
        &self.emote_bindings
    }

    /// Whether the action has a static-when-using override at any layer.
    pub fn static_when_using(&self, item: Option<&str>, group: Option<&str>) -> bool {
        if let Some(id) = item {
            if let Some(v) = self.specific_static_when_using.get(id) {
                return *v;
            }
            if let Some(group) = group
                && let Some(v) = self.group_static_when_using.get(group)
            {
                return *v;
            }
            return self.default_static_when_using.unwrap_or(false);
        }
        self.empty_hand_static_when_using
            .or(self.default_static_when_using)
            .unwrap_or(false)
    }
}

/// `sequenceAt`: the sequence list for an action, or the single clip an
/// entries map carries for it.
fn sequence_at(
    sequences: Option<&HashMap<String, Vec<String>>>,
    entries: Option<&HashMap<String, String>>,
    action: &str,
) -> Option<Vec<String>> {
    if let Some(map) = sequences
        && let Some(list) = map.get(action)
    {
        return Some(list.clone());
    }
    if let Some(map) = entries
        && let Some(clip) = map.get(action)
    {
        return Some(vec![clip.clone()]);
    }
    None
}

/// `load_clips_from_str`: parses one animation JSON document (a root object
/// with an `animations` map of name → clip object) into `clips`, like the
/// Java `loadClips`. Malformed entries are skipped, like the Java catch.
pub fn load_clips_from_str(text: &str, clips: &mut HashMap<String, Clip>) -> Result<(), String> {
    let root = crate::json::parse(text)?;
    let Some(members) = root
        .get("animations")
        .and_then(crate::json::Value::as_object)
    else {
        return Ok(());
    };
    for (name, value) in members {
        if let crate::json::Value::Object(_) = value {
            clips.insert(name.clone(), read_clip(name, value));
        }
    }
    Ok(())
}

/// Builds an animation library from one animation JSON document and one
/// bindings JSON document.
pub fn load_library_from_str(
    clips_text: &str,
    bindings_json: Option<&crate::json::Value>,
) -> Result<AnimationLibrary, String> {
    let mut clips = std::collections::HashMap::new();
    load_clips_from_str(clips_text, &mut clips)?;
    let mut bindings = ItemAnimationBindings::new();
    if let Some(bindings_json) = bindings_json {
        bindings.add(bindings_json);
    }
    Ok(AnimationLibrary::new(clips, bindings))
}

/// Normalizes a legacy compat pack into format 2: the Java `CompatFormat`.
/// `Err` carries the validation message (the Java `IllegalArgumentException`).
pub mod compat_format {
    /// The sequence phases the format 2 accepts (`PHASES`).
    pub const PHASES: [&str; 11] = [
        "equip", "idle", "charge", "hold", "release", "slam", "attack", "use", "cast", "fishing",
        "reel",
    ];
    /// The sequence behaviors the format 2 accepts (`BEHAVIORS`).
    pub const BEHAVIORS: [&str; 8] = [
        "mace",
        "bow",
        "trident",
        "flint",
        "shears",
        "fishing",
        "bucket",
        "throwable",
    ];
    /// The per-phase settings the format 2 accepts (`PHASE_SETTINGS`).
    pub const PHASE_SETTINGS: [&str; 8] = [
        "transition_seconds",
        "locomotion_retain",
        "locomotion_influence",
        "retrigger_progress",
        "moving_legs_influence",
        "legs_locomotion_retain",
        "base_pose_retain",
        "over_idle",
    ];

    /// Normalizes a legacy compat pack into format 2. `Err` carries the
    /// validation message (the Java `IllegalArgumentException`).
    pub fn normalize(root: &crate::json::Value) -> Result<crate::json::Value, String> {
        if !root.is_object() {
            return Err("compat: expected an object".to_string());
        }
        let Some(format_version) = root.get("format_version") else {
            for key in [
                "defaults",
                "item_groups",
                "items",
                "empty_hand",
                "cameras",
                "emotes",
            ] {
                if root.has(key) {
                    return Err(error(key, "o formato novo precisa de format_version: 2"));
                }
            }
            return Ok(deep_copy(root));
        };
        let version = format_version.as_str().unwrap_or("");
        if version != "2" {
            return Err(error(
                "format_version",
                "versão suportada: 2 (ou omita para packs antigos)",
            ));
        }
        keys(
            root,
            &[
                "$schema",
                "format_version",
                "defaults",
                "item_groups",
                "items",
                "empty_hand",
                "variables",
                "cameras",
                "emotes",
            ],
            "compat",
        )?;
        let mut out_members: Vec<(String, crate::json::Value)> = Vec::new();
        if let Some(variables) = root.get("variables") {
            out_members.push(("variables".to_string(), deep_copy(variables)));
        }
        // defaults → default_animations.
        let defaults = object_or_empty(root, "defaults", "defaults")?;
        keys(
            defaults,
            &["animations", "static_when_using", "better_combat"],
            "defaults",
        )?;
        let mut defaults_out = crate::json::Value::Object(Vec::new());
        copy_better_combat(defaults, &mut defaults_out, "defaults.better_combat")?;
        if let Some(static_when_using) = defaults.get("static_when_using")
            && let crate::json::Value::Object(out) = &mut defaults_out
        {
            out.push((
                "static_when_using".to_string(),
                deep_copy(static_when_using),
            ));
        }
        let mut default_animations: Vec<crate::json::Value> = Vec::new();
        // defaults.animations → the default entries.
        let defaults_animations = animations(
            defaults.get("animations"),
            &crate::json::Value::Object(Vec::new()),
            false,
            "defaults.animations",
        )?;
        if let crate::json::Value::Array(items) = defaults_animations {
            default_animations.extend(items);
        }
        // cameras → camera entries appended to the default animations.
        let cameras = object_or_empty(root, "cameras", "cameras")?;
        keys(cameras, &["proximity", "aiming"], "cameras")?;
        let Some(camera_members) = cameras.as_object() else {
            return Ok(crate::json::Value::Object(Vec::new()));
        };
        for (camera_name, camera_elem) in camera_members {
            let label = format!("cameras.{}", camera_name);
            let mut camera = deep_copy(object(Some(camera_elem), &label)?);
            let camera_type = if camera_name == "aiming" {
                "bow_and_crossbow_camera_fix"
            } else {
                "proximity_camera"
            };
            set_string(&mut camera, "type", camera_type);
            default_animations.push(camera);
        }
        if let Some(emotes_elem) = root.get("emotes") {
            let label = "emotes";
            let mut emotes = deep_copy(object(Some(emotes_elem), label)?);
            set_string(&mut emotes, "type", "emotes");
            default_animations.push(emotes);
        }
        // empty_hand → the emptyHand layer.
        if let Some(empty_hand) = root.get("empty_hand") {
            let empty_item = item(empty_hand, "empty_hand", false)?;
            if let crate::json::Value::Object(out) = &mut defaults_out {
                // The empty-hand layer rides beside the defaults.
                out.push(("emptyHand".to_string(), empty_item));
            }
        }
        // item_groups and items → per-group and per-item layers.
        for (section, out_key) in [("item_groups", "item"), ("items", "itemSpecific")] {
            let mut items_out: Vec<(String, crate::json::Value)> = Vec::new();
            let section_obj = object_or_empty(root, section, section)?;
            let Some(section_members) = section_obj.as_object() else {
                continue;
            };
            for (entry_name, entry_elem) in section_members {
                let label = format!("{}.{}", section, entry_name);
                let item = item(entry_elem, &label, true)?;
                items_out.push((entry_name.clone(), item));
            }
            out_members.push((out_key.to_string(), crate::json::Value::Object(items_out)));
        }
        if let crate::json::Value::Object(out) = &mut defaults_out {
            out.push((
                "default_animations".to_string(),
                crate::json::Value::Array(default_animations),
            ));
        }
        let mut out = defaults_out;
        if let crate::json::Value::Object(members) = &mut out {
            let ordered: Vec<(String, crate::json::Value)> = std::mem::take(members);
            let mut reordered: Vec<(String, crate::json::Value)> = Vec::new();
            for key in [
                "variables",
                "static_when_using",
                "default_animations",
                "emptyHand",
            ] {
                if let Some(pos) = ordered.iter().position(|(k, _)| k == key) {
                    reordered.push(ordered[pos].clone());
                }
            }
            for entry in ordered {
                if !reordered.iter().any(|(k, _)| k == &entry.0) {
                    reordered.push(entry);
                }
            }
            *members = reordered;
        }
        for (key, value) in out_members {
            if let crate::json::Value::Object(members) = &mut out {
                members.push((key, value));
            }
        }
        legacy_clip_names(&mut out);
        Ok(out)
    }

    /// `legacyClipNames`: every `clip` key becomes `name`, recursively.
    fn legacy_clip_names(elem: &mut crate::json::Value) {
        match elem {
            crate::json::Value::Array(items) => {
                for item in items {
                    legacy_clip_names(item);
                }
            }
            crate::json::Value::Object(members) => {
                if let Some(pos) = members.iter().position(|(k, _)| k == "clip") {
                    let (_, value) = members.remove(pos);
                    members.insert(0, ("name".to_string(), value));
                }
                for (_, value) in members.iter_mut() {
                    legacy_clip_names(value);
                }
            }
            _ => {}
        }
    }

    /// `item`: normalizes one item/group/empty-hand entry into the
    /// `customAnimation` shape the bindings ingest.
    fn item(
        elem: &crate::json::Value,
        label: &str,
        is_item: bool,
    ) -> Result<crate::json::Value, String> {
        let source = object(Some(elem), label)?;
        keys(
            source,
            &[
                "static_when_using",
                "camera",
                "sequence",
                "animations",
                "better_combat",
            ],
            label,
        )?;
        let mut out = crate::json::Value::Object(Vec::new());
        copy_better_combat(source, &mut out, &format!("{}.better_combat", label))?;
        for key in ["static_when_using", "camera"] {
            if let Some(v) = source.get(key)
                && let crate::json::Value::Object(out_obj) = &mut out
            {
                out_obj.push((key.to_string(), deep_copy(v)));
            }
        }
        let sequence = object_or_empty(source, "sequence", &format!("{}.sequence", label))?;
        let has_sequence = source.has("sequence");
        if has_sequence && !is_item {
            return Err(error(label, "sequence só pode ser usado em itens"));
        }
        keys(
            sequence,
            &[
                "behavior",
                "defaults",
                "charge_ticks",
                "minimum_charge_ticks",
                "charge_height",
                "smash_distance",
                "legs_locomotion_retain",
            ],
            &format!("{}.sequence", label),
        )?;
        let sequence_defaults = object_or_empty(
            sequence,
            "defaults",
            &format!("{}.sequence.defaults", label),
        )?;
        keys(
            sequence_defaults,
            &PHASE_SETTINGS,
            &format!("{}.sequence.defaults", label),
        )?;
        let mut custom: Vec<crate::json::Value> = Vec::new();
        if has_sequence {
            let behavior = string(
                sequence.get("behavior"),
                &format!("{}.sequence.behavior", label),
            )?;
            if !BEHAVIORS.contains(&behavior.as_str()) {
                return Err(error(
                    &format!("{}.sequence.behavior", label),
                    &format!("comportamento desconhecido: {}", behavior),
                ));
            }
            let mut merged = merge(sequence_defaults, sequence);
            if let crate::json::Value::Object(m) = &mut merged {
                m.retain(|(k, _)| k != "defaults");
            }
            if let Some(retain) = sequence_defaults.get("base_pose_retain") {
                // Java validates the retention here, at normalize time.
                crate::signals::BasePoseRetention::parse(
                    Some(retain),
                    &format!("{}.sequence.defaults.base_pose_retain", label),
                )?;
            }
            set_string(&mut merged, "type", "item_sequence");
            custom.push(merged);
        }
        let source_animations = source.get("animations");
        let parsed = animations(
            source_animations,
            sequence_defaults,
            has_sequence,
            &format!("{}.animations", label),
        )?;
        if let crate::json::Value::Array(items) = &parsed {
            custom.extend(items.iter().cloned());
        }
        if let crate::json::Value::Object(out_obj) = &mut out {
            out_obj.push((
                "customAnimation".to_string(),
                crate::json::Value::Array(custom),
            ));
        }
        Ok(out)
    }

    /// `animations`: normalizes one action-to-clip map into an array of clip
    /// descriptors.
    fn animations(
        elem: Option<&crate::json::Value>,
        parent: &crate::json::Value,
        is_phase: bool,
        label: &str,
    ) -> Result<crate::json::Value, String> {
        let mut out: Vec<crate::json::Value> = Vec::new();
        let Some(elem) = elem else {
            return Ok(crate::json::Value::Array(out));
        };
        let source = object(Some(elem), label)?;
        let Some(source_members) = source.as_object() else {
            return Ok(crate::json::Value::Array(out));
        };
        for (action_name, value) in source_members {
            let mut action = action_name.clone();
            if action == "useItem" || action == "useitem" {
                action = "use_item".to_string();
            }
            let full_label = format!("{}.{}", label, action);
            if action
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
                && action
                    .chars()
                    .skip(1)
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                && !action.starts_with("sequence_")
                && action != "item_sequence"
            {
                // A valid action name.
            } else {
                return Err(error(
                    &full_label,
                    "use o nome da ação, sem sequence_ ou type",
                ));
            }
            if is_phase && (action.starts_with("attack_") || action.ends_with("_one_handed")) {
                return Err(error(
                    &full_label,
                    "mantenha variants e one_handed dentro da ação",
                ));
            }
            let is_phase_name = is_phase && PHASES.contains(&action.as_str());
            let final_name = if is_phase_name {
                format!("sequence_{}", action)
            } else {
                action.clone()
            };
            if value.is_array() {
                if !["biome", "passenger_state", "sitting_state"].contains(&action.as_str()) {
                    return Err(error(
                        &full_label,
                        "use um objeto com variants para variações",
                    ));
                }
                let Some(items) = value.as_array() else {
                    continue;
                };
                for item in items {
                    emit(
                        &mut out,
                        &final_name,
                        item,
                        &crate::json::Value::Object(Vec::new()),
                        false,
                        &full_label,
                    )?;
                }
            } else {
                let parent_for_merge = if is_phase_name {
                    parent
                } else {
                    &crate::json::Value::Object(Vec::new())
                };
                emit(
                    &mut out,
                    &final_name,
                    value,
                    parent_for_merge,
                    is_phase_name,
                    &full_label,
                )?;
            }
        }
        Ok(crate::json::Value::Array(out))
    }

    /// `emit`: normalizes one animation descriptor into the format 2 shape.
    fn emit(
        out: &mut Vec<crate::json::Value>,
        name: &str,
        elem: &crate::json::Value,
        parent: &crate::json::Value,
        is_phase: bool,
        label: &str,
    ) -> Result<(), String> {
        let mut desc = descriptor(elem, label)?;
        if name == "turn_in_place" {
            if desc.has("left_overlay")
                && !desc.has("left_add")
                && let Some(v) = desc.get("left_overlay").cloned()
            {
                set_member(&mut desc, "left_add", v);
            }
            if desc.has("right_overlay")
                && !desc.has("right_add")
                && let Some(v) = desc.get("right_overlay").cloned()
            {
                set_member(&mut desc, "right_add", v);
            }
            desc.remove_member("left_overlay");
            desc.remove_member("right_overlay");
            keys(
                &desc,
                &[
                    "enabled",
                    "mode",
                    "clip",
                    "reference_degrees",
                    "double_turn_degrees",
                    "minimum_turn_degrees",
                    "turn_delay_seconds",
                    "look_idle_seconds",
                    "fade_out_seconds",
                    "left",
                    "right",
                    "left_add",
                    "right_add",
                ],
                label,
            )?;
            if let Some(mode) = desc.get("mode") {
                let valid = matches!(mode, crate::json::Value::String(s)
                    if matches!(s.as_str(), "scaled" | "steps" | "camera" | "dynamic" | "procedural"));
                if !valid {
                    return Err(error(label, "use dynamic, camera, scaled ou steps"));
                }
            }
            let scaled = desc
                .get("mode")
                .and_then(crate::json::Value::as_str)
                .map(|s| s == "scaled")
                .unwrap_or(false);
            if scaled
                && (desc.has("left")
                    || desc.has("right")
                    || desc.has("left_add")
                    || desc.has("right_add"))
            {
                return Err(error(
                    label,
                    "scaled usa um clip; left e right pertencem ao modo steps",
                ));
            }
            if !scaled && desc.has("clip") {
                return Err(error(label, "declare mode: scaled para usar um único clip"));
            }
            if let Some(enabled) = desc.get("enabled")
                && !matches!(enabled, crate::json::Value::Bool(_))
            {
                return Err(error(label, "use true/false"));
            }
            if desc.has("left") != desc.has("right") {
                return Err(error(label, "declare left e right juntos"));
            }
            for key in ["left", "right", "left_add", "right_add", "clip"] {
                if let Some(v) = desc.get(key) {
                    let blank = match v {
                        crate::json::Value::String(s) => s.trim().is_empty(),
                        _ => false,
                    };
                    if blank {
                        return Err(error(
                            &format!("{}.{}", label, key),
                            "use o nome de um clip",
                        ));
                    }
                }
            }
            if let Some(degrees) = desc.get("reference_degrees") {
                let valid = matches!(degrees, crate::json::Value::Number(n)
                    if n.is_finite() && *n as f32 > 0.0);
                if !valid {
                    return Err(error(label, "use um número positivo"));
                }
            }
            if let Some(degrees) = desc.get("double_turn_degrees") {
                let valid = matches!(degrees, crate::json::Value::Number(n)
                    if n.is_finite() && (*n as f32) >= 30.0 && (*n as f32) <= 180.0);
                if !valid {
                    return Err(error(label, "use graus entre 30 e 180"));
                }
            }
            if let Some(seconds) = desc.get("turn_delay_seconds") {
                let valid = matches!(seconds, crate::json::Value::Number(n)
                    if n.is_finite() && (*n as f32) >= 0.0 && (*n as f32) <= 0.3);
                if !valid {
                    return Err(error(label, "use segundos entre 0 e 0.3"));
                }
            }
        }
        if is_phase {
            let mut allowed: Vec<&str> = PHASE_SETTINGS.to_vec();
            allowed.extend([
                "clip",
                "variants",
                "one_handed",
                "hold_last_frame",
                "overlays",
                "one_handed_overlays",
            ]);
            keys(&desc, &allowed, label)?;
        }
        if desc.has("type") || desc.has("name") || desc.has("start_at_end") {
            return Err(error(
                label,
                "use a chave da ação, clip e hold_last_frame no formato 2",
            ));
        }
        if let Some(members) = desc.as_object() {
            for (key, _) in members {
                if key.starts_with("var_")
                    && key[4..].chars().all(|c| c.is_ascii_digit())
                    && key.len() > 4
                {
                    return Err(error(label, "use variants em vez de var_N"));
                }
            }
        }
        let mut merged = merge(parent, &desc);
        if let Some(retain) = merged.get("base_pose_retain") {
            let allowed = ["attack", "mining", "pose", "use_item"]
                .iter()
                .any(|k| name == *k || name.starts_with("sequence_"));
            if !is_phase && !allowed {
                return Err(error(
                    label,
                    "disponível em attack, mining, pose, use_item e fases de sequence",
                ));
            }
            let _ = retain; // BasePoseRetention.parse runs in the game layer.
        }
        set_string(&mut merged, "type", name);
        if merged.has("clip") {
            if let Some(v) = merged.get("clip").cloned() {
                set_member(&mut merged, "name", v);
            }
            merged.remove_member("clip");
        }
        if let Some(hold) = merged.get("hold_last_frame").cloned() {
            merged.remove_member("hold_last_frame");
            if !is_phase || !matches!(hold, crate::json::Value::Bool(_)) {
                return Err(error(label, "use true/false em uma fase de sequence"));
            }
            let start = matches!(hold, crate::json::Value::Bool(true));
            set_number(&mut merged, "start_at_end", if start { 1.0 } else { 0.0 });
        }
        let variants = merged.get("variants").cloned();
        if let Some(variants) = variants {
            merged.remove_member("variants");
            if merged.has("name") {
                return Err(error(label, "escolha clip ou variants"));
            }
            let Some(items) = variants.as_array().filter(|a| !a.is_empty()) else {
                return Err(error(label, "informe uma lista não vazia"));
            };
            if name == "greeting" || name == "idle_events" {
                set_member(&mut merged, "variants", variants);
            } else {
                for (index, item) in items.iter().enumerate() {
                    let index = index + 1;
                    let variant_label = format!("{}.variants[{}]", label, index - 1);
                    let desc = descriptor(item, &variant_label)?;
                    let clip = string(desc.get("clip"), &variant_label)?;
                    set_string(&mut merged, &format!("var_{}", index), &clip);
                    if name == "sequence_attack" {
                        return Err(error(label, "variants de sequência pertence à ação attack"));
                    }
                    let mut copy = deep_copy(&desc);
                    copy.remove_member("variants");
                    let named = format!("{}_{}", name, index);
                    let merged_variant = merge(&copy, &desc);
                    emit_entry(&mut *out, &named, merged_variant)?;
                }
            }
        } else if name == "sequence_attack" && merged.has("name") {
            let named = format!("{}_1", name);
            let mut copy = deep_copy(&desc);
            copy.remove_member("variants");
            let mut merged_variant = merge(&copy, &desc);
            if let Some(v) = merged.get("name").cloned() {
                set_member(&mut merged_variant, "var_1", v);
            }
            emit_entry(out, &named, merged_variant)?;
        }
        if name == "climbing" {
            if merged.has("up") {
                if !merged.has("name")
                    && let Some(v) = merged.get("up").cloned()
                {
                    set_member(&mut merged, "name", v);
                }
                let mut copy = deep_copy(&merged);
                copy.remove_member("up");
                let named = "climb_up".to_string();
                emit_entry(out, &named, copy)?;
            }
            if merged.has("down") {
                let mut copy = deep_copy(&merged);
                copy.remove_member("down");
                emit_entry(out, "climb_down", copy)?;
            }
        }
        let one_handed = merged.get("one_handed");
        if let Some(one_handed) = one_handed {
            if one_handed.as_object().is_some() {
                if !is_phase {
                    return Err(error(
                        label,
                        "fora de sequence, informe apenas o nome do clip",
                    ));
                }
                let desc = descriptor(one_handed, &format!("{}.one_handed", label))?;
                let clip = string(desc.get("clip"), &format!("{}.one_handed.clip", label))?;
                set_string(&mut merged, "one_handed", &clip);
                let mut copy = deep_copy(&merged);
                copy.remove_member("one_handed");
                copy.remove_member("variants");
                let named = format!("{}_one_handed", name);
                let merged_one_handed = merge(&copy, &desc);
                emit_entry(out, &named, merged_one_handed)?;
            } else if is_phase && !one_handed.is_null() {
                let clip = string(Some(one_handed), &format!("{}.one_handed", label))?;
                let mut copy = deep_copy(&merged);
                copy.remove_member("one_handed");
                copy.remove_member("variants");
                set_string(&mut copy, "clip", &clip);
                let named = format!("{}_one_handed", name);
                emit_entry(out, &named, copy)?;
            }
        }
        for key in ["overlays", "one_handed_overlays"] {
            if let Some(overlays) = merged.get(key) {
                let Some(items) = overlays.as_array() else {
                    return Err(error(&format!("{}.{}", label, key), "esperada uma lista"));
                };
                for item in items {
                    let overlay_label = format!("{}.{}", label, key);
                    let mut obj = deep_copy(object(Some(item), &overlay_label)?);
                    if obj.has("clip") {
                        if let Some(v) = obj.get("clip").cloned() {
                            set_member(&mut obj, "name", v);
                        }
                        obj.remove_member("clip");
                    }
                }
            }
        }
        out.push(merged);
        Ok(())
    }

    /// `descriptor`: a bare string becomes `{clip: s}`, an object is copied.
    fn descriptor(elem: &crate::json::Value, label: &str) -> Result<crate::json::Value, String> {
        if let crate::json::Value::String(s) = elem
            && !s.trim().is_empty()
        {
            let mut obj = crate::json::Value::Object(Vec::new());
            set_string(&mut obj, "clip", s);
            return Ok(obj);
        }
        Ok(deep_copy(object(Some(elem), label)?))
    }

    /// `copyBetterCombat`: the better_combat block is copied verbatim (its
    /// validation runs in the game layer).
    fn copy_better_combat(
        source: &crate::json::Value,
        out: &mut crate::json::Value,
        _label: &str,
    ) -> Result<(), String> {
        if let Some(v) = source.get("better_combat")
            && let crate::json::Value::Object(out_obj) = out
        {
            out_obj.push(("better_combat".to_string(), deep_copy(v)));
        }
        Ok(())
    }

    /// `merge`: a recursive object merge; `base_pose_retain` merges
    /// recursively, everything else overwrites.
    fn merge(base: &crate::json::Value, overlay: &crate::json::Value) -> crate::json::Value {
        let Some(base_obj) = base.as_object() else {
            return deep_copy(overlay);
        };
        let mut out: Vec<(String, crate::json::Value)> = base_obj.to_vec();
        if let Some(overlay_obj) = overlay.as_object() {
            for (key, value) in overlay_obj {
                if key == "base_pose_retain" {
                    let existing = out.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
                    if let (
                        Some(crate::json::Value::Object(_)),
                        Some(crate::json::Value::Object(_)),
                    ) = (existing.as_ref(), Some(value))
                    {
                        let base_member = existing.as_ref().unwrap();
                        let merged = merge(base_member, value);
                        if let Some(pos) = out.iter().position(|(k, _)| k == key) {
                            out[pos] = (key.clone(), merged);
                        }
                        continue;
                    }
                }
                if let Some(pos) = out.iter().position(|(k, _)| k == key) {
                    out[pos] = (key.clone(), deep_copy(value));
                } else {
                    out.push((key.clone(), deep_copy(value)));
                }
            }
        }
        crate::json::Value::Object(out)
    }

    /// `objectOrEmpty`: the member as an object, or an empty object.
    fn object_or_empty<'a>(
        root: &'a crate::json::Value,
        key: &str,
        label: &str,
    ) -> Result<&'a crate::json::Value, String> {
        if let Some(v) = root.get(key) {
            object(Some(v), label)?;
            return Ok(v);
        }
        static EMPTY_OBJECT: crate::json::Value = crate::json::Value::Object(Vec::new());
        Ok(&EMPTY_OBJECT)
    }

    /// `object`: the element as an object, or a validation error.
    fn object<'a>(
        elem: Option<&'a crate::json::Value>,
        label: &str,
    ) -> Result<&'a crate::json::Value, String> {
        match elem {
            Some(v @ crate::json::Value::Object(_)) => Ok(v),
            _ => Err(error(label, "esperado um objeto")),
        }
    }

    /// `string`: a non-empty string, or a validation error.
    fn string(elem: Option<&crate::json::Value>, label: &str) -> Result<String, String> {
        match elem {
            Some(crate::json::Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
            _ => Err(error(label, "esperado um nome não vazio")),
        }
    }

    /// `keys`: every object key must be in the allowed set.
    fn keys(obj: &crate::json::Value, allowed: &[&str], label: &str) -> Result<(), String> {
        let Some(members) = obj.as_object() else {
            return Ok(());
        };
        for (key, _) in members {
            if !allowed.contains(&key.as_str()) {
                return Err(error(&format!("{}.{}", label, key), "campo desconhecido"));
            }
        }
        Ok(())
    }

    /// `error`: the Java `IllegalArgumentException` message shape.
    fn error(label: &str, message: &str) -> String {
        format!("Compat {}: {}", label, message)
    }

    /// A deep copy of a JSON value.
    fn deep_copy(value: &crate::json::Value) -> crate::json::Value {
        value.clone()
    }

    fn set_string(value: &mut crate::json::Value, key: &str, s: &str) {
        if let crate::json::Value::Object(members) = value {
            if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                members[pos] = (key.to_string(), crate::json::Value::String(s.to_string()));
            } else {
                members.push((key.to_string(), crate::json::Value::String(s.to_string())));
            }
        }
    }

    fn set_number(value: &mut crate::json::Value, key: &str, n: f64) {
        if let crate::json::Value::Object(members) = value {
            if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                members[pos] = (key.to_string(), crate::json::Value::Number(n));
            } else {
                members.push((key.to_string(), crate::json::Value::Number(n)));
            }
        }
    }

    fn set_member(value: &mut crate::json::Value, key: &str, member: crate::json::Value) {
        if let crate::json::Value::Object(members) = value {
            if let Some(pos) = members.iter().position(|(k, _)| k == key) {
                members[pos] = (key.to_string(), member);
            } else {
                members.push((key.to_string(), member));
            }
        }
    }

    /// Emits one finished descriptor into the output array, applying the
    /// same tail `emit` applies: `type` = the name, `clip` becomes `name`.
    fn emit_entry(
        out: &mut Vec<crate::json::Value>,
        name: &str,
        mut merged: crate::json::Value,
    ) -> Result<(), String> {
        set_string(&mut merged, "type", name);
        if merged.has("clip") {
            if let Some(v) = merged.get("clip").cloned() {
                set_member(&mut merged, "name", v);
            }
            merged.remove_member("clip");
        }
        out.push(merged);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn channel_from(keys: Vec<(f32, Vec3)>) -> Channel {
        Channel::new(
            keys.into_iter()
                .map(|(t, v)| {
                    (
                        t,
                        Key {
                            pre: v,
                            post: v,
                            interpolation: "linear".into(),
                            easing: None,
                        },
                    )
                })
                .collect(),
            Vec3::ZERO,
        )
    }

    #[test]
    fn vec3_lerp_and_componentwise_mul() {
        assert_eq!(
            v(1.0, 2.0, 3.0).lerp(v(5.0, 6.0, 7.0), 0.25),
            v(2.0, 3.0, 4.0)
        );
        assert_eq!(v(1.0, 2.0, 3.0) * v(2.0, 3.0, 4.0), v(2.0, 6.0, 12.0));
        assert_eq!(v(1.0, 2.0, 3.0) + v(1.0, 1.0, 1.0), v(2.0, 3.0, 4.0));
        assert_eq!(v(3.0, 2.0, 1.0) - v(1.0, 1.0, 1.0), v(2.0, 1.0, 0.0));
    }

    /// The quaternion math matches JOML 1.10.8 exactly (values probed from
    /// joml-1.10.8 on the JVM).
    #[test]
    fn quaternion_math_matches_joml() {
        let q = Quat::from_euler_zyx(60.0, 45.0, 30.0);
        assert!((q.x - 0.02226001).abs() < 1.0e-5, "x {}", q.x);
        assert!((q.y - 0.43967977).abs() < 1.0e-5, "y {}", q.y);
        assert!((q.z - 0.3604234).abs() < 1.0e-5, "z {}", q.z);
        assert!((q.w - 0.82236314).abs() < 1.0e-5, "w {}", q.w);
        // A pure X rotation round-trips through getEulerAnglesZYX.
        let q = Quat::from_euler_zyx(0.0, 0.0, 90.0);
        let e = q.get_euler_zyx();
        assert!((e.x - core::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
        // qz(90) * qx(90) sends (1,0,0) to (0,1,0): `other` applies first.
        let qz = Quat::from_euler_zyx(90.0, 0.0, 0.0);
        let qx = Quat::from_euler_zyx(0.0, 0.0, 90.0);
        let combined = qz * qx;
        let out = combined.transform_by(v(1.0, 0.0, 0.0));
        assert!((out.x - 0.0).abs() < 1.0e-5);
        assert!((out.y - 1.0).abs() < 1.0e-5);
        assert!((out.z - 0.0).abs() < 1.0e-5);
        // JOML's transform normalizes by the quaternion's squared length.
        let q = Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 2.0,
        };
        let out = q.transform_by(v(3.0, 4.0, 5.0));
        assert!((out.x - 3.0).abs() < 1.0e-5, "unnormalized: {}", out.x);
    }

    /// Linear interpolation between two keys, with the ceiling key's easing
    /// applied to the alpha.
    #[test]
    fn channel_samples_linear_and_eased() {
        let keys = vec![
            (
                0.0,
                Key {
                    pre: v(0.0, 0.0, 0.0),
                    post: v(0.0, 0.0, 0.0),
                    interpolation: "linear".into(),
                    easing: None,
                },
            ),
            (
                1.0,
                Key {
                    pre: v(10.0, 0.0, 0.0),
                    post: v(10.0, 0.0, 0.0),
                    interpolation: "linear".into(),
                    easing: None,
                },
            ),
        ];
        let channel = Channel::new(keys, Vec3::ZERO);
        assert_eq!(channel.sample(0.5), v(5.0, 0.0, 0.0));
        // Before the first key: its pre value; after the last: its post.
        assert_eq!(channel.sample(-1.0), v(0.0, 0.0, 0.0));
        assert_eq!(channel.sample(2.0), v(10.0, 0.0, 0.0));
        // The ceiling key's easing shapes the alpha.
        let keys = vec![
            (
                0.0,
                Key {
                    pre: v(0.0, 0.0, 0.0),
                    post: v(0.0, 0.0, 0.0),
                    interpolation: "linear".into(),
                    easing: None,
                },
            ),
            (
                1.0,
                Key {
                    pre: v(10.0, 0.0, 0.0),
                    post: v(10.0, 0.0, 0.0),
                    interpolation: "linear".into(),
                    easing: Some(Easing::EaseInQuad),
                },
            ),
        ];
        let channel = Channel::new(keys, Vec3::ZERO);
        assert_eq!(channel.sample(0.5), v(2.5, 0.0, 0.0));
    }

    /// Catmull-Rom through the midpoints of a straight line stays straight.
    #[test]
    fn catmullrom_through_a_straight_line_stays_straight() {
        let keys: Vec<(f32, Key)> = [(0.0, 0.0f32), (1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]
            .into_iter()
            .map(|(t, x)| {
                (
                    t,
                    Key {
                        pre: v(x, 0.0, 0.0),
                        post: v(x, 0.0, 0.0),
                        interpolation: "catmullrom".into(),
                        easing: None,
                    },
                )
            })
            .collect();
        let channel = Channel::new(keys, Vec3::ZERO);
        let mid = channel.sample(1.5);
        assert!((mid.x - 15.0).abs() < 1.0e-4, "mid {}", mid.x);
        assert!(mid.y.abs() < 1.0e-4 && mid.z.abs() < 1.0e-4);
    }

    #[test]
    fn channel_endpoints_and_last_key_time() {
        let channel = channel_from(vec![(0.0, v(1.0, 0.0, 0.0)), (2.0, v(3.0, 0.0, 0.0))]);
        assert_eq!(channel.sample(-1.0), v(1.0, 0.0, 0.0));
        assert_eq!(channel.sample(3.0), v(3.0, 0.0, 0.0));
        assert_eq!(channel.last_key_time(), 2.0);
        let empty = Channel::new(Vec::new(), v(4.0, 5.0, 6.0));
        assert_eq!(empty.sample(1.0), v(4.0, 5.0, 6.0));
        assert_eq!(empty.last_key_time(), 0.0);
    }

    #[test]
    fn clip_samples_and_loops() {
        let mut bones = std::collections::HashMap::new();
        bones.insert(
            "body".to_string(),
            BoneTrack {
                position: channel_from(vec![(0.0, v(0.0, 0.0, 0.0)), (1.0, v(10.0, 0.0, 0.0))]),
                rotation: Channel::new(Vec::new(), Vec3::ZERO),
                scale: Channel::new(Vec::new(), Vec3::ONE),
            },
        );
        let looping = Clip::new("walk".into(), 1.0, true, bones.clone());
        let pose = looping.sample("BODY", 1.5);
        assert!(
            (pose.position.x - 5.0).abs() < 1.0e-5,
            "{}",
            pose.position.x
        );
        // A non-looping clip clamps at its length.
        let once = Clip::new("hit".into(), 1.0, false, bones);
        let pose = once.sample("body", 5.0);
        assert!((pose.position.x - 10.0).abs() < 1.0e-5);
        // An unknown bone samples to identity.
        let pose = once.sample("nope", 0.5);
        assert_eq!(pose, Pose::identity());
    }

    #[test]
    fn positive_modulo_wraps_negatives() {
        assert_eq!(positive_modulo(1.5, 1.0), 0.5);
        assert_eq!(positive_modulo(-0.5, 1.0), 0.5);
        assert_eq!(positive_modulo(2.5, 1.0), 0.5);
    }

    #[test]
    fn parse_channel_reads_all_key_shapes() {
        // A bare array is one symmetric key at time 0.
        let channel = parse_channel(Some(&crate::json::parse("[1,2,3]").unwrap()), Vec3::ZERO);
        assert_eq!(channel.sample(0.0), v(1.0, 2.0, 3.0));
        assert_eq!(channel.last_key_time(), 0.0);
        // An object maps time strings to keys; a bare array value is a
        // symmetric key, an object carries pre/post/easing.
        let obj = crate::json::parse(
            r#"{"0.0": [0,0,0], "1.0": {"pre": [10,0,0], "post": [10,0,0], "easing": "easeInQuad"}}"#,
        )
        .unwrap();
        let channel = parse_channel(Some(&obj), Vec3::ZERO);
        assert_eq!(channel.last_key_time(), 1.0);
        let mid = channel.sample(0.5);
        // The ceiling key's easing (easeInQuad at alpha 0.5 → 0.25).
        assert!((mid.x - 2.5).abs() < 1.0e-5, "{}", mid.x);
        // A time that does not parse skips the entry.
        let bad = crate::json::parse(r#"{"abc": [1,2,3]}"#).unwrap();
        let channel = parse_channel(Some(&bad), Vec3::ZERO);
        assert!(channel.keys.is_empty());
        // A null element is an empty channel on the fallback.
        let channel = parse_channel(None, v(4.0, 5.0, 6.0));
        assert!(channel.keys.is_empty());
        assert_eq!(channel.sample(1.0), v(4.0, 5.0, 6.0));
    }

    #[test]
    fn vector_reads_scalars_and_arrays() {
        let scalar = crate::json::parse("2").unwrap();
        assert_eq!(vector(Some(&scalar), Vec3::ZERO), v(2.0, 2.0, 2.0));
        let arr = crate::json::parse("[1, 2, 3]").unwrap();
        assert_eq!(vector(Some(&arr), Vec3::ZERO), v(1.0, 2.0, 3.0));
        let short = crate::json::parse("[1, 2]").unwrap();
        assert_eq!(vector(Some(&short), v(7.0, 8.0, 9.0)), v(7.0, 8.0, 9.0));
        let with_default = crate::json::parse("[1]").unwrap();
        assert_eq!(
            vector(Some(&with_default), v(7.0, 8.0, 9.0)),
            v(7.0, 8.0, 9.0)
        );
        assert_eq!(vector(None, v(7.0, 8.0, 9.0)), v(7.0, 8.0, 9.0));
    }

    #[test]
    fn parse_clip_reads_the_animation_shape() {
        let obj = crate::json::parse(
            r#"{
                "animation_length": 2.0,
                "loop": true,
                "bones": {
                    "Body": {
                        "position": {"0.0": [0,0,0], "1.0": [10,0,0]},
                        "rotation": [0,0,0],
                        "scale": [1,1,1]
                    }
                }
            }"#,
        )
        .unwrap();
        let clip = parse_clip("walk", &obj);
        assert_eq!(clip.name, "walk");
        assert_eq!(clip.length, 2.0);
        assert!(clip.loop_);
        assert!(clip.bones.contains_key("body"));
        let pose = clip.sample("BODY", 0.5);
        assert!((pose.position.x - 5.0).abs() < 1.0e-5);
        // A non-looping clip stretches to the last key's time.
        let obj = crate::json::parse(
            r#"{"bones": {"body": {"position": {"0.0": [0,0,0], "2.5": [5,0,0]}}}}"#,
        )
        .unwrap();
        let clip = parse_clip("hit", &obj);
        assert!((clip.length - 2.5).abs() < 1.0e-5);
        // The length never drops below 1e-4.
        let obj = crate::json::parse("{}").unwrap();
        let clip = parse_clip("empty", &obj);
        assert!((clip.length - 1.0e-4).abs() < 1.0e-9);
    }

    #[test]
    fn face_shorthand_installs_the_face_bones() {
        let obj = crate::json::parse(r#"{"face": {"look": "Happy", "mix": 0.5}}"#).unwrap();
        let mut bones = std::collections::HashMap::new();
        face_shorthand(&obj, &mut bones);
        assert!(bones.contains_key("face_mix"));
        assert!(bones.contains_key("face_happy"));
        // The mix bone carries the mix weight; the named bone is a marker.
        assert!((bones["face_mix"].position.sample(0.0).y - 0.5).abs() < 1.0e-6);
        assert!((bones["face_happy"].position.sample(0.0).y - 1.0).abs() < 1.0e-6);
        // A bare string is the look with the default mix.
        let obj = crate::json::parse(r#"{"face": "sad face"}"#).unwrap();
        let mut bones = std::collections::HashMap::new();
        face_shorthand(&obj, &mut bones);
        assert!(bones.contains_key("face_sad_face"));
        assert!((bones["face_mix"].position.sample(0.0).y - 1.0).abs() < 1.0e-6);
        // Blank or missing faces install nothing.
        let obj = crate::json::parse(r#"{"face": "  "}"#).unwrap();
        let mut bones = std::collections::HashMap::new();
        face_shorthand(&obj, &mut bones);
        assert!(bones.is_empty());
    }

    #[test]
    fn load_clips_reads_the_animation_document() {
        let mut clips = HashMap::new();
        load_clips_from_str(
            r#"{"animations": {"walk": {"animation_length": 1.0, "bones": {"body": {"position": {"0.0": [0,0,0], "1.0": [10,0,0]}}}}}}"#,
            &mut clips,
        )
        .unwrap();
        assert_eq!(clips.len(), 1);
        let clip = clips.get("walk").unwrap();
        assert!((clip.length - 1.0).abs() < 1.0e-5);
        let pose = clip.sample("body", 0.5);
        assert!((pose.position.x - 5.0).abs() < 1.0e-5);
        // A document without an animations map loads nothing.
        let mut clips = HashMap::new();
        load_clips_from_str("{}", &mut clips).unwrap();
        assert!(clips.is_empty());
    }

    #[test]
    fn resolved_overlay_applies_to_bones() {
        let clip = Clip::new("ov".into(), 1.0, false, HashMap::new());
        let overlay = ResolvedOverlay {
            clip,
            mode: OverlayMode::Additive,
            bones: vec!["left_arm".to_string()],
        };
        assert!(overlay.applies_to("left_arm"));
        assert!(!overlay.applies_to("right_arm"));
        let overlay = ResolvedOverlay {
            clip: Clip::new("ov".into(), 1.0, false, HashMap::new()),
            mode: OverlayMode::Override,
            bones: Vec::new(),
        };
        assert!(overlay.applies_to("anything"));
    }

    #[test]
    fn overlay_mode_parses_the_java_names() {
        assert_eq!(OverlayMode::from_name(None), OverlayMode::Additive);
        assert_eq!(
            OverlayMode::from_name(Some("additive")),
            OverlayMode::Additive
        );
        assert_eq!(
            OverlayMode::from_name(Some("override")),
            OverlayMode::Override
        );
        assert_eq!(
            OverlayMode::from_name(Some("exclusive")),
            OverlayMode::Exclusive
        );
        assert_eq!(OverlayMode::from_name(Some("nope")), OverlayMode::Additive);
    }

    #[test]
    fn locomotion_influence_selects_by_state() {
        let influence = LocomotionInfluence::new(1.0, 2.0, 3.0);
        assert_eq!(influence.select(false, false), 1.0);
        assert_eq!(influence.select(true, false), 2.0);
        assert_eq!(influence.select(false, true), 3.0);
        assert_eq!(influence.select(true, true), 3.0);
    }

    #[test]
    fn normalize_id_strips_separators() {
        assert_eq!(normalize_id("ease_In-Out Sine"), "easeinoutsine");
        assert_eq!(normalize_id("Walk-Forward"), "walkforward");
    }

    #[test]
    fn turn_in_place_copies_overlays_to_the_add_layers() {
        let root = crate::json::parse(
            r#"{
                "format_version": "2",
                "items": {
                    "minecraft:stick": {
                        "animations": {
                            "turn_in_place": {
                                "left_overlay": {"clip": "ov_left"},
                                "right_overlay": {"clip": "ov_right"}
                            }
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let out = compat_format::normalize(&root).unwrap();
        let entry = out
            .get("itemSpecific")
            .and_then(|v| v.get("minecraft:stick"))
            .and_then(|v| v.get("customAnimation"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .expect("turn_in_place entry");
        // The overlays became the add layers; the overlay keys are gone.
        assert_eq!(
            entry.get("left_add").and_then(|v| v.get("name")),
            Some(&crate::json::Value::String("ov_left".to_string()))
        );
        assert_eq!(
            entry.get("right_add").and_then(|v| v.get("name")),
            Some(&crate::json::Value::String("ov_right".to_string()))
        );
        assert!(entry.get("left_overlay").is_none());
        assert!(entry.get("right_overlay").is_none());
    }
    #[test]
    fn entries_ingest_from_the_java_array_shape() {
        let root = crate::json::parse(
            r#"{
                "format_version": "2",
                "defaults": {
                    "animations": {
                        "attack": {"clip": "swing"},
                        "use_item": {"clip": "drink", "base_pose_retain": {"right_arm": 0.5}},
                        "idle_events": {"variants": ["wave", {"clip": "clap", "name": "clap"}]},
                        "emotes": {
                            "menu_key": "key.keyboard.b",
                            "entries": [
                                {"animation": "wave", "display_name": "Wave", "keybind": "key.keyboard.h", "face": "cheerful"}
                            ]
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let normalized = compat_format::normalize(&root).unwrap();
        // default_animations is an array of typed entries now.
        let list = normalized
            .get("default_animations")
            .and_then(|v| v.as_array())
            .expect("array default_animations");
        let attack = list
            .iter()
            .find(|e| e.get("type").and_then(crate::json::Value::as_str) == Some("attack"))
            .unwrap();
        assert_eq!(
            attack.get("name").and_then(crate::json::Value::as_str),
            Some("swing")
        );

        let mut bindings = ItemAnimationBindings::new();
        bindings.add(&normalized);
        assert_eq!(
            bindings.defaults.get("attack").map(String::as_str),
            Some("swing")
        );
        // idle_events keeps its variants list; the ingest reads name or clip.
        assert_eq!(bindings.variants.get("idleevents").map(Vec::len), Some(2));
        // The retention came through the typed entry.
        let retention = bindings.first_retention("", "", "useitem").unwrap();
        assert_eq!(
            retention.value("right_arm", false),
            0.5,
            "retention from the entry"
        );
        // Emote entries: menu key, id/display defaults, keybind fallback.
        assert_eq!(bindings.emote_menu_key.as_deref(), Some("key.keyboard.b"));
        assert_eq!(bindings.emote_bindings.len(), 1);
        let emote = &bindings.emote_bindings[0];
        assert_eq!(emote.id, "wave");
        assert_eq!(emote.display_name, "Wave");
        assert_eq!(emote.animation, "wave");
        assert_eq!(emote.key.as_deref(), Some("key.keyboard.h"));
        assert_eq!(emote.face.as_deref(), Some("cheerful"));
    }

    #[test]
    fn normalize_orders_the_java_top_level_keys() {
        let root = crate::json::parse(
            r#"{
                "format_version": "2",
                "defaults": {"static_when_using": {"attack": {"clip": "s"}}},
                "empty_hand": {"animations": {"attack": {"clip": "e"}}},
                "items": {"minecraft:stick": {"animations": {"attack": {"clip": "i"}}}},
                "item_groups": {"swords": {"animations": {"attack": {"clip": "g"}}}}
            }"#,
        )
        .unwrap();
        let out = compat_format::normalize(&root).unwrap();
        let keys: Vec<&str> = out
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(
            keys,
            vec![
                "static_when_using",
                "default_animations",
                "emptyHand",
                "item",
                "itemSpecific"
            ]
        );
        // The layers reach the bindings through the Java key names.
        let mut bindings = ItemAnimationBindings::new();
        bindings.add(&out);
        assert_eq!(
            bindings.empty_hand.get("attack").map(String::as_str),
            Some("e")
        );
        assert_eq!(
            bindings
                .groups
                .get("swords")
                .and_then(|m| m.get("attack"))
                .map(String::as_str),
            Some("g")
        );
        assert_eq!(
            bindings
                .specific_items
                .get("minecraft:stick")
                .and_then(|m| m.get("attack"))
                .map(String::as_str),
            Some("i")
        );
    }

    #[test]
    fn sequences_retrigger_influences_and_blends_ingest_from_entries() {
        // The normalized bindings document: typed entries under
        // default_animations, with the settings the compat format does not
        // carry.
        let normalized = crate::json::parse(
            r#"{
                "default_animations": [
                    {
                        "type": "sequence_attack",
                        "var_1": "s1",
                        "var_2": "s2",
                        "retrigger_progress": 0.4,
                        "locomotion_influence": {"walk": 0.5, "sprint": 150, "jump": 2},
                        "blending": {"from": 0.2, "to": 0.8},
                        "blending_progression": 0.3,
                        "timer_modifier": 0.5
                    }
                ]
            }"#,
        )
        .unwrap();
        let mut bindings = ItemAnimationBindings::new();
        bindings.add(&normalized);
        assert_eq!(
            bindings
                .default_sequences
                .get("sequenceattack")
                .map(|v| v.as_slice()),
            Some(&["s1".to_string(), "s2".to_string()][..])
        );
        assert_eq!(bindings.default_retrigger.get("sequenceattack"), Some(&0.4));
        let influence = bindings.default_influences.get("sequenceattack").unwrap();
        assert_eq!(influence.walk, 0.5);
        assert_eq!(influence.sprint, 1.0, "150 reads as 150% and clamps");
        assert_eq!(influence.jump, 0.02, "2 reads as 200%: 2/100");
        assert_eq!(influence.transition, 1.0);
        let blend = bindings.default_blends.get("sequenceattack").unwrap();
        assert_eq!(blend.from, 0.2);
        assert_eq!(blend.to, 0.8);
        assert_eq!(blend.duration_seconds, 0.3);
        assert!(blend.condition, "absent condition is true");
        let timer = bindings.default_timers.get("sequenceattack").unwrap();
        assert_eq!(timer.modifier, 0.5);
    }

    #[test]
    fn one_handed_index_walks_subanimations() {
        let root = crate::json::parse(
            r#"{
                "format_version": "2",
                "defaults": {
                    "animations": {
                        "attack": {
                            "clip": "swing",
                            "one_handed": "swing_one",
                            "subanimations": [
                                {"type": "use_item", "one_handed": {"clip": "drink_one"}}
                            ]
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let normalized = compat_format::normalize(&root).unwrap();
        let mut bindings = ItemAnimationBindings::new();
        bindings.add(&normalized);
        assert_eq!(
            bindings
                .default_one_handed
                .get("attack")
                .map(String::as_str),
            Some("swing_one")
        );
        assert_eq!(
            bindings
                .default_one_handed
                .get("useitem")
                .map(String::as_str),
            Some("drink_one")
        );
    }
}

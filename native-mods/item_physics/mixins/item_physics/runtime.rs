// Item Physics for Horizon (native mixin runtime): dropped items tumble while
// they fly, and once they land a flat item lies down on the ground and a block
// sits up, both turned to a yaw seeded by the item, resting flush on the
// surface instead of floating and spinning.
//
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game as `game::item_physics`. It is pure Rust: the
// game-side draw loop that applies these poses is in the bundle's `dropped.rs`.
use std::cell::RefCell;

/// The mod's id: its settings folder and its row in Options > Mods.
pub const ID: &str = "item_physics";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str =
    "Dropped items tumble in the air and lie flat on the ground. Horizon's own.";
/// The mod's setting file, beside the WebAssembly mods' (`enabled`).
pub const ENABLED: &str = "enabled";
/// How thin a model's depth must be for the item to count as flat: a block's
/// ground display is a quarter block deep, a tool's is a blade.
const THIN: f32 = 0.2;
/// The hair above the surface a resting item keeps, so its face does not
/// disappear into the block's top face: flush to the eye, but clear of it in
/// the depth buffer across the player's view distance.
const REST: f32 = 0.005;
/// How fast an airborne item turns, at the slowest and fastest, in radians a
/// second: a tumble, not vanilla's rigid spin.
const TUMBLE_SLOW: f32 = 1.75;
const TUMBLE_FAST: f32 = 4.25;

/// The mod's settings: on unless the player turned it off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub enabled: bool,
}

impl Settings {
    /// From the stored values (`get` reads one key's file); on by default.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let flag = |key: &str| get(key).is_none_or(|v| v.trim() != "false");
        Self {
            enabled: flag(ENABLED),
        }
    }
    /// The settings page's buttons.
    pub fn labels(&self) -> Vec<String> {
        vec![format!(
            "Enabled: {}",
            if self.enabled { "ON" } else { "OFF" }
        )]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        if i == 0 {
            self.enabled = !self.enabled;
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 1] {
        [(
            "enabled",
            if self.enabled { "true" } else { "false" }.to_owned(),
        )]
    }
}

#[derive(Default)]
struct State {
    settings: Option<Settings>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

/// Replaces the settings (the settings page changed them).
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

/// Whether the mod is on: the player's choice, read once.
pub fn enabled(get: impl Fn(&str) -> Option<String>) -> bool {
    settings(get).enabled
}

/// Whether a model this deep counts as flat: a tool's blade, not a block.
pub fn is_thin(depth: f32) -> bool {
    depth < THIN
}

/// One dropped item's pose: the three turns and the lift that keeps the model
/// out of the ground (resting flush) or centred on the item while it flies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemPose {
    /// About the model's x axis: what lays a flat item down.
    pub rotation_x: f32,
    /// About the world's y axis: the item's own yaw.
    pub rotation_y: f32,
    /// About the model's z axis: the tumble's second axis.
    pub rotation_z: f32,
    /// The height to raise the model by, in blocks, before it is turned.
    pub lift: f32,
}

/// The pose of dropped item `id` at `age` ticks: on the ground a flat item lies
/// down and a block sits up, both on a yaw seeded by the item and resting flush
/// on the surface (`lo..hi` is the model's bounding box); in the air it tumbles.
pub fn pose(
    id: u32,
    on_ground: bool,
    thin: bool,
    age: f32,
    lo: [f32; 3],
    hi: [f32; 3],
) -> ItemPose {
    let (rotation_x, rotation_y, rotation_z) = if on_ground {
        if thin {
            // Lying flat on its face, turned to the item's own yaw.
            (std::f32::consts::FRAC_PI_2, yaw(id), 0.0)
        } else {
            // Sitting up as a block does, turned to the item's own yaw.
            (0.0, yaw(id), 0.0)
        }
    } else {
        // Tumbling in the air: each item spins about its own axes at its own
        // rates, smoothly as `age` advances.
        let (x, y, z) = tumble(id);
        (x * age, y + yaw(id), z * age)
    };
    let (low, high) = turned_y_range(lo, hi, rotation_x, rotation_z);
    let lift = if on_ground {
        // Rest flush: the model's lowest point sits a hair above the surface.
        REST - low
    } else {
        // Centre the tumble on the item's position, so it spins in place.
        -(low + high) / 2.0
    };
    ItemPose {
        rotation_x,
        rotation_y,
        rotation_z,
        lift,
    }
}

/// The item's yaw on the ground, from its id: the same item always lands the
/// same way round, and different items land differently.
pub fn yaw(id: u32) -> f32 {
    hash01(id, 0) * std::f32::consts::TAU
}

/// The airborne tumble's rates about x, y and z, in radians a second, from the
/// item's id.
fn tumble(id: u32) -> (f32, f32, f32) {
    let spin = |salt| TUMBLE_SLOW + hash01(id, salt) * (TUMBLE_FAST - TUMBLE_SLOW);
    // Signs from the hash too, so items tumble over either shoulder.
    let sign = |salt| if hash01(id, salt) < 0.5 { -1.0 } else { 1.0 };
    (
        spin(1) * sign(4),
        spin(2) * 0.4 * sign(5),
        spin(3) * sign(6),
    )
}

/// The lowest and highest y the model's box reaches after the pose's turns,
/// relative to the item's origin: the box `lo..hi`'s corners turned by x, then
/// z. The yaw's turn is about y itself, so it never changes the height — which
/// is why a resting item sits flush at any yaw.
fn turned_y_range(lo: [f32; 3], hi: [f32; 3], rx: f32, rz: f32) -> (f32, f32) {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for x in [lo[0], hi[0]] {
        for y in [lo[1], hi[1]] {
            for z in [lo[2], hi[2]] {
                // About x: y and z turn.
                let (y, _) = turn(y, z, rx);
                // About z: x and y turn; y is the height from here on.
                let (_, y) = turn(x, y, rz);
                low = low.min(y);
                high = high.max(y);
            }
        }
    }
    (low, high)
}

/// One turn of `(a, b)` about the axis the other coordinate runs along.
fn turn(a: f32, b: f32, angle: f32) -> (f32, f32) {
    let (sin, cos) = angle.sin_cos();
    (a * cos - b * sin, a * sin + b * cos)
}

/// A deterministic 0..1 roll for an item's yaw and tumble.
fn hash01(id: u32, salt: u32) -> f32 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in [u64::from(id), u64::from(salt)] {
        hash = (hash ^ part).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    (hash >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tool's ground display: thin, its bottom below the origin.
    const BLADE: ([f32; 3], [f32; 3]) = ([-0.25, -0.4, -0.03], [0.25, 0.4, 0.03]);
    /// A block's ground display: a quarter-block cube centred on the origin.
    const CUBE: ([f32; 3], [f32; 3]) = ([-0.25; 3], [0.25; 3]);

    #[test]
    fn a_grounded_flat_item_lies_flat_flush_on_a_seeded_yaw() {
        let resting = pose(7, true, true, 0.0, BLADE.0, BLADE.1);
        assert_eq!(
            resting.rotation_x,
            std::f32::consts::FRAC_PI_2,
            "it lies down"
        );
        assert_eq!(resting.rotation_z, 0.0);
        assert_eq!(resting.rotation_y, yaw(7), "its yaw is its own");
        // Flush: turned, the model's lowest point is REST above the surface.
        let (low, _) = turned_y_range(BLADE.0, BLADE.1, resting.rotation_x, resting.rotation_z);
        assert!(
            (low + resting.lift - REST).abs() < 1e-6,
            "{low} + {}",
            resting.lift
        );
        assert!(resting.lift > 0.0, "it lifts the blade off the ground");
        // And it does not bob: the age does not move a resting item.
        assert_eq!(resting, pose(7, true, true, 600.0, BLADE.0, BLADE.1));
        // Flush at every yaw, since the yaw's turn cannot change the height.
        for id in 0..32u32 {
            let pose = pose(id, true, true, 0.0, BLADE.0, BLADE.1);
            let (low, _) = turned_y_range(BLADE.0, BLADE.1, pose.rotation_x, pose.rotation_z);
            assert!((low + pose.lift - REST).abs() < 1e-6, "{id}: {low}");
        }
    }

    #[test]
    fn a_grounded_block_sits_upright_flush_on_a_seeded_yaw() {
        let resting = pose(3, true, false, 0.0, CUBE.0, CUBE.1);
        assert_eq!(resting.rotation_x, 0.0, "it sits up");
        assert_eq!(resting.rotation_z, 0.0);
        assert_eq!(resting.rotation_y, yaw(3));
        // Flush: the cube's bottom (its lo.y) sits REST above the surface.
        assert!(
            (resting.lift - (REST - CUBE.0[1])).abs() < 1e-6,
            "{}",
            resting.lift
        );
        assert!((resting.lift - 0.255).abs() < 1e-3, "{}", resting.lift);
    }

    #[test]
    fn an_airborne_item_tumbles_smoothly_as_age_advances() {
        let at = |age| pose(11, false, false, age, CUBE.0, CUBE.1);
        let (rate_x, rate_y, rate_z) = tumble(11);
        assert!(rate_x.abs() > 0.0 && rate_z.abs() > 0.0, "it tumbles");
        // Constant angular velocity: equal steps, equal turns, no snapping.
        let (a, b, c) = (at(0.0), at(0.5), at(1.0));
        assert!((b.rotation_x - a.rotation_x - (c.rotation_x - b.rotation_x)).abs() < 1e-5);
        assert!((b.rotation_y - a.rotation_y - (c.rotation_y - b.rotation_y)).abs() < 1e-5);
        assert!((b.rotation_z - a.rotation_z - (c.rotation_z - b.rotation_z)).abs() < 1e-5);
        assert!((c.rotation_x - rate_x).abs() < 1e-4);
        assert!((c.rotation_y - (rate_y + yaw(11))).abs() < 1e-4);
        assert!((c.rotation_z - rate_z).abs() < 1e-4);
        // Centred on the item while it flies, whatever the turn.
        for age in [0.0, 0.37, 1.9, 30.0] {
            let flying = at(age);
            let (low, high) = turned_y_range(CUBE.0, CUBE.1, flying.rotation_x, flying.rotation_z);
            assert!((low + high + 2.0 * flying.lift).abs() < 1e-5, "{age}");
        }
    }

    #[test]
    fn the_yaw_is_deterministic_and_differs_between_items() {
        assert_eq!(yaw(7), yaw(7));
        assert_eq!(
            pose(7, true, true, 0.0, BLADE.0, BLADE.1),
            pose(7, true, true, 0.0, BLADE.0, BLADE.1)
        );
        // Most items land differently; three is enough to be sure here.
        let yaw = (yaw(1), yaw(2), yaw(3));
        assert!(yaw.0 != yaw.1 && yaw.1 != yaw.2, "{yaw:?}");
    }

    #[test]
    fn thin_follows_the_models_depth() {
        assert!(is_thin(0.06));
        assert!(is_thin(0.19));
        assert!(!is_thin(0.2));
        assert!(!is_thin(0.25), "a block's ground display is not flat");
    }

    #[test]
    fn the_mod_is_on_until_turned_off_and_stays_off() {
        fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
            |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
        }
        assert!(Settings::read(stored(&[])).enabled, "on by default");
        assert!(!Settings::read(stored(&[("enabled", "false")])).enabled);
        let mut s = Settings::read(stored(&[]));
        s.click(0);
        assert_eq!(s.labels(), ["Enabled: OFF"]);
        let off: Vec<(&str, String)> = s.stored().into();
        assert_eq!(off, [("enabled", "false".to_owned())]);
        let back: Vec<(&str, &str)> = off.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert!(!Settings::read(stored(&back)).enabled);
        s.click(0);
        assert_eq!(s.labels(), ["Enabled: ON"]);
        // Another button does nothing.
        let mut s = Settings::read(stored(&[]));
        s.click(1);
        assert!(s.enabled);
    }
}

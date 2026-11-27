// Timber Fall for Horizon (native mixin runtime): a log cut with an axe takes the whole
// tree down. The tree is cleared in the cut's tick, tips around its stump hinge with the
// pole equation, stops on solid ground, and its blocks drop at the stump a second later.
//
// Everything here is self-contained: `horizon-mixins` copies this file into
// `crates/horizon-client/src/game.rs`, so it may only use what the game crate itself has.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use super::Game;
use crate::items::{BlockLootRoll, ItemStack};
use crate::mod_api::NativeMod;
use horizon_client::mesher::Vertex;
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;
use winit::keyboard::KeyCode;

pub const ID: &str = "timber_fall";
pub const NAME: &str = "Timber Fall";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str =
    "Chop a tree's base and the whole tree tips over, then drops its logs where it lands.";

/// The most logs one tree may have.
const MAX_LOGS: usize = 256;
/// The deepest a leaf may sit from a log and still belong to the tree.
const MAX_LEAVES_RADIUS: u32 = 10;
/// The whole animation lasts this long, in seconds.
const DURATION: f64 = 3.0;
/// Timeline fractions from `voxel-tree-fall.html`.
const IMPACT: f64 = 0.50;
const BREAK: f64 = 0.66;
const LAST: f64 = 0.80;
/// Free-fall gravity for logs and leaves, in the reference animation.
const GRAVITY_FALL: f64 = 14.0;
/// How far each of the three contacts kicks back.
const BOUNCE_DEG: f64 = 5.0;
/// One server tick, in seconds.
const TICK_SECS: f64 = 0.05;
/// Three parabolic hops inside the bounce segment: (start, length, amplitude).
const HOPS: [[f64; 3]; 3] = [[0.0, 0.54, 1.0], [0.54, 0.30, 0.3], [0.84, 0.16, 0.09]];

/// `#minecraft:logs` (and `#c:logs`) by name.
fn is_log(block: &str) -> bool {
    block != "minecraft:mushroom_stem"
        && ["_log", "_wood", "_stem", "_hyphae"]
            .iter()
            .any(|s| block.ends_with(s))
}

/// `#minecraft:leaves` by name.
fn is_leaves(block: &str) -> bool {
    block.ends_with("_leaves")
}

/// `#minecraft:axes` by name.
fn is_axe(item: &str) -> bool {
    item.ends_with("_axe") && !item.ends_with("_pickaxe")
}

/// The durability of a known axe, for the extra wear.
fn axe_durability(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:wooden_axe" => 59,
        "minecraft:golden_axe" => 32,
        "minecraft:stone_axe" => 131,
        "minecraft:copper_axe" => 190,
        "minecraft:iron_axe" => 250,
        "minecraft:diamond_axe" => 1561,
        "minecraft:netherite_axe" => 2031,
        _ => return None,
    })
}

/// What the finder needs to know about a block.
#[derive(Debug, Clone, PartialEq, Default)]
struct Block {
    name: String,
    /// The leaves' `distance`, if the block has one.
    distance: Option<u32>,
    persistent: bool,
}

/// A tree to take down, apart from the chopped log.
#[derive(Debug, Clone, Default, PartialEq)]
struct Tree {
    logs: Vec<[i32; 3]>,
    leaves: Vec<[i32; 3]>,
    /// Vines, cocoa and bee nests.
    attached: Vec<[i32; 3]>,
}

const SIDES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

fn add(a: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The tree whose log at `start` is chopped, or None if it isn't one (no leaves, too big).
fn find_tree(start: [i32; 3], read: &dyn Fn([i32; 3]) -> Block) -> Option<Tree> {
    // gatherLogs: breadth-first through dx, dz in -1..=1, dy in 0..=1.
    let mut logs = Vec::new();
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(pos) = queue.pop_front() {
        if !is_log(&read(pos).name) {
            continue;
        }
        logs.push(pos);
        if logs.len() > MAX_LOGS {
            return None;
        }
        for dy in 0..=1 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let next = add(pos, [dx, dy, dz]);
                    if seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
        }
    }
    if logs.is_empty() {
        return None;
    }
    // gatherLeavesAroundLog, for each log; a leaf counts where its distance is its depth.
    let mut leaves = HashSet::new();
    for &log in &logs {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<([i32; 3], u32)> =
            SIDES.iter().map(|&d| (add(log, d), 1)).collect();
        while let Some((pos, depth)) = queue.pop_front() {
            if depth > MAX_LEAVES_RADIUS || visited.contains(&pos) {
                continue;
            }
            let b = read(pos);
            if b.distance.unwrap_or(0) != depth {
                continue;
            }
            visited.insert(pos);
            if is_leaves(&b.name) && !b.persistent {
                leaves.insert(pos);
                for d in SIDES {
                    let next = add(pos, d);
                    if !visited.contains(&next) {
                        queue.push_back((next, depth + 1));
                    }
                }
            }
        }
    }
    if leaves.is_empty() {
        return None;
    }
    // gatherAdjacentBlocks: vines (and the vines hanging below them), cocoa, bee nests.
    let mut attached = HashSet::new();
    for &pos in logs.iter().chain(leaves.iter()) {
        for d in SIDES {
            let mut next = add(pos, d);
            match read(next).name.as_str() {
                "minecraft:vine" => {
                    while read(next).name == "minecraft:vine" && attached.insert(next) {
                        next = add(next, [0, -1, 0]);
                    }
                }
                "minecraft:cocoa" | "minecraft:bee_nest" => {
                    attached.insert(next);
                }
                _ => {}
            }
        }
    }
    let mut leaves: Vec<_> = leaves.into_iter().collect();
    let mut attached: Vec<_> = attached.into_iter().collect();
    leaves.sort();
    attached.sort();
    logs.retain(|&p| p != start);
    Some(Tree {
        logs,
        leaves,
        attached,
    })
}

/// The mod's settings in Options > Mods > Timber Fall.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Settings {
    enabled: bool,
    break_leaves: bool,
    wear_axe: bool,
}

impl Settings {
    fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let flag = |key: &str, default: bool| {
            get(key).map_or(default, |v| {
                let trimmed = v.trim();
                trimmed != "false" && trimmed != "off"
            })
        };
        Self {
            enabled: flag("enabled", true),
            break_leaves: flag("break_leaves", true),
            wear_axe: flag("wear_axe", true),
        }
    }

    fn labels(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("{}: {}", NAME, on_off(self.enabled)),
            format!("Break Leaves: {}", on_off(self.break_leaves)),
            format!("Wear Axe: {}", on_off(self.wear_axe)),
        ]
    }

    fn click(&mut self, button: usize) {
        match button {
            0 => self.enabled = !self.enabled,
            1 => self.break_leaves = !self.break_leaves,
            2 => self.wear_axe = !self.wear_axe,
            _ => {}
        }
    }

    fn stored(&self) -> [(&'static str, String); 3] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("enabled", flag(self.enabled)),
            ("break_leaves", flag(self.break_leaves)),
            ("wear_axe", flag(self.wear_axe)),
        ]
    }
}

/// One log of the fallen tree, with the drops spawned when it pops.
struct LogBlock {
    pos: BlockPos,
    state: BlockStateId,
    drops: Vec<ItemStack>,
    dropped: bool,
}

/// One leaf (or attached vine/cocoa/nest), with its detach schedule and free fall.
struct LeafBlock {
    center: [f64; 3],
    state: BlockStateId,
    /// Timeline fraction where it breaks off the tree.
    at: f64,
    /// World centre at the moment it breaks off.
    from: [f64; 3],
    /// The trunk angle at that moment.
    ang: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    spin: f64,
    size: f64,
}

/// One tree in flight.
struct ActiveTree {
    dimension: String,
    player: String,
    /// The cut log; the stump's top centre is the hinge.
    cut: [i32; 3],
    toward: [f64; 3],
    elapsed_secs: f64,
    /// Wall-clock seconds used for rendering, so frames are not tied to 20 TPS.
    smooth_secs: f64,
    /// The last server tick; a gap over 0.25 s freezes the animation.
    last_tick_at: Instant,
    last_render_at: Option<Instant>,
    /// The angle the trunk stops at (lowered by terrain hit during the fall).
    impact_angle: f64,
    logs: Vec<LogBlock>,
    leaves: Vec<LeafBlock>,
    axe: Option<String>,
}

struct State {
    settings: Option<Settings>,
    active: Vec<ActiveTree>,
}

static STATE: Mutex<State> = Mutex::new(State {
    settings: None,
    active: Vec::new(),
});

fn lock_state() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

impl Game<'_> {
    pub(super) fn timber_fall_dimension_id(&self) -> &'static str {
        match self.dim {
            super::Dim::Nether => "minecraft:the_nether",
            super::Dim::End => "minecraft:the_end",
            _ => "minecraft:overworld",
        }
    }

    fn timber_fall_settings_state(&self) -> Settings {
        let mut s = lock_state();
        *s.settings
            .get_or_insert_with(|| Settings::read(|key| self.native_setting(ID, key)))
    }

    pub(super) fn timber_fall_mod_entry() -> NativeMod {
        NativeMod {
            id: ID.into(),
            title: format!("{NAME} {VERSION}"),
            description: DESCRIPTION.into(),
            keys: Vec::new(),
        }
    }

    pub(super) fn timber_fall_settings(&self) -> Vec<String> {
        self.timber_fall_settings_state().labels()
    }

    pub(super) fn timber_fall_settings_click(&mut self, button: usize) {
        let mut settings = self.timber_fall_settings_state();
        settings.click(button);
        {
            let mut s = lock_state();
            s.settings = Some(settings);
        }
        for (key, value) in settings.stored() {
            self.native_set_setting(ID, key, &value);
        }
    }

    fn timber_fall_read_block(&self, pos: [i32; 3]) -> Block {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let facts = self.res.facts().get(state);
        let name = facts.name.to_string();
        let (distance, persistent) = if let Some(state_data) = self.pack.assets.registry.state(state)
        {
            let distance = state_data
                .properties
                .get("distance")
                .and_then(|d| d.parse::<u32>().ok());
            let persistent = state_data
                .properties
                .get("persistent")
                .is_some_and(|p| p == "true");
            (distance, persistent)
        } else {
            (None, false)
        };
        Block {
            name,
            distance,
            persistent,
        }
    }

    fn timber_fall_sneaking(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.shift_key_down)
            || self.keys.contains(&KeyCode::ShiftLeft)
            || self.keys.contains(&KeyCode::ShiftRight)
            || self.touch_sneaking()
    }

    /// A log was cut: clear the rest of the tree this tick and start its fall.
    ///
    /// The cut log itself is left to vanilla, so the game removes and drops it normally
    /// and the tree cannot be left behind by a canceled break.
    pub(super) fn timber_fall_destroy_block(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        _canceled: &mut bool,
    ) {
        if self.remote.is_some() {
            return;
        }
        let settings = self.timber_fall_settings_state();
        if !settings.enabled || self.timber_fall_sneaking() || self.spectator() {
            return;
        }
        let Some(held) = self.held() else {
            return;
        };
        let held_item = held.item.to_string();
        if !is_axe(&held_item) {
            return;
        }
        let facts = self.res.facts().get(state);
        if !is_log(&facts.name) {
            return;
        }
        let read = |p: [i32; 3]| self.timber_fall_read_block(p);
        let Some(mut tree) = find_tree(pos, &read) else {
            return;
        };
        if !settings.break_leaves {
            tree.leaves.clear();
        }

        let (px, pz, yaw) = self
            .player
            .as_ref()
            .map_or((pos[0] as f64 + 0.5, pos[2] as f64 + 0.5, 0.0), |p| {
                (p.position.x, p.position.z, p.yaw)
            });
        let dx = (pos[0] as f64 + 0.5) - px;
        let dz = (pos[2] as f64 + 0.5) - pz;
        let len = (dx * dx + dz * dz).sqrt();
        let toward = if len > 0.001 {
            [dx / len, 0.0, dz / len]
        } else {
            let rad = (yaw as f64).to_radians();
            [-rad.sin(), 0.0, rad.cos()]
        };

        // Base to tip: the lowest log pops first.
        let mut log_positions = tree.logs.clone();
        log_positions.sort_by_key(|p| {
            (p[1], (p[0] - pos[0]).abs() + (p[2] - pos[2]).abs())
        });

        let held_stack = self.held().cloned();
        let mut logs = Vec::new();
        for p in &log_positions {
            let block = self.world.block(p[0], p[1], p[2]);
            let drops = self.res.items().block_drops(
                self.pack.assets.registry,
                block,
                BlockLootRoll {
                    held: held_stack.as_ref(),
                    ..Default::default()
                },
                &mut self.world.random,
                None,
            );
            logs.push(LogBlock {
                pos: BlockPos::new(p[0], p[1], p[2]),
                state: block,
                drops,
                dropped: false,
            });
        }

        let mut leaves = Vec::new();
        for p in tree.attached.iter().chain(&tree.leaves) {
            let block = self.world.block(p[0], p[1], p[2]);
            let center = [
                p[0] as f64 + 0.5,
                p[1] as f64 + 0.5,
                p[2] as f64 + 0.5,
            ];
            let (at, from, ang) = schedule_leaf(pos, toward, center);
            leaves.push(LeafBlock {
                center,
                state: block,
                at,
                from,
                ang,
                vx: (hash01(*p, 10) - 0.5) * 5.0,
                vy: hash01(*p, 11) * 3.0,
                vz: (hash01(*p, 12) - 0.5) * 5.0,
                spin: (hash01(*p, 13) - 0.5) * 8.0,
                size: 0.3 + hash01(*p, 14) * 0.25,
            });
        }

        // Clear everything except the cut log, which vanilla breaks normally.
        for log in &logs {
            let left = self.fluid_left(log.state);
            let at = log.pos;
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, left, 3));
        }
        for leaf in &leaves {
            let left = self.fluid_left(leaf.state);
            let at = BlockPos::new(
                leaf.center[0].floor() as i32,
                leaf.center[1].floor() as i32,
                leaf.center[2].floor() as i32,
            );
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, left, 3));
        }

        let axe = (settings.wear_axe && !self.creative).then_some(held_item);
        let mut s = lock_state();
        if !s.active.iter().any(|t| t.cut == pos) {
            s.active.push(ActiveTree {
                dimension: self.timber_fall_dimension_id().to_string(),
                player: self.acting_player_name().to_string(),
                cut: pos,
                toward,
                elapsed_secs: 0.0,
                smooth_secs: 0.0,
                last_tick_at: Instant::now(),
                last_render_at: None,
                impact_angle: std::f64::consts::FRAC_PI_2,
                logs,
                leaves,
                axe,
            });
        }
    }

    /// Advances every active tree through the `voxel-tree-fall.html` timeline.
    pub(super) fn timber_fall_tick(&mut self) {
        if self.remote.is_some() {
            return;
        }
        let mut spawns: Vec<(String, Vec<ItemStack>, [f64; 3])> = Vec::new();
        let mut landed = Vec::new();
        {
            let mut s = lock_state();
            let mut i = 0;
            while i < s.active.len() {
                let finished = {
                    let tree = &mut s.active[i];
                    tree.elapsed_secs += TICK_SECS;
                    tree.last_tick_at = Instant::now();
                    let p = (tree.elapsed_secs / DURATION).clamp(0.0, 1.0);

                    // Solid ground can stop the fall before the whole 90 degrees.
                    if p < IMPACT {
                        let angle = angle_at(p);
                        if angle < tree.impact_angle && self.timber_fall_hits(tree, angle) {
                            tree.impact_angle = angle;
                        }
                    }

                    let log_count = tree.logs.len();
                    for index in 0..log_count {
                        if tree.logs[index].dropped {
                            continue;
                        }
                        let pop = pop_at(index, log_count);
                        if p < pop {
                            continue;
                        }
                        let angle = tree_angle(tree, pop);
                        let home = block_center(tree.logs[index].pos);
                        let landing = rotate_point(tree.cut, tree.toward, home, angle);
                        tree.logs[index].dropped = true;
                        let drops = std::mem::take(&mut tree.logs[index].drops);
                        spawns.push((tree.dimension.clone(), drops, landing));
                    }

                    // Leaves only shrink and vanish; they never become items.

                    if p >= 1.0 {
                        // Safety net: nothing may be lost if a leaf ran past the end.
                        for log in tree.logs.iter_mut() {
                            if log.dropped {
                                continue;
                            }
                            log.dropped = true;
                            let home = block_center(log.pos);
                            let landing =
                                rotate_point(tree.cut, tree.toward, home, tree.impact_angle);
                            spawns.push((
                                tree.dimension.clone(),
                                std::mem::take(&mut log.drops),
                                landing,
                            ));
                        }
                        true
                    } else {
                        false
                    }
                };
                if finished {
                    landed.push(s.active.remove(i));
                } else {
                    i += 1;
                }
            }
        }
        for (dimension, drops, at) in spawns {
            self.timber_fall_spawn(&dimension, drops, at);
        }
        for tree in landed {
            self.timber_fall_wear(tree);
        }
    }

    /// Spawns a block's drops at `at`, switching levels when the tree is elsewhere.
    fn timber_fall_spawn(&mut self, dimension: &str, drops: Vec<ItemStack>, at: [f64; 3]) {
        if drops.is_empty() || !self.ui.rules.bool("block_drops") {
            return;
        }
        let back = self.dim;
        let target = super::Dim::from_id(dimension);
        let switched = if back != target {
            self.enter_level(target).is_ok()
        } else {
            false
        };
        for stack in drops {
            if &*stack.item != "minecraft:air" {
                self.spawn_item_own(stack, at, 0);
            }
        }
        if switched {
            let _ = self.enter_level(back);
        }
    }

    /// The tree is done: wear the axe for the logs it brought down.
    fn timber_fall_wear(&mut self, tree: ActiveTree) {
        let logs = tree.logs.len();
        if let Some(axe) = tree.axe
            && let Some(line) = timber_fall_wear_line(&axe, &tree.player, logs)
        {
            self.run_command_quietly(&line);
        }
    }

    /// A little camera shake for 0.35 s after the trunk hits the ground.
    pub(super) fn timber_fall_camera_shake(&self, _partial: f32, shake: &mut mod_api::Shake) {
        let s = lock_state();
        let dimension = self.timber_fall_dimension_id();
        for tree in &s.active {
            if tree.dimension != dimension {
                continue;
            }
            let p = (tree.smooth_secs / DURATION).clamp(0.0, 1.0);
            let t = (p - IMPACT) * DURATION;
            if !(0.0..0.35).contains(&t) {
                continue;
            }
            let env = (1.0 - t / 0.35).powi(2);
            shake.offset[1] += ((t * 75.0).sin() * 0.05 * env) as f32;
            shake.offset[0] += ((t * 53.0).sin() * 0.025 * env) as f32;
            shake.roll += ((t * 60.0).sin() * 0.45 * env) as f32;
        }
    }

    /// Whether a trunk block would be inside solid ground at `angle`.
    fn timber_fall_hits(&self, tree: &ActiveTree, angle: f64) -> bool {
        let (sin, cos) = angle.sin_cos();
        let t = 1.0 - cos;
        let (dx, dz) = (tree.toward[0], tree.toward[2]);
        let hinge = [
            tree.cut[0] as f64 + 0.5,
            tree.cut[1] as f64,
            tree.cut[2] as f64 + 0.5,
        ];
        for log in &tree.logs {
            let ox = log.pos.x as f64 + 0.5 - hinge[0];
            let oy = log.pos.y as f64 + 0.5 - hinge[1];
            let oz = log.pos.z as f64 + 0.5 - hinge[2];
            let x = (cos + t * dz * dz) * ox + sin * dx * oy - t * dx * dz * oz;
            let y = -sin * dx * ox + cos * oy - sin * dz * oz;
            let z = -t * dx * dz * ox + sin * dz * oy + (cos + t * dx * dx) * oz;
            let at = [
                (hinge[0] + x).floor() as i32,
                (hinge[1] + y).floor() as i32,
                (hinge[2] + z).floor() as i32,
            ];
            if !self.native_block_collision(at).is_empty() {
                return true;
            }
        }
        false
    }

    /// Draws every active tree in the current dimension as rotating block models.
    pub(super) fn timber_fall_render(
        &mut self,
        camera: glam::DVec3,
        _partial: f64,
        out: &mut Vec<Vertex>,
    ) {
        let mut s = lock_state();
        if s.active.is_empty() {
            return;
        }
        let dimension = self.timber_fall_dimension_id();
        for tree in &mut s.active {
            if tree.dimension != dimension {
                continue;
            }
            // Wall-clock seconds keep the animation smooth between 20 TPS server ticks.
            let now = Instant::now();
            if tree.last_tick_at.elapsed().as_secs_f64() < 0.25 {
                if let Some(last) = tree.last_render_at {
                    let frame = now.duration_since(last).as_secs_f64().clamp(0.0, 0.05);
                    tree.smooth_secs = (tree.smooth_secs + frame).min(DURATION);
                }
            }
            tree.last_render_at = Some(now);
            let p = (tree.smooth_secs / DURATION).clamp(0.0, 1.0);
            let angle = tree_angle(tree, p) as f32;
            let axis = glam::Vec3::new(tree.toward[2] as f32, 0.0, -(tree.toward[0] as f32));
            let hinge = glam::DVec3::new(
                tree.cut[0] as f64 + 0.5,
                tree.cut[1] as f64,
                tree.cut[2] as f64 + 0.5,
            );
            let to_camera = (hinge - camera).as_vec3();
            let rot = glam::Mat4::from_axis_angle(axis, angle);
            self.mark_entity(out.len(), "minecraft:falling_block");

            let log_count = tree.logs.len();
            for (index, log) in tree.logs.iter().enumerate() {
                let pop = pop_at(index, log_count);
                if p >= pop {
                    continue;
                }
                // Swell and brighten in the last 0.16 s before this log pops.
                let w = (1.0 - (pop - p) * DURATION / 0.16).max(0.0);
                let scale = (1.0 + 0.35 * w * w) as f32;
                let light = self.world.light(log.pos.x, log.pos.y, log.pos.z);
                let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
                let offset = glam::Vec3::new(
                    log.pos.x as f32 - hinge.x as f32,
                    log.pos.y as f32 - hinge.y as f32,
                    log.pos.z as f32 - hinge.z as f32,
                );
                let matrix = glam::Mat4::from_translation(to_camera)
                    * rot
                    * glam::Mat4::from_translation(offset)
                    * glam::Mat4::from_scale(glam::Vec3::splat(scale));
                let v_start = out.len();
                self.block_model_vertices(log.state, 42, matrix, packed, true, out);
                if w > 0.0 {
                    // White-hot pop flash in the last 0.16 s before this log bursts.
                    let k = (0.75 * w * w * w).min(1.0) as f32;
                    let mix = |c: u32| (c as f32 + (255.0 - c as f32) * k) as u32;
                    for v in &mut out[v_start..] {
                        let alpha = v.color >> 24;
                        let r = mix(v.color & 0xFF);
                        let g = mix((v.color >> 8) & 0xFF);
                        let b = mix((v.color >> 16) & 0xFF);
                        v.color = (alpha << 24) | (b << 16) | (g << 8) | r;
                        v.light = (v.light as f32 + (0xF0F0 as f32 - v.light as f32) * k) as u32;
                    }
                }
            }

            let ground = tree.cut[1] as f64;
            for leaf in &tree.leaves {
                let (center, scale, spin) = if p <= leaf.at {
                    (
                        rotate_point(tree.cut, tree.toward, leaf.center, angle as f64),
                        1.0,
                        None,
                    )
                } else {
                    let (center, scale, spin) = leaf_state(leaf, p, ground);
                    (center, scale, Some(spin))
                };
                if scale <= 0.0 {
                    continue;
                }
                let corner = glam::Vec3::new(
                    (center[0] - 0.5) as f32 - camera.x as f32,
                    (center[1] - 0.5) as f32 - camera.y as f32,
                    (center[2] - 0.5) as f32 - camera.z as f32,
                );
                let base = if p <= leaf.at {
                    rot
                } else {
                    glam::Mat4::from_axis_angle(axis, leaf.ang as f32)
                        * glam::Mat4::from_rotation_y(spin.unwrap_or(0.0) as f32)
                };
                let matrix = glam::Mat4::from_translation(corner)
                    * base
                    * glam::Mat4::from_scale(glam::Vec3::splat(scale as f32));
                let light = self.world.light(
                    center[0].floor() as i32,
                    center[1].floor() as i32,
                    center[2].floor() as i32,
                );
                let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
                self.block_model_vertices(leaf.state, 42, matrix, packed, true, out);
            }
        }
    }
}

/// The inverted-pendulum shape from `voxel-tree-fall.html`, normalized 0..=1.
fn shape_curve() -> &'static [f64] {
    static SHAPE: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    SHAPE.get_or_init(|| {
        let mut angles = vec![0.0];
        let mut th = 0.08;
        let mut w = 0.0;
        while th < std::f64::consts::FRAC_PI_2 {
            w += th.sin() * 0.002;
            th += w * 0.002;
            angles.push(th);
        }
        let last = angles
            .last()
            .copied()
            .unwrap_or(std::f64::consts::FRAC_PI_2);
        angles
            .into_iter()
            .map(|v| (v - 0.08) / (last - 0.08))
            .collect()
    })
}

/// The reference angle at timeline fraction `p`: fall shape, then three hops.
fn angle_at(p: f64) -> f64 {
    if p < IMPACT {
        let shape = shape_curve();
        let u = (p / IMPACT) * (shape.len() - 1) as f64;
        let i = u.floor() as usize;
        let a = shape.get(i).copied().unwrap_or(1.0);
        let b = shape.get(i + 1).copied().unwrap_or(1.0);
        (a + (b - a) * (u - i as f64)).max(0.0) * std::f64::consts::FRAC_PI_2
    } else {
        let mut q = (p - IMPACT) / (BREAK - IMPACT);
        if q >= 1.0 {
            return std::f64::consts::FRAC_PI_2;
        }
        q = q.max(0.0);
        for [start, len, amp] in HOPS {
            if q < start + len {
                let u = (q - start) / len;
                return std::f64::consts::FRAC_PI_2
                    - 4.0 * u * (1.0 - u) * amp * BOUNCE_DEG.to_radians();
            }
        }
        std::f64::consts::FRAC_PI_2
    }
}

/// The trunk angle at `p`, respecting a terrain hit and the bounce around it.
fn tree_angle(tree: &ActiveTree, p: f64) -> f64 {
    if p < IMPACT {
        angle_at(p).min(tree.impact_angle)
    } else {
        let offset = std::f64::consts::FRAC_PI_2 - angle_at(p);
        (tree.impact_angle - offset).max(0.0)
    }
}

/// Timeline fraction where log `index` pops, base to tip.
fn pop_at(index: usize, count: usize) -> f64 {
    let steps = count.saturating_sub(1).max(1) as f64;
    BREAK + (LAST - BREAK) * index as f64 / steps
}

fn block_center(pos: BlockPos) -> [f64; 3] {
    [
        pos.x as f64 + 0.5,
        pos.y as f64 + 0.5,
        pos.z as f64 + 0.5,
    ]
}

/// Rotates a world-space centre around the stump hinge by `angle`.
fn rotate_point(cut: [i32; 3], toward: [f64; 3], home: [f64; 3], angle: f64) -> [f64; 3] {
    let hinge = [
        cut[0] as f64 + 0.5,
        cut[1] as f64,
        cut[2] as f64 + 0.5,
    ];
    let (sin, cos) = angle.sin_cos();
    let t = 1.0 - cos;
    let (dx, dz) = (toward[0], toward[2]);
    let ox = home[0] - hinge[0];
    let oy = home[1] - hinge[1];
    let oz = home[2] - hinge[2];
    [
        hinge[0] + (cos + t * dz * dz) * ox + sin * dx * oy - t * dx * dz * oz,
        hinge[1] + (-sin * dx * ox + cos * oy - sin * dz * oz),
        hinge[2] + (-t * dx * dz * ox + sin * dz * oy + (cos + t * dx * dx) * oz),
    ]
}

/// The earliest fraction where a leaf touches the ground, and its state there.
fn schedule_leaf(cut: [i32; 3], toward: [f64; 3], center: [f64; 3]) -> (f64, [f64; 3], f64) {
    let ground = cut[1] as f64;
    let mut p = 0.0;
    loop {
        let ang = angle_at(p);
        let at = rotate_point(cut, toward, center, ang);
        if p >= IMPACT || at[1] <= ground + 0.55 {
            return (p.min(IMPACT), at, ang);
        }
        p += 0.004;
    }
}

/// A detached leaf's world centre, scale and spin at timeline fraction `p`.
fn leaf_state(leaf: &LeafBlock, p: f64, ground: f64) -> ([f64; 3], f64, f64) {
    let t = (p - leaf.at) * DURATION;
    let g = GRAVITY_FALL;
    let size = leaf.size;
    let vy = leaf.vy.max(0.0);
    let h = (leaf.from[1] - size / 2.0 - ground).max(0.0);
    let land = (vy + (vy * vy + 2.0 * g * h).sqrt()) / g;
    let a = t.min(land);
    let x = leaf.from[0] + leaf.vx * a * 0.6;
    let z = leaf.from[2] + leaf.vz * a * 0.6;
    let y = if t < land {
        leaf.from[1] + vy * a - 0.5 * g * a * a
    } else {
        ground + size / 2.0
    };
    let gone = ((t - land - 0.2 - (size - 0.3) * 2.4) / 0.25).clamp(0.0, 1.0);
    let grow = 1.0 + (size - 1.0) * (t * 5.0).min(1.0);
    let scale = (grow * (1.0 - gone)).max(0.0);
    ([x, y, z], scale, leaf.spin * a)
}

/// A deterministic 0..1 roll for a leaf's velocity, size and spin.
fn hash01(pos: [i32; 3], salt: i64) -> f64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in pos {
        hash = (hash ^ part as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash = (hash ^ salt as u64).wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    (hash >> 40) as f64 / 16_777_216.0
}

/// The axe's wear for `logs` felled logs, as a command.
fn timber_fall_wear_line(item: &str, player: &str, logs: usize) -> Option<String> {
    let max = axe_durability(item)?;
    if logs == 0 {
        return None;
    }
    Some(format!(
        "execute as {player} run item modify entity @s weapon.mainhand {{\"type\":\"minecraft:set_damage\",\"damage\":{:.6},\"add\":true}}",
        -(logs as f64 + 0.5) / f64::from(max)
    ))
}
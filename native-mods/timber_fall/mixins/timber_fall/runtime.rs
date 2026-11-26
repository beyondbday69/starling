//! Timber Fall for Horizon (native mixin runtime): a log cut with an axe takes the whole
//! tree down. The tree is cleared in the cut's tick, tips around its stump hinge with the
//! pole equation, stops on solid ground, and its blocks drop at the stump a second later.
//!
//! Everything here is self-contained: `horizon-mixins` copies this file into
//! `crates/horizon-client/src/game.rs`, so it may only use what the game crate itself has.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use super::Game;
use crate::items::{BlockLootRoll, ItemStack};
use crate::mod_api::{
    Identifier, LightCoords, ModelInstance, ModelRef, NativeMod, Overlay,
};
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
/// Gravity, `g` in the pole equation.
const GRAVITY: f64 = 9.8;
/// The lean warning before the tree moves.
const WARNING_SECS: f64 = 0.5;
/// Time from the impact to the blocks falling apart.
const BREAKUP_SECS: f64 = 1.0;
/// The angle the fall starts at, after the warning.
const START_ANGLE: f64 = 3.0 * std::f64::consts::PI / 180.0;
/// The fall speed the concept ships with.
const FALL_SPEED: f64 = 1.5;

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

/// Where a falling tree is.
#[derive(Clone, Copy, Debug)]
enum Phase {
    Warning { since: f64 },
    Falling { angle: f64, vel: f64 },
    Impact { since: f64, angle: f64 },
}

/// One tree in flight.
struct ActiveTree {
    dimension: String,
    player: String,
    /// The cut log; the stump's top centre is the hinge.
    cut: [i32; 3],
    toward: [f64; 3],
    length: f64,
    blocks: Vec<(BlockPos, BlockStateId, bool)>,
    logs: usize,
    drops: Vec<ItemStack>,
    axe: Option<String>,
    phase: Phase,
}

struct State {
    settings: Option<Settings>,
    active: Vec<ActiveTree>,
}

static STATE: Mutex<State> = Mutex::new(State {
    settings: None,
    active: Vec::new(),
});

fn state() -> std::sync::MutexGuard<'static, State> {
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
        let mut s = state();
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
            let mut s = state();
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

    /// A log was cut: clear the whole tree this tick and start its fall.
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

        let mut max_y = pos[1];
        for &p in tree.logs.iter().chain(&tree.leaves).chain(&tree.attached) {
            if p[1] > max_y {
                max_y = p[1];
            }
        }
        let height = (max_y - pos[1] + 1) as f64;

        let mut blocks = Vec::new();
        let mut drops = Vec::new();
        let held_stack = self.held().cloned();
        let positions: Vec<[i32; 3]> = tree
            .attached
            .iter()
            .chain(&tree.leaves)
            .chain(&tree.logs)
            .copied()
            .collect();
        for p in &positions {
            let s = self.world.block(p[0], p[1], p[2]);
            let is_log_block = is_log(&self.res.facts().get(s).name);
            blocks.push((BlockPos::new(p[0], p[1], p[2]), s, is_log_block));
            let d = self.res.items().block_drops(
                self.pack.assets.registry,
                s,
                BlockLootRoll {
                    held: held_stack.as_ref(),
                    ..Default::default()
                },
                &mut self.world.random,
                None,
            );
            drops.extend(d);
        }

        for (at, block, _) in &blocks {
            let left = self.fluid_left(*block);
            let at = *at;
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, left, 3));
        }

        self.audio.play_unshared(
            "minecraft:block.wood.break",
            Some([pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5]),
            1.0,
            0.7,
        );

        let now = self.world.game_time as f64 + self.clock.fraction();
        let axe = (settings.wear_axe && !self.creative).then_some(held_item);
        let logs = tree.logs.len() + 1;
        let mut s = state();
        if !s.active.iter().any(|t| t.cut == pos) {
            s.active.push(ActiveTree {
                dimension: self.timber_fall_dimension_id().to_string(),
                player: self.acting_player_name().to_string(),
                cut: pos,
                toward,
                length: height,
                blocks,
                logs,
                drops,
                axe,
                phase: Phase::Warning { since: now },
            });
        }
    }

    /// Advances every active tree: the warning lean, the fall, the impact and the break-up.
    pub(super) fn timber_fall_tick(&mut self) {
        if self.remote.is_some() {
            return;
        }
        let now = self.world.game_time as f64 + self.clock.fraction();
        let mut impacts = Vec::new();
        let mut landed = Vec::new();
        {
            let mut s = state();
            let mut i = 0;
            while i < s.active.len() {
                let mut remove = false;
                match s.active[i].phase {
                    Phase::Warning { since } => {
                        if now - since >= WARNING_SECS {
                            s.active[i].phase = Phase::Falling {
                                angle: START_ANGLE,
                                vel: 0.0,
                            };
                        }
                    }
                    Phase::Falling { angle, vel } => {
                        let step = 0.05 * FALL_SPEED;
                        let length = s.active[i].length.max(1.0);
                        let alpha = 3.0 * GRAVITY / (2.0 * length) * angle.sin();
                        let vel = vel + alpha * step;
                        let angle = angle + vel * step;
                        if self.timber_fall_hits(&s.active[i], angle)
                            || angle >= std::f64::consts::FRAC_PI_2
                        {
                            let angle = angle.min(std::f64::consts::FRAC_PI_2);
                            s.active[i].phase = Phase::Impact { since: now, angle };
                            impacts.push(s.active[i].cut);
                        } else {
                            s.active[i].phase = Phase::Falling { angle, vel };
                        }
                    }
                    Phase::Impact { since, .. } => {
                        if now - since >= BREAKUP_SECS {
                            remove = true;
                        }
                    }
                }
                if remove {
                    landed.push(s.active.remove(i));
                } else {
                    i += 1;
                }
            }
        }
        for cut in impacts {
            self.audio.play_unshared(
                "minecraft:block.wood.fall",
                Some([cut[0] as f64 + 0.5, cut[1] as f64 + 0.5, cut[2] as f64 + 0.5]),
                1.2,
                0.8,
            );
        }
        for tree in landed {
            self.timber_fall_land(tree);
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
        for (pos, _, is_log_block) in &tree.blocks {
            if !is_log_block {
                continue;
            }
            let ox = pos.x as f64 - hinge[0];
            let oy = pos.y as f64 - hinge[1] + 0.5;
            let oz = pos.z as f64 - hinge[2];
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

    /// The tree hit the ground: spawn its drops at the stump and wear the axe.
    fn timber_fall_land(&mut self, tree: ActiveTree) {
        let back = self.dim;
        let target = super::Dim::from_id(&tree.dimension);
        let switched = if back != target {
            self.enter_level(target).is_ok()
        } else {
            false
        };

        let stump = [
            tree.cut[0] as f64 + 0.5,
            tree.cut[1] as f64 + 0.125,
            tree.cut[2] as f64 + 0.5,
        ];
        if self.ui.rules.bool("block_drops") {
            for stack in tree.drops {
                if &*stack.item != "minecraft:air" {
                    self.spawn_item_own(stack, stump, 0);
                }
            }
        }

        if let Some(axe) = tree.axe
            && let Some(line) = timber_fall_wear_line(&axe, &tree.player, tree.logs)
        {
            self.run_command_quietly(&line);
        }

        if switched {
            let _ = self.enter_level(back);
        }
    }

    /// Draws every active tree in the current dimension as rotated model instances.
    pub(super) fn timber_fall_geometry(
        &self,
        draw: &mut horizon_client::world_geometry::WorldGeometry,
    ) {
        let now = self.world.game_time as f64 + self.clock.fraction();
        let s = state();
        if s.active.is_empty() {
            return;
        }
        let dimension = self.timber_fall_dimension_id();
        let mut models: HashMap<BlockStateId, Option<ModelRef>> = HashMap::new();
        for tree in &s.active {
            if tree.dimension != dimension {
                continue;
            }
            let angle = match tree.phase {
                Phase::Warning { since } => {
                    let progress = ((now - since) / WARNING_SECS).clamp(0.0, 1.0);
                    (1.0 + 2.0 * progress).to_radians()
                }
                Phase::Falling { angle, .. } | Phase::Impact { angle, .. } => angle,
            };
            let (sin, cos) = angle.sin_cos();
            let t = 1.0 - cos;
            let (dx, dz) = (tree.toward[0], tree.toward[2]);
            let origin = [
                tree.cut[0] as f64 + 0.5,
                tree.cut[1] as f64,
                tree.cut[2] as f64 + 0.5,
            ];
            for (pos, state, _) in &tree.blocks {
                let model = *models
                    .entry(*state)
                    .or_insert_with(|| self.timber_fall_model(*state));
                let Some(model) = model else {
                    continue;
                };
                let ox = pos.x as f64 - origin[0];
                let oy = pos.y as f64 - origin[1];
                let oz = pos.z as f64 - origin[2];
                let rx = (cos + t * dz * dz) * ox + sin * dx * oy - t * dx * dz * oz;
                let ry = -sin * dx * ox + cos * oy - sin * dz * oz;
                let rz = -t * dx * dz * ox + sin * dz * oy + (cos + t * dx * dx) * oz;
                let transform = [
                    (cos + t * dz * dz) as f32,
                    (sin * dx) as f32,
                    (-t * dx * dz) as f32,
                    rx as f32,
                    (-sin * dx) as f32,
                    cos as f32,
                    (-sin * dz) as f32,
                    ry as f32,
                    (-t * dx * dz) as f32,
                    (sin * dz) as f32,
                    (cos + t * dx * dx) as f32,
                    rz as f32,
                ];
                let instance = ModelInstance::new(
                    origin,
                    transform,
                    0xFFFF_FFFF,
                    LightCoords::FULL_BRIGHT,
                    Overlay::NONE,
                );
                let _ = self.native_world_draw_model_instances(
                    draw,
                    model,
                    std::slice::from_ref(&instance),
                );
            }
        }
    }

    /// Resolves the block's model (`minecraft:block/<name>`) from the shared model store.
    fn timber_fall_model(&self, state: BlockStateId) -> Option<ModelRef> {
        let facts = self.res.facts().get(state);
        let name = facts.name.to_string();
        let path = name.strip_prefix("minecraft:").unwrap_or(&name);
        let id = Identifier::parse(&format!("minecraft:block/{path}")).ok()?;
        self.native_model(&id).ok()
    }
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
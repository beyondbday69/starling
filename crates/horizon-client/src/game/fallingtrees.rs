// Panda's Falling Trees for Horizon (built-in core feature): chopping a tree's log
// with an axe takes the whole tree down, as Falling Trees 0.13.2 (PandaMods) does with
// Homestead's config (generic trees, axes only, 256 logs at most, leaves within 10,
// persistent leaves left alone, sneak to chop one log).
// Settings are kept in `Options > Mods > Falling Trees`.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use super::Game;
use crate::items::ItemStack;
use crate::mesher::Vertex;
use crate::mod_api::NativeMod;
use horizon_core::position::BlockPos;
use horizon_world::registry::BlockStateId;
use winit::keyboard::KeyCode;

pub const ID: &str = "fallingtrees";
pub const NAME: &str = "Falling Trees";
pub const VERSION: &str = "0.13.2";
pub const DESCRIPTION: &str =
    "Trees fall when their base is chopped with an axe. Port of Panda's Falling Trees.";

pub const MAX_LOGS: usize = 256;
pub const MAX_LEAVES_RADIUS: u32 = 10;

/// `#minecraft:logs` (and `#c:logs`) by name.
pub fn is_log(block: &str) -> bool {
    block != "minecraft:mushroom_stem"
        && ["_log", "_wood", "_stem", "_hyphae"]
            .iter()
            .any(|s| block.ends_with(s))
}

/// `#minecraft:leaves` by name.
pub fn is_leaves(block: &str) -> bool {
    block.ends_with("_leaves")
}

/// `#minecraft:axes` by name.
pub fn is_axe(item: &str) -> bool {
    item.ends_with("_axe") && !item.ends_with("_pickaxe")
}

/// The durability of the axes, for the extra wear.
pub fn axe_durability(item: &str) -> Option<u32> {
    Some(match item {
        "minecraft:wooden_axe" => 59,
        "minecraft:golden_axe" => 32,
        "minecraft:stone_axe" => 131,
        "minecraft:copper_axe" => 190,
        "minecraft:iron_axe" => 250,
        "minecraft:diamond_axe" => 1561,
        "minecraft:netherite_axe" => 2031,
        "obsidianequipmentrework:obsidian_axe" => 1800,
        "tokimistoolshed:wooden_lumber_axe" => 144,
        "tokimistoolshed:stone_lumber_axe" => 324,
        "tokimistoolshed:iron_lumber_axe" => 621,
        "tokimistoolshed:golden_lumber_axe" => 81,
        "tokimistoolshed:diamond_lumber_axe" => 3906,
        "tokimistoolshed:netherite_lumber_axe" => 5076,
        "tokimistoolshed:obsidian_lumber_axe" => 4000,
        _ => return None,
    })
}

/// What the finder needs to know about a block.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Block {
    pub name: String,
    /// The leaves' `distance`, if the block has one.
    pub distance: Option<u32>,
    pub persistent: bool,
}

/// A tree to take down, apart from the chopped log.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tree {
    pub logs: Vec<[i32; 3]>,
    pub leaves: Vec<[i32; 3]>,
    /// Vines, cocoa and bee nests.
    pub attached: Vec<[i32; 3]>,
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
pub fn find_tree(start: [i32; 3], read: &dyn Fn([i32; 3]) -> Block) -> Option<Tree> {
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

/// One felled tree, waiting for execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Felling {
    pub dimension: String,
    pub player: String,
    pub origin: [i32; 3],
    pub tree: Tree,
    /// The axe to wear, if survival and a known one.
    pub axe: Option<String>,
}

impl Felling {
    pub fn loot_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        let [ox, oy, oz] = self.origin;
        let (x, y, z) = (
            f64::from(ox) + 0.5,
            f64::from(oy) + 0.125,
            f64::from(oz) + 0.5,
        );
        format!(
            "execute in {} as {} run loot spawn {x:.1} {y:.3} {z:.1} mine {bx} {by} {bz} mainhand",
            self.dimension, self.player
        )
    }

    pub fn clear_line(&self, [bx, by, bz]: [i32; 3]) -> String {
        format!(
            "execute in {} run setblock {bx} {by} {bz} minecraft:air",
            self.dimension
        )
    }

    /// The axe's wear for `logs` felled logs, if it's a known axe.
    pub fn wear_line(&self, logs: usize) -> Option<String> {
        let max = self.axe.as_deref().and_then(axe_durability)?;
        if logs == 0 {
            return None;
        }
        Some(format!(
            "execute as {} run item modify entity @s weapon.mainhand {{\"type\":\"minecraft:set_damage\",\"damage\":{:.6},\"add\":true}}",
            self.player,
            // `SetItemDamageFunction` floors a float: half a point keeps it from landing one short.
            -(logs as f64 + 0.5) / f64::from(max)
        ))
    }

    /// Fells the tree with `run`: each block's drops spawn at the stump, then the block is
    /// cleared. A block whose loot command fails is left standing, so nothing is removed without
    /// its drops; the axe wears for the logs actually felled. Returns how many blocks were felled.
    pub fn run(&self, run: &mut dyn FnMut(&str) -> bool) -> usize {
        let mut felled = 0;
        let mut logs = 0;
        let blocks = self
            .tree
            .attached
            .iter()
            .chain(&self.tree.leaves)
            .map(|b| (b, false));
        for (&b, is_log) in blocks.chain(self.tree.logs.iter().map(|b| (b, true))) {
            let loot = self.loot_line(b);
            if !run(&loot) {
                eprintln!("[fallingtrees] command failed, block left standing: {loot}");
                continue;
            }
            let clear = self.clear_line(b);
            if !run(&clear) {
                eprintln!("[fallingtrees] command failed: {clear}");
                continue;
            }
            felled += 1;
            logs += usize::from(is_log);
        }
        if let Some(line) = self.wear_line(logs)
            && !run(&line)
        {
            eprintln!("[fallingtrees] command failed: {line}");
        }
        felled
    }
}

/// The mod settings for Options > Mods > Falling Trees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallingTreesSettings {
    pub enabled: bool,
    pub break_leaves: bool,
    pub wear_axe: bool,
}

impl FallingTreesSettings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
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

    pub fn labels(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Falling Trees: {}", on_off(self.enabled)),
            format!("Break Leaves: {}", on_off(self.break_leaves)),
            format!("Wear Axe: {}", on_off(self.wear_axe)),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.enabled = !self.enabled,
            1 => self.break_leaves = !self.break_leaves,
            2 => self.wear_axe = !self.wear_axe,
            _ => {}
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 3] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("enabled", flag(self.enabled)),
            ("break_leaves", flag(self.break_leaves)),
            ("wear_axe", flag(self.wear_axe)),
        ]
    }
}

/// An active tree falling in the world.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveFallingTree {
    pub origin: [i32; 3],
    pub toward: [f64; 3],
    pub start_time: f64,
    pub fall_ticks: f64,
    pub max_angle: f64,
}

impl ActiveFallingTree {
    pub const DROP_TICKS: f64 = 5.0;

    /// Calculate fall duration and ground impact angle from tree height and blocks.
    pub fn calculate_fall(height: f64, min_angle: f64) -> (f64, f64) {
        let ticks = Self::DROP_TICKS + 8.0 + 8.0 * height.max(1.0).sqrt();
        let angle = min_angle.clamp(std::f64::consts::FRAC_PI_3, std::f64::consts::FRAC_PI_2);
        (ticks, angle)
    }

    /// Calculate topple progression factor [0.0, 1.0] at elapsed time `t`.
    pub fn progress(&self, elapsed_ticks: f64) -> f64 {
        if elapsed_ticks <= Self::DROP_TICKS {
            0.0
        } else {
            ((elapsed_ticks - Self::DROP_TICKS)
                / (self.fall_ticks - Self::DROP_TICKS).max(0.1))
            .clamp(0.0, 1.0)
        }
    }

    /// Check if the tree has landed (elapsed time >= fall_ticks).
    pub fn has_landed(&self, current_time: f64) -> bool {
        current_time - self.start_time >= self.fall_ticks
    }
}

pub(crate) struct ActiveTree {
    pub(crate) felling: Felling,
    pub(crate) blocks: Vec<([i32; 3], BlockStateId)>,
    pub(crate) drops: Vec<ItemStack>,
    pub(crate) cut: [i32; 3],
    pub(crate) toward: [f64; 3],
    pub(crate) start_time: f64,
    pub(crate) fall_ticks: f64,
    pub(crate) angle: f64,
}

impl ActiveTree {
    pub(crate) fn pivot(&self) -> glam::DVec3 {
        glam::DVec3::new(
            self.cut[0] as f64 + 0.5 + self.toward[0] * 0.5,
            self.cut[1] as f64,
            self.cut[2] as f64 + 0.5 + self.toward[2] * 0.5,
        )
    }

    pub(crate) fn turn(&self, t: f64) -> glam::DMat4 {
        if t < ActiveFallingTree::DROP_TICKS {
            let f = (t / ActiveFallingTree::DROP_TICKS).clamp(0.0, 1.0);
            return glam::DMat4::from_translation(glam::DVec3::NEG_Y * f * f);
        }
        let f = ((t - ActiveFallingTree::DROP_TICKS)
            / (self.fall_ticks - ActiveFallingTree::DROP_TICKS).max(0.1))
        .clamp(0.0, 1.0);
        let pivot = self.pivot();
        let way = glam::DVec3::new(self.toward[0], 0.0, self.toward[2]);
        let axis = glam::DVec3::Y.cross(way).normalize();
        glam::DMat4::from_translation(pivot)
            * glam::DMat4::from_axis_angle(axis, self.angle * f * f)
            * glam::DMat4::from_translation(-pivot - glam::DVec3::Y)
    }
}

#[derive(Default)]
struct State {
    settings: Option<FallingTreesSettings>,
    active: Vec<ActiveTree>,
}

static STATE: Mutex<State> = Mutex::new(State {
    settings: None,
    active: Vec::new(),
});

impl Game<'_> {
    pub(crate) fn fallingtrees_dimension_id(&self) -> &'static str {
        match self.dim {
            super::Dim::Nether => "minecraft:the_nether",
            super::Dim::End => "minecraft:the_end",
            _ => "minecraft:overworld",
        }
    }

    pub(crate) fn fallingtrees_mod_entry() -> NativeMod {
        NativeMod {
            id: ID.into(),
            title: format!("{NAME} {VERSION}"),
            description: DESCRIPTION.into(),
            keys: Vec::new(),
        }
    }

    pub(crate) fn fallingtrees_settings(&self) -> Vec<String> {
        let settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                FallingTreesSettings::read(|key| self.native_setting(ID, key))
            })
        };
        settings.labels()
    }

    pub(crate) fn fallingtrees_settings_click(&mut self, button: usize) {
        let mut settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                FallingTreesSettings::read(|key| self.native_setting(ID, key))
            })
        };
        settings.click(button);
        {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            s.settings = Some(settings);
        }
        for (key, value) in settings.stored() {
            self.native_set_setting(ID, key, &value);
        }
    }

    pub(crate) fn fallingtrees_read_block(&self, pos: [i32; 3]) -> Block {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let facts = self.res.facts().get(state);
        let name = facts.name.to_string();
        let (distance, persistent) =
            if let Some(state_data) = self.pack.assets.registry.state(state) {
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

    pub(crate) fn fallingtrees_destroy_block(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        _canceled: &mut bool,
    ) {
        let settings = {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            *s.settings.get_or_insert_with(|| {
                FallingTreesSettings::read(|key| self.native_setting(ID, key))
            })
        };
        if !settings.enabled {
            return;
        }
        // Sneak: Shift key or touch sneak cancels whole tree fall, chopping only the single log
        if self.keys.contains(&KeyCode::ShiftLeft)
            || self.keys.contains(&KeyCode::ShiftRight)
            || self.touch_sneaking()
        {
            return;
        }
        // Creative and Survival only
        if self.spectator() {
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
        let read = |p: [i32; 3]| self.fallingtrees_read_block(p);
        let Some(mut tree) = find_tree(pos, &read) else {
            return;
        };
        if !settings.break_leaves {
            tree.leaves.clear();
        }

        // 1. Calculate topple vector away from the player
        let px = self.position.x;
        let pz = self.position.z;
        let dx = (pos[0] as f64 + 0.5) - px;
        let dz = (pos[2] as f64 + 0.5) - pz;
        let len = (dx * dx + dz * dz).sqrt();
        let toward = if len > 0.001 {
            [dx / len, 0.0, dz / len]
        } else {
            let rad = (self.yaw as f64).to_radians();
            [-rad.sin(), 0.0, rad.cos()]
        };

        // 2. Calculate tree height and fall physics parameters
        let mut max_y = pos[1];
        for &p in tree.logs.iter().chain(&tree.leaves).chain(&tree.attached) {
            if p[1] > max_y {
                max_y = p[1];
            }
        }
        let height = (max_y - pos[1] + 1) as f64;
        let (fall_ticks, angle) =
            ActiveFallingTree::calculate_fall(height, std::f64::consts::FRAC_PI_2);

        // 3. Collect block positions and pre-calculate item drops
        let mut blocks = Vec::new();
        let mut drops = Vec::new();
        let block_positions: Vec<[i32; 3]> = tree
            .attached
            .iter()
            .chain(&tree.leaves)
            .chain(&tree.logs)
            .copied()
            .collect();

        for p in &block_positions {
            let s = self.world.block(p[0], p[1], p[2]);
            blocks.push((*p, s));
            let d = self.res.items().block_drops(
                self.pack.assets.registry,
                s,
                self.held(),
                &mut self.world.random,
            );
            drops.extend(d);
        }

        // 4. Clear world blocks immediately to avoid ghost block duplication
        for &(p, s) in &blocks {
            let left = self.fluid_left(s);
            let at = BlockPos::new(p[0], p[1], p[2]);
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, left, 3));
        }

        // 5. Initial wood cracking sound
        let dim = self.fallingtrees_dimension_id().to_string();
        self.run_command_quietly(&format!(
            "execute in {dim} run playsound minecraft:block.wood.break block @a {} {} {} 1.0 0.7",
            pos[0], pos[1], pos[2]
        ));

        // 6. Queue active falling tree
        let axe = (settings.wear_axe && !self.creative).then_some(held_item);
        let player = self.acting_player_name().to_string();
        let felling = Felling {
            dimension: dim,
            player,
            origin: pos,
            tree,
            axe,
        };
        let start_time = self.world.game_time as f64 + self.clock.fraction();
        let active = ActiveTree {
            felling,
            blocks,
            drops,
            cut: pos,
            toward,
            start_time,
            fall_ticks,
            angle,
        };
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if !s.active.iter().any(|t| t.cut == pos) {
            s.active.push(active);
        }
    }

    pub(crate) fn fallingtrees_tick(&mut self) {
        let now = self.world.game_time as f64 + self.clock.fraction();
        let mut landed = Vec::new();
        {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            let mut i = 0;
            while i < s.active.len() {
                if now - s.active[i].start_time >= s.active[i].fall_ticks {
                    landed.push(s.active.remove(i));
                } else {
                    i += 1;
                }
            }
        }
        for tree in landed {
            let turn = tree.turn(f64::INFINITY);
            let dim = &tree.felling.dimension;
            let [ox, oy, oz] = tree.cut;

            // Wood impact crash sound
            self.run_command_quietly(&format!(
                "execute in {dim} run playsound minecraft:block.wood.fall block @a {ox} {oy} {oz} 1.2 0.8"
            ));

            // Block break particles (level event 2001) at landing coordinates
            for &(at, state) in &tree.blocks {
                let center = turn.transform_point3(glam::DVec3::new(
                    at[0] as f64 + 0.5,
                    at[1] as f64 + 0.5,
                    at[2] as f64 + 0.5,
                ));
                let land_pos = center.floor().as_ivec3().to_array();
                self.level_event_at(2001, land_pos, state.0 as i32);
            }

            // Spawn drops cleanly at the stump
            let stump_pos = [ox as f64 + 0.5, oy as f64 + 0.125, oz as f64 + 0.5];
            for stack in tree.drops {
                self.spawn_item(stack, stump_pos, [0.0, 0.1, 0.0], 0);
            }

            // Held axe receives durability wear per felled log
            if let Some(line) = tree.felling.wear_line(tree.felling.tree.logs.len()) {
                self.run_command_quietly(&line);
            }
        }
        let _ = self.process_level_events();
    }

    pub(crate) fn fallingtrees_geometry(
        &self,
        camera: glam::DVec3,
        out: &mut Vec<Vertex>,
    ) {
        let now = self.world.game_time as f64 + self.clock.fraction();
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if s.active.is_empty() {
            return;
        }
        for tree in &s.active {
            let t = now - tree.start_time;
            if t < 0.0 || t >= tree.fall_ticks {
                continue;
            }
            let turn = glam::DMat4::from_translation(-camera) * tree.turn(t);
            for &(at, state) in &tree.blocks {
                let corner = glam::DVec3::from_array(at.map(f64::from));
                let pose = (turn * glam::DMat4::from_translation(corner)).as_mat4();
                let light = self.world.light(at[0], at[1], at[2]);
                let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
                let seed = BlockPos::new(at[0], at[1], at[2]);
                let tint = self.particle_sprite(state, at).map(|(_, tint)| tint);
                let start = out.len();
                self.block_model_vertices(
                    state,
                    horizon_core::random::position_seed(seed),
                    pose,
                    packed,
                    false,
                    out,
                );
                if let Some(tint) = tint {
                    let tint = horizon_client::mobs::render::abgr(tint);
                    for vertex in &mut out[start..] {
                        vertex.color = horizon_client::hud_layout::multiply(vertex.color, tint);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn world(blocks: &[([i32; 3], &str, Option<u32>)]) -> HashMap<[i32; 3], Block> {
        blocks
            .iter()
            .map(|&(p, n, d)| {
                (
                    p,
                    Block {
                        name: n.into(),
                        distance: d,
                        persistent: false,
                    },
                )
            })
            .collect()
    }

    fn reader(w: &HashMap<[i32; 3], Block>) -> impl Fn([i32; 3]) -> Block + '_ {
        |p| {
            w.get(&p).cloned().unwrap_or(Block {
                name: "minecraft:air".into(),
                ..Block::default()
            })
        }
    }

    #[test]
    fn names() {
        assert!(is_log("minecraft:oak_log") && is_log("minecraft:stripped_birch_wood"));
        assert!(is_log("minecraft:crimson_stem") && !is_log("minecraft:mushroom_stem"));
        assert!(is_leaves("minecraft:azalea_leaves") && !is_leaves("minecraft:oak_log"));
        assert!(is_axe("minecraft:iron_axe") && !is_axe("minecraft:iron_pickaxe"));
    }

    #[test]
    fn a_small_tree_falls_whole() {
        let w = world(&[
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_log", None),
            ([0, 2, 0], "minecraft:oak_log", None),
            ([1, 2, 0], "minecraft:oak_leaves", Some(1)),
            ([2, 2, 0], "minecraft:oak_leaves", Some(2)),
            ([0, 3, 0], "minecraft:oak_leaves", Some(1)),
            ([1, 1, 0], "minecraft:cocoa", None),
            ([2, 1, 0], "minecraft:vine", None),
            ([2, 0, 0], "minecraft:vine", None),
        ]);
        let t = find_tree([0, 0, 0], &reader(&w)).unwrap();
        assert_eq!(t.logs, vec![[0, 1, 0], [0, 2, 0]]);
        assert_eq!(t.leaves, vec![[0, 3, 0], [1, 2, 0], [2, 2, 0]]);
        assert_eq!(t.attached, vec![[1, 1, 0], [2, 0, 0], [2, 1, 0]]);
    }

    #[test]
    fn logs_below_and_other_trees_leaves_stay() {
        let w = world(&[
            ([0, -1, 0], "minecraft:oak_log", None),
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_leaves", Some(1)),
            // Distance 1 from another log, two steps from ours: not ours.
            ([1, 1, 0], "minecraft:oak_leaves", Some(1)),
        ]);
        let t = find_tree([0, 0, 0], &reader(&w)).unwrap();
        assert!(t.logs.is_empty());
        assert_eq!(t.leaves, vec![[0, 1, 0]]);
    }

    #[test]
    fn no_leaves_or_persistent_leaves_is_no_tree() {
        let mut w = world(&[
            ([0, 0, 0], "minecraft:oak_log", None),
            ([0, 1, 0], "minecraft:oak_log", None),
        ]);
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
        w.insert(
            [0, 2, 0],
            Block {
                name: "minecraft:oak_leaves".into(),
                distance: Some(1),
                persistent: true,
            },
        );
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
    }

    #[test]
    fn too_big_is_no_tree() {
        let mut w = world(&[([0, 300, 0], "minecraft:oak_leaves", Some(1))]);
        for y in 0..300 {
            w.insert(
                [0, y, 0],
                Block {
                    name: "minecraft:oak_log".into(),
                    ..Block::default()
                },
            );
        }
        assert_eq!(find_tree([0, 0, 0], &reader(&w)), None);
    }

    fn felling() -> Felling {
        Felling {
            dimension: "minecraft:overworld".into(),
            player: "Steve".into(),
            origin: [-3, 64, 2],
            tree: Tree {
                logs: vec![[-3, 65, 2]],
                leaves: vec![[-3, 66, 2]],
                attached: vec![],
            },
            axe: Some("minecraft:iron_axe".into()),
        }
    }

    #[test]
    fn each_block_drops_at_the_stump_then_clears_then_the_axe_wears() {
        let mut c = Vec::new();
        let felled = felling().run(&mut |l| {
            c.push(l.to_string());
            true
        });
        assert_eq!(felled, 2);
        assert_eq!(c.len(), 5);
        assert_eq!(
            c[0],
            "execute in minecraft:overworld as Steve run loot spawn -2.5 64.125 2.5 mine -3 66 2 mainhand"
        );
        assert_eq!(
            c[1],
            "execute in minecraft:overworld run setblock -3 66 2 minecraft:air"
        );
        assert_eq!(
            c[3],
            "execute in minecraft:overworld run setblock -3 65 2 minecraft:air"
        );
        assert!(
            c[4].ends_with("{\"type\":\"minecraft:set_damage\",\"damage\":-0.006000,\"add\":true}"),
            "{}",
            c[4]
        );
    }

    #[test]
    fn a_block_whose_loot_fails_stays_and_doesnt_wear_the_axe() {
        let mut c = Vec::new();
        // The log's loot fails: it isn't cleared, and with no log felled the axe doesn't wear.
        let felled = felling().run(&mut |l| {
            c.push(l.to_string());
            !(l.contains("loot spawn") && l.contains("mine -3 65 2"))
        });
        assert_eq!(felled, 1);
        assert!(!c.iter().any(|l| l.contains("setblock -3 65 2")));
        assert!(!c.iter().any(|l| l.contains("set_damage")));
    }

    #[test]
    fn settings_read_and_toggle() {
        let map: HashMap<String, String> = HashMap::new();
        let s = FallingTreesSettings::read(|k| map.get(k).cloned());
        assert!(s.enabled);
        assert!(s.break_leaves);
        assert!(s.wear_axe);
        assert_eq!(
            s.labels(),
            vec!["Falling Trees: ON", "Break Leaves: ON", "Wear Axe: ON",]
        );

        let mut s2 = s;
        s2.click(0);
        assert!(!s2.enabled);
        s2.click(1);
        assert!(!s2.break_leaves);
        s2.click(2);
        assert!(!s2.wear_axe);
        assert_eq!(
            s2.labels(),
            vec!["Falling Trees: OFF", "Break Leaves: OFF", "Wear Axe: OFF",]
        );
    }

    #[test]
    fn active_falling_tree_physics_progression() {
        let (ticks, angle) = ActiveFallingTree::calculate_fall(16.0, std::f64::consts::FRAC_PI_2);
        assert!(ticks >= 20.0);
        assert!(angle >= std::f64::consts::FRAC_PI_3);

        let active = ActiveFallingTree {
            origin: [0, 64, 0],
            toward: [0.0, 0.0, 1.0],
            start_time: 100.0,
            fall_ticks: ticks,
            max_angle: angle,
        };

        assert_eq!(active.progress(0.0), 0.0);
        assert_eq!(active.progress(ActiveFallingTree::DROP_TICKS), 0.0);
        assert!(active.progress(ticks * 0.5) > 0.0);
        assert_eq!(active.progress(ticks), 1.0);
        assert!(active.has_landed(100.0 + ticks));
        assert!(!active.has_landed(100.0 + ticks - 1.0));
    }
}

//! The Horizon adapter: hooks on both sides feeding the pure-Rust runtime.
//!
//! **UNTESTED in game.** Nothing here has been compiled against a built game or run
//! in a world; the machine this crate was written on cannot build Horizon. Every
//! symbol comes from `crates/horizon-mod-vocab` and the generated `Hooks` trait, and
//! the pure logic it calls is covered by the crate's tests.
//!
//! Known gaps:
//! - no transport carries the sync messages to the client yet: payloads need a
//!   registration step this adapter does not have, so `TimberClientMirror` is ready and
//!   rendered, but only a future payload or single-player bridge fills it;
//! - `unbreaking_level` is passed as 0: the API exposes enchantment effects but the
//!   stack's enchantment component still needs a reader;
//! - the config file is parsed by the pure `TimberConfig::parse`, but the mod has no
//!   file-read API here, so the adapter runs on defaults;
//! - leaf loot uses fixed vanilla-style chances and a roll derived from the landing
//!   position; Fortune and shears are not modelled.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, MutexGuard};

use mod_api::{
    BlockShapeKind, BlockState, BlockUpdateFlags, BreakBlockEvent, CameraShakeEvent, ClientHost,
    EntityRef, Flow, Host, HostSide, Identifier, ItemStack, LevelRef, LevelTickPostEvent,
    LightCoords, ModelInstance, ModelRef, Overlay, PlayerBreakSpeedEvent, PlayerRef, ServerHost,
    ServerStoppingEvent, SoundSource, SubmitCustomGeometryEvent, WorldDrawContext,
};

use crate::chop::{ChopContext, ToolState, chop_speed_multiplier};
use crate::config::TimberConfig;
use crate::drops::DropPlan;
use crate::impact::{CollisionGrid, impact_damage};
use crate::runtime::{
    FxEvent, StartFallOutcome, TickOutputs, TimberClientMirror, TimberWorldRuntime,
    TwoByTwoCutTracker,
};
use crate::scanner::{BlockKind, BlockPos, TreeSpecies, scan_tree};

/// Ticks between two break-speed scans of the same block.
const BREAK_SPEED_CACHE_TICKS: i64 = 20;
/// How far around the hinge entities are hurt by the impact.
const IMPACT_DAMAGE_RADIUS: f64 = 8.0;
/// Vanilla's default pickup delay, in ticks.
const ITEM_PICKUP_DELAY: u32 = 10;

/// The mod's state on one side of the game.
#[derive(Default)]
pub struct TimberFallMod {
    runtime: Mutex<TimberWorldRuntime>,
    config: Mutex<TimberConfig>,
    /// Block name behind each variant, so drops and models can be resolved.
    names: Mutex<HashMap<u16, String>>,
    models: Mutex<HashMap<u16, ModelRef>>,
    break_speed: Mutex<Option<BreakSpeedCache>>,
    cut_tracker: Mutex<TwoByTwoCutTracker>,
    /// What the client draws; filled by the sync transport when it lands.
    mirror: Mutex<TimberClientMirror>,
}

struct BreakSpeedCache {
    level: LevelRef,
    pos: [i32; 3],
    bucket: i64,
    multiplier: f32,
}

#[mod_api::horizon_mod]
impl Hooks for TimberFallMod {
    /// A player's destroy speed for a log already includes the tree's size.
    fn player_break_speed(&self, host: HostSide, event: &mut PlayerBreakSpeedEvent) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        let Some(level) = player.level(&*host) else {
            return Flow::Continue;
        };
        let bucket = level.game_time(&*host).unwrap_or(0) / BREAK_SPEED_CACHE_TICKS;
        let cached = {
            let cache = self.break_speed();
            cache
                .as_ref()
                .filter(|cache| {
                    cache.level == level && cache.pos == event.pos && cache.bucket == bucket
                })
                .map(|cache| cache.multiplier)
        };
        let multiplier = match cached {
            Some(multiplier) => multiplier,
            None => {
                let names = RefCell::new(HashMap::new());
                let reader = LevelBlockReader {
                    host: &*host,
                    level,
                    names: &names,
                };
                let scan = scan_tree(
                    &reader,
                    BlockPos::new(event.pos[0], event.pos[1], event.pos[2]),
                    &self.config(),
                );
                let multiplier = match scan {
                    Ok(tree) => chop_speed_multiplier(tree.logs.len()),
                    Err(_) => 1.0,
                };
                *self.break_speed() = Some(BreakSpeedCache {
                    level,
                    pos: event.pos,
                    bucket,
                    multiplier,
                });
                multiplier
            }
        };
        event.new_speed *= multiplier;
        Flow::Continue
    }

    /// A log was cut: clear the whole tree and start its fall, or let vanilla break it.
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        let cut = BlockPos::new(event.pos[0], event.pos[1], event.pos[2]);
        let Some(state_text) = host.block_state_string(event.state) else {
            return Flow::Continue;
        };
        if !is_log(&state_text) {
            return Flow::Continue;
        }
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        let Some(level) = player.level(&*host) else {
            return Flow::Continue;
        };
        let entity: EntityRef = player.into();
        let player_pos = entity.position(&*host).unwrap_or([0.0, 0.0, 0.0]);
        let ctx = ChopContext {
            sneaking: entity.is_crouching(&*host).unwrap_or(false),
            has_permission: player.may_build(&*host).unwrap_or(true),
            tool: self.tool_state(&*host, player),
        };
        let config = self.config();
        let dimension_id = level_dimension_id(&*host, level);
        let now = level.game_time(&*host).unwrap_or(0).max(0) as u64;
        let cuts = {
            let mut tracker = self.cut_tracker();
            match tracker.record(dimension_id, cut, now) {
                // The new cut must come first: the scan starts from a log that is still there.
                Some(base) => std::iter::once(cut)
                    .chain(base.into_iter().filter(|pos| *pos != cut))
                    .collect(),
                None => vec![cut],
            }
        };
        let names = RefCell::new(HashMap::new());
        let outcome = {
            let reader = LevelBlockReader {
                host: &*host,
                level,
                names: &names,
            };
            self.runtime().try_start_fall(
                &reader,
                &cuts,
                &ctx,
                &config,
                &[],
                dimension_id,
                [player_pos[0] as f32, player_pos[2] as f32],
            )
        };

        let StartFallOutcome::FellTree {
            blocks_to_remove,
            tool_damage,
            hunger_exhaustion,
            ..
        } = outcome
        else {
            return Flow::Continue;
        };

        let Some(air) = host.block_state_id("minecraft:air") else {
            return Flow::Continue;
        };
        for block in blocks_to_remove {
            let _ = level.set_block(
                host,
                [block.x, block.y, block.z],
                air,
                BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS,
            );
        }
        self.names().extend(names.into_inner());
        self.wear_tool(host, player, tool_damage);
        let _ = player.add_exhaustion(host, hunger_exhaustion);
        // The tree's drops come from the runtime's plan; vanilla must not also drop
        // the cut log, or every felled tree would duplicate one item.
        event.cancelled = true;
        Flow::Cancel
    }

    /// Advance the fall on the server and act on what it produced.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        match host {
            HostSide::Server(host) => {
                let level = event.level;
                let dimension_id = level_dimension_id(&*host, level);
                let outputs = {
                    let grid = LevelCollisionGrid {
                        host: &*host,
                        level,
                    };
                    self.runtime().tick_dimension(dimension_id, 0.05, &grid)
                };
                self.apply_tick_outputs(host, level, outputs);
            }
            HostSide::Client(_) => self.mirror().advance(0.05),
        }
    }

    /// Draw every active tree's blocks as rotated model instances.
    fn submit_custom_geometry(
        &self,
        host: &dyn ClientHost,
        event: &mut SubmitCustomGeometryEvent<'_>,
    ) {
        let mut instances = self.mirror().collect_render_instances();
        instances.extend(self.runtime().collect_render_instances());
        if instances.is_empty() {
            return;
        }
        let mut batches: BTreeMap<u16, Vec<ModelInstance>> = BTreeMap::new();
        for instance in instances {
            batches
                .entry(instance.variant)
                .or_default()
                .push(ModelInstance::new(
                    instance.origin,
                    instance.transform_3x4,
                    0xFFFF_FFFF,
                    LightCoords::FULL_BRIGHT,
                    Overlay::NONE,
                ));
        }
        for (variant, batch) in batches {
            let Some(model) = self.model_for(host, variant) else {
                continue;
            };
            if event.draw.draw_model_instances(model, &batch).is_err() {
                // A resource reload makes the handle stale; resolve it again next frame.
                self.models().remove(&variant);
            }
        }
    }

    /// Add the decaying impact shake to this frame's camera.
    fn camera_shake(&self, _host: &dyn ClientHost, event: &mut CameraShakeEvent<'_>) {
        let shake = self.runtime().camera_shake_offset();
        if shake <= 0.0 {
            return;
        }
        event.shake.pitch += shake;
        event.shake.roll += shake * 0.5;
    }

    /// The server is stopping: drop every tree still in the air.
    fn server_stopping(&self, host: &mut dyn ServerHost, _event: &mut ServerStoppingEvent) {
        let plans = self.runtime().drain_all_midfall_drops();
        if plans.is_empty() {
            return;
        }
        let Some(level) = host.levels().list_levels().into_iter().next() else {
            return;
        };
        let names = self.names();
        for plan in plans {
            spawn_plan(host, level, &plan, &names);
        }
    }
}

impl TimberFallMod {
    fn runtime(&self) -> MutexGuard<'_, TimberWorldRuntime> {
        self.runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn config(&self) -> TimberConfig {
        *self
            .config
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn names(&self) -> MutexGuard<'_, HashMap<u16, String>> {
        self.names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn models(&self) -> MutexGuard<'_, HashMap<u16, ModelRef>> {
        self.models
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn break_speed(&self) -> MutexGuard<'_, Option<BreakSpeedCache>> {
        self.break_speed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cut_tracker(&self) -> MutexGuard<'_, TwoByTwoCutTracker> {
        self.cut_tracker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn mirror(&self) -> MutexGuard<'_, TimberClientMirror> {
        self.mirror
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn tool_state(&self, host: &dyn ServerHost, player: PlayerRef) -> ToolState {
        let creative = player.is_creative(host).unwrap_or(false);
        let Some(slot) = player.selected_slot(host) else {
            return ToolState {
                is_axe: false,
                remaining_durability: 0,
                unbreaking_level: 0,
                creative_mode: creative,
            };
        };
        let Some(stack) = player
            .inventory(host)
            .and_then(|inventory| inventory.get(slot).cloned())
        else {
            return ToolState {
                is_axe: false,
                remaining_durability: 0,
                unbreaking_level: 0,
                creative_mode: creative,
            };
        };
        let item = stack.item();
        let max_damage = host.max_damage(item).unwrap_or(0);
        ToolState {
            is_axe: item.id(host).ends_with("_axe"),
            remaining_durability: max_damage.saturating_sub(stack.damage()),
            // Reading the real Unbreaking level needs a component reader; see the module docs.
            unbreaking_level: 0,
            creative_mode: creative,
        }
    }

    fn wear_tool(&self, host: &mut dyn ServerHost, player: PlayerRef, wear: u32) {
        if wear == 0 {
            return;
        }
        let Some(slot) = player.selected_slot(&*host) else {
            return;
        };
        let Some(mut stack) = player
            .inventory(&*host)
            .and_then(|inventory| inventory.get(slot).cloned())
        else {
            return;
        };
        stack.set_damage(stack.damage() + wear);
        let _ = player.set_inventory_slot(host, slot as u32, Some(stack));
    }

    fn apply_tick_outputs(&self, host: &mut dyn ServerHost, level: LevelRef, outputs: TickOutputs) {
        for fx in &outputs.fx {
            self.play_fx(host, level, *fx);
        }
        for impact in outputs.impacts {
            self.damage_around(host, level, impact);
        }
        for completed in outputs.completed {
            let names = self.names();
            spawn_plan(host, level, &completed.drops, &names);
            replant(host, level, &completed.drops, &names);
        }
    }

    fn play_fx(&self, host: &mut dyn ServerHost, level: LevelRef, fx: FxEvent) {
        match fx {
            FxEvent::CreakSound { hinge, .. } => {
                let _ = level.play_sound(
                    host,
                    "minecraft:block.wood.break",
                    hinge_center(hinge),
                    SoundSource::Blocks,
                    0.5,
                    0.6,
                );
            }
            FxEvent::LeafBurstParticles { hinge, .. } => {
                let at = [hinge.x, hinge.y + 3, hinge.z];
                emit_block_particles(host, level, at, "minecraft:oak_leaves");
                let _ = level.play_sound(
                    host,
                    "minecraft:block.grass.break",
                    hinge_center(hinge),
                    SoundSource::Blocks,
                    0.4,
                    1.4,
                );
            }
            FxEvent::ImpactDustAndThud { hinge, .. } => {
                let _ = level.play_sound(
                    host,
                    "minecraft:block.wood.fall",
                    hinge_center(hinge),
                    SoundSource::Blocks,
                    0.9,
                    0.8,
                );
                emit_block_particles(
                    host,
                    level,
                    [hinge.x, hinge.y, hinge.z],
                    "minecraft:oak_log",
                );
            }
        }
    }

    fn damage_around(
        &self,
        host: &mut dyn ServerHost,
        level: LevelRef,
        impact: crate::runtime::ImpactEvent,
    ) {
        let center = [
            impact.hinge.x as f64 + 0.5,
            impact.hinge.y as f64,
            impact.hinge.z as f64 + 0.5,
        ];
        let min = [
            center[0] - IMPACT_DAMAGE_RADIUS,
            center[1] - 4.0,
            center[2] - IMPACT_DAMAGE_RADIUS,
        ];
        let max = [
            center[0] + IMPACT_DAMAGE_RADIUS,
            center[1] + 8.0,
            center[2] + IMPACT_DAMAGE_RADIUS,
        ];
        let Some(entities) = level.entities_in(&*host, min, max) else {
            return;
        };
        let falling_damage = self.config().falling_damage;
        for entity in entities {
            let Some(at) = entity.position(&*host) else {
                continue;
            };
            let dx = at[0] - center[0];
            let dy = at[1] - center[1];
            let dz = at[2] - center[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt() as f32;
            let damage = impact_damage(impact.speed, distance, falling_damage);
            if damage > 0.0 {
                let _ = entity.hurt(host, "minecraft:fall", damage, None, None);
            }
        }
    }

    fn model_for(&self, host: &dyn ClientHost, variant: u16) -> Option<ModelRef> {
        let cached = { self.models().get(&variant).copied() };
        if let Some(model) = cached {
            return Some(model);
        }
        let name = { self.names().get(&variant)?.clone() };
        let path = name.strip_prefix("minecraft:").unwrap_or(&name);
        let id = Identifier::parse(&format!("minecraft:block/{path}")).ok()?;
        let model = host.render_assets().model(&id).ok()?;
        self.models().insert(variant, model);
        Some(model)
    }
}

/// Reads a level as the pure scanner's `BlockReader`, recording block names by variant.
struct LevelBlockReader<'a, 'n> {
    host: &'a dyn Host,
    level: LevelRef,
    names: &'n RefCell<HashMap<u16, String>>,
}

impl<'a, 'n> LevelBlockReader<'a, 'n> {
    fn classify(&self, state: BlockState) -> BlockKind {
        let Some(text) = self.host.block_state_string(state) else {
            return BlockKind::Air;
        };
        let (name, properties) = split_state(&text);
        let path = name.strip_prefix("minecraft:").unwrap_or(name);
        if path.ends_with("_leaves") {
            let variant = variant_of(name);
            self.names.borrow_mut().insert(variant, name.to_owned());
            return BlockKind::Leaf {
                species: species_of(path),
                persistent: properties.contains("persistent=true"),
                variant,
            };
        }
        if is_log_path(path) {
            let variant = variant_of(name);
            self.names.borrow_mut().insert(variant, name.to_owned());
            return BlockKind::Log {
                species: species_of(path),
                variant,
            };
        }
        if path == "mangrove_roots" {
            return BlockKind::MangroveRoot;
        }
        if path == "nether_wart_block" || path == "warped_wart_block" {
            let variant = variant_of(name);
            self.names.borrow_mut().insert(variant, name.to_owned());
            return BlockKind::WartBlock { variant };
        }
        BlockKind::Passable
    }
}

impl BlockReader for LevelBlockReader<'_, '_> {
    fn block_at(&self, pos: BlockPos) -> BlockKind {
        let Some(state) = self.level.block_state(self.host, [pos.x, pos.y, pos.z]) else {
            return BlockKind::Air;
        };
        self.classify(state)
    }
}

/// Solid terrain for the fall: a block with a collision shape stops the trunk, so
/// grass and flowers (empty collision shapes) let it pass.
struct LevelCollisionGrid<'a> {
    host: &'a dyn Host,
    level: LevelRef,
}

impl CollisionGrid for LevelCollisionGrid<'_> {
    fn is_solid_obstacle(&self, pos: BlockPos) -> bool {
        let at = [pos.x, pos.y, pos.z];
        let Some(state) = self.level.block_state(self.host, at) else {
            return false;
        };
        self.level
            .block_shape(self.host, state, at, BlockShapeKind::Collision)
            .is_some_and(|boxes| !boxes.is_empty())
    }
}

fn split_state(text: &str) -> (&str, &str) {
    match text.split_once('[') {
        Some((name, properties)) => (name, properties.trim_end_matches(']')),
        None => (text, ""),
    }
}

fn is_log(text: &str) -> bool {
    let (name, _) = split_state(text);
    is_log_path(name.strip_prefix("minecraft:").unwrap_or(name))
}

fn is_log_path(path: &str) -> bool {
    path.ends_with("_log")
        || path.ends_with("_wood")
        || path.ends_with("_stem")
        || path.ends_with("_hyphae")
}

fn species_of(path: &str) -> TreeSpecies {
    if path.starts_with("crimson_") || path.starts_with("warped_") {
        TreeSpecies::NetherFungus
    } else if path.starts_with("mangrove") {
        TreeSpecies::Mangrove
    } else if path.starts_with("cherry") {
        TreeSpecies::Cherry
    } else {
        TreeSpecies::Standard
    }
}

/// A stable id for one block name, the opaque `variant` the pure modules compare.
fn variant_of(name: &str) -> u16 {
    name.bytes().fold(0u16, |hash, byte| {
        (hash ^ u16::from(byte)).wrapping_mul(167)
    })
}

/// The sapling block a log's tree replants with, when the tree is known.
fn sapling_id(log_name: &str) -> Option<String> {
    let path = log_name.strip_prefix("minecraft:").unwrap_or(log_name);
    if path == "mangrove_log" || path == "mangrove_wood" {
        return Some("minecraft:mangrove_propagule".into());
    }
    let base = path
        .strip_suffix("_log")
        .or_else(|| path.strip_suffix("_wood"))
        .or_else(|| path.strip_suffix("_stem"))?;
    if base == "crimson" || base == "warped" {
        Some(format!("minecraft:{base}_fungus"))
    } else {
        Some(format!("minecraft:{base}_sapling"))
    }
}

/// Spawns the plan's log stacks. Leaf entries need their loot roll, which the
/// adapter does not evaluate yet; see the module docs.
fn spawn_plan(
    host: &mut dyn ServerHost,
    level: LevelRef,
    plan: &DropPlan,
    names: &HashMap<u16, String>,
) {
    for stack in &plan.stacks {
        if stack.is_leaf {
            spawn_leaf_loot(host, level, names, stack);
        } else if let Some(name) = names.get(&stack.item_variant) {
            spawn_item(host, level, name, stack.count, stack.landing_pos);
        }
    }
}

fn spawn_item(host: &mut dyn ServerHost, level: LevelRef, name: &str, count: u32, at: [f64; 3]) {
    let Ok(id) = Identifier::parse(name) else {
        return;
    };
    let Some(item) = host.item_key(&id) else {
        return;
    };
    let Some(item_stack) = ItemStack::new(item, count) else {
        return;
    };
    let _ = level.spawn_item(host, at[0], at[1], at[2], item_stack, ITEM_PICKUP_DELAY);
}

/// Rolls a leaf entry's vanilla-style loot: sapling, stick and oak/dark oak apple.
///
/// The roll is derived from the landing position and the leaf variant, so a leaf
/// always produces the same drops; canal fortune and shears are not modelled.
fn spawn_leaf_loot(
    host: &mut dyn ServerHost,
    level: LevelRef,
    names: &HashMap<u16, String>,
    stack: &crate::drops::DropStack,
) {
    let Some(name) = names.get(&stack.item_variant) else {
        return;
    };
    let path = name.strip_prefix("minecraft:").unwrap_or(name);
    if path.ends_with("_wart_block") {
        spawn_item(host, level, name, 1, stack.landing_pos);
        return;
    }
    let Some(base) = path.strip_suffix("_leaves") else {
        return;
    };
    let sapling = if base == "mangrove" {
        "minecraft:mangrove_propagule".to_owned()
    } else {
        format!("minecraft:{base}_sapling")
    };
    let sapling_chance = if base == "jungle" { 0.025 } else { 0.05 };
    if leaf_roll(stack.landing_pos, stack.item_variant, 1) < sapling_chance {
        spawn_item(host, level, &sapling, 1, stack.landing_pos);
    }
    if leaf_roll(stack.landing_pos, stack.item_variant, 2) < 0.02 {
        spawn_item(host, level, "minecraft:stick", 1, stack.landing_pos);
    }
    if matches!(base, "oak" | "dark_oak")
        && leaf_roll(stack.landing_pos, stack.item_variant, 3) < 0.005
    {
        spawn_item(host, level, "minecraft:apple", 1, stack.landing_pos);
    }
}

fn leaf_roll(at: [f64; 3], variant: u16, salt: u64) -> f32 {
    let mut hash =
        0xcbf2_9ce4_8422_2325_u64 ^ u64::from(variant) ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for part in at {
        hash = (hash ^ part.to_bits()).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    (hash >> 40) as f32 / 16_777_216.0
}

fn hinge_center(hinge: BlockPos) -> [f64; 3] {
    [hinge.x as f64 + 0.5, hinge.y as f64, hinge.z as f64 + 0.5]
}

fn emit_block_particles(host: &mut dyn ServerHost, level: LevelRef, at: [i32; 3], block: &str) {
    let Some(block_key) = host.block_key(block) else {
        return;
    };
    let Some(state) = host.block_state_id(block) else {
        return;
    };
    let _ = level.block_event(host, at, block_key, 2001, state as i32);
}

/// The level's dimension name hashed to the runtime's opaque u64 key.
fn level_dimension_id(host: &dyn Host, level: LevelRef) -> u64 {
    let Some(name) = level.dimension().id(host) else {
        return 0;
    };
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in name.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn replant(
    host: &mut dyn ServerHost,
    level: LevelRef,
    plan: &DropPlan,
    names: &HashMap<u16, String>,
) {
    let Some(replant) = plan.replant else {
        return;
    };
    let Some(log_name) = names.get(&replant.sapling_variant) else {
        return;
    };
    let Some(sapling) = sapling_id(log_name) else {
        return;
    };
    let Some(state) = host.block_state_id(&sapling) else {
        return;
    };
    let _ = level.set_block(
        host,
        [replant.pos.x, replant.pos.y, replant.pos.z],
        state,
        BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS,
    );
}

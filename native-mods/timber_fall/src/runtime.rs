//! The mod's runtime: active falling trees, render batches, camera shake and recovery.
//!
//! Everything here is pure Rust. The Horizon adapter only feeds it block reads, tool
//! state and collision answers, and consumes the plans and transforms it returns.

use crate::chop::{ChopContext, ChopDecision, SingleBlockReason, decide_chop};
use crate::config::TimberConfig;
use crate::drops::{DropPlan, plan_breakup_drops, plan_midfall_drops};
use crate::falling::{FallDirection, FallingBlock, FallingTree};
use crate::impact::{CollisionGrid, block_transform_3x4};
use crate::scanner::{
    BlockPos, BlockReader, ScanRejectReason, ScannedTree, TreeSpecies, scan_tree_with_cuts,
};

/// Identifies one active falling tree for the server's bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActiveTreeId(pub u64);

/// One tree in flight, with the config chosen when it was felled.
#[derive(Clone, Debug)]
pub struct ActiveTreeEntry {
    pub id: ActiveTreeId,
    pub tree: FallingTree,
    pub config: TimberConfig,
    pub species: TreeSpecies,
    pub variant: u16,
    /// Opaque key of the level the tree was cut in.
    pub dimension_id: u64,
}

/// A block ready to be handed to `ModelInstance`.
///
/// `origin` is the hinge in world space and `transform_3x4` is relative to it, so
/// `ModelInstance::new(origin, transform_3x4, ...)` places the block's model origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBlockInstance {
    pub variant: u16,
    pub is_leaf: bool,
    pub origin: [f64; 3],
    pub transform_3x4: [f32; 12],
    pub block_pos: BlockPos,
}

/// What a cut did: fell the scanned tree, or let vanilla break one block.
#[derive(Clone, Debug, PartialEq)]
pub enum StartFallOutcome {
    FellTree {
        tree_id: ActiveTreeId,
        /// Every log and leaf to clear in the same tick, so nothing drops twice.
        blocks_to_remove: Vec<BlockPos>,
        tool_damage: u32,
        hunger_exhaustion: f32,
    },
    VanillaSingleBlock {
        reason: SingleBlockReason,
    },
}

/// A tree that just hit the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactEvent {
    pub tree_id: ActiveTreeId,
    pub hinge: BlockPos,
    pub speed: f32,
}

/// A tree that finished falling, with the items to spawn.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletedTree {
    pub tree_id: ActiveTreeId,
    pub hinge: BlockPos,
    pub species: TreeSpecies,
    pub variant: u16,
    pub drops: DropPlan,
}

/// Lightweight effects the adapter turns into sounds and particles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FxEvent {
    /// The warning is over and the trunk is moving: the creak and the leaves shaking free.
    CreakSound {
        tree_id: ActiveTreeId,
        hinge: BlockPos,
    },
    LeafBurstParticles {
        tree_id: ActiveTreeId,
        hinge: BlockPos,
    },
    /// The trunk hit the ground.
    ImpactDustAndThud {
        tree_id: ActiveTreeId,
        hinge: BlockPos,
        speed: f32,
    },
}

/// What one runtime tick produced, for the adapter to act on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickOutputs {
    pub entered_falling: Vec<ActiveTreeId>,
    pub impacts: Vec<ImpactEvent>,
    pub completed: Vec<CompletedTree>,
    pub fx: Vec<FxEvent>,
}

/// Peak camera shake, in degrees, when an impact happens with the option on.
pub const CAMERA_SHAKE_IMPULSE: f32 = 1.0;
/// How long a full shake takes to decay to zero.
pub const CAMERA_SHAKE_DECAY_SECS: f32 = 0.5;
/// A log stack's cap when the caller does not name one.
pub const DEFAULT_MAX_STACK_SIZE: u32 = 64;

/// Everything one cut needs to decide on and start a fall.
pub struct StartFallRequest<'a> {
    pub world: &'a dyn BlockReader,
    pub cuts: &'a [BlockPos],
    pub ctx: &'a ChopContext,
    pub config: &'a TimberConfig,
    pub wear_rolls: &'a [f32],
    pub dimension_id: u64,
    /// The cutter's horizontal position; the tree tips away from it.
    pub player: [f32; 2],
}

/// The mod's state on one side of the game.
#[derive(Debug)]
pub struct TimberWorldRuntime {
    next_id: u64,
    trees: Vec<ActiveTreeEntry>,
    shake: f32,
    max_stack_size: u32,
}

impl Default for TimberWorldRuntime {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_STACK_SIZE)
    }
}

impl TimberWorldRuntime {
    pub fn new(max_stack_size: u32) -> Self {
        Self {
            next_id: 0,
            trees: Vec::new(),
            shake: 0.0,
            max_stack_size: max_stack_size.max(1),
        }
    }

    pub fn active_trees(&self) -> &[ActiveTreeEntry] {
        &self.trees
    }

    pub fn active_tree_mut(&mut self, id: ActiveTreeId) -> Option<&mut ActiveTreeEntry> {
        self.trees.iter_mut().find(|entry| entry.id == id)
    }

    /// Scans the cut, decides whether it may fell the tree, and starts the fall.
    pub fn try_start_fall(&mut self, request: StartFallRequest<'_>) -> StartFallOutcome {
        let config = request.config.sanitized();
        let scan = scan_tree_with_cuts(request.world, request.cuts, &config);
        match decide_chop(request.ctx, &scan, &config, request.wear_rolls) {
            ChopDecision::VanillaSingleBlock { reason } => {
                StartFallOutcome::VanillaSingleBlock { reason }
            }
            ChopDecision::FellTree {
                tool_damage,
                hunger_exhaustion,
            } => {
                let scanned = match scan {
                    Ok(tree) => tree,
                    Err(reason) => {
                        // `decide_chop` already refused a rejected scan.
                        return StartFallOutcome::VanillaSingleBlock {
                            reason: SingleBlockReason::NotATree(reason),
                        };
                    }
                };
                let hinge = match request.cuts.first() {
                    Some(&cut) => cut,
                    None => {
                        return StartFallOutcome::VanillaSingleBlock {
                            reason: SingleBlockReason::NotATree(ScanRejectReason::NotALog),
                        };
                    }
                };
                let id = self.add_tree(
                    &scanned,
                    request.cuts,
                    hinge,
                    request.dimension_id,
                    request.player,
                    &config,
                );
                StartFallOutcome::FellTree {
                    tree_id: id,
                    blocks_to_remove: removal_plan(&scanned),
                    tool_damage,
                    hunger_exhaustion,
                }
            }
        }
    }

    /// Advances every active tree and removes the ones that finished falling.
    pub fn tick(&mut self, dt: f32, grid: &dyn CollisionGrid) -> TickOutputs {
        self.tick_filter(None, dt, grid)
    }

    /// Advances only the trees cut in `dimension_id`, leaving the other levels' trees alone.
    pub fn tick_dimension(
        &mut self,
        dimension_id: u64,
        dt: f32,
        grid: &dyn CollisionGrid,
    ) -> TickOutputs {
        self.tick_filter(Some(dimension_id), dt, grid)
    }

    fn tick_filter(
        &mut self,
        dimension: Option<u64>,
        dt: f32,
        grid: &dyn CollisionGrid,
    ) -> TickOutputs {
        let mut outputs = TickOutputs::default();
        if dt.is_finite() && dt > 0.0 {
            let decay = dt * CAMERA_SHAKE_IMPULSE / CAMERA_SHAKE_DECAY_SECS;
            self.shake = (self.shake - decay).max(0.0);
        }

        let mut survivors = Vec::with_capacity(self.trees.len());
        for mut entry in std::mem::take(&mut self.trees) {
            if dimension.is_some_and(|id| id != entry.dimension_id) {
                survivors.push(entry);
                continue;
            }
            let events = entry.tree.step(dt, grid);
            if let Some(speed) = events.landed {
                outputs.fx.push(FxEvent::ImpactDustAndThud {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                    speed,
                });
                if entry.config.camera_shake {
                    self.shake = self.shake.max(CAMERA_SHAKE_IMPULSE * 0.4);
                }
            }
            if events.entered_falling {
                outputs.entered_falling.push(entry.id);
                outputs.fx.push(FxEvent::CreakSound {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                });
                outputs.fx.push(FxEvent::LeafBurstParticles {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                });
            }
            if let Some(speed) = events.impacted {
                outputs.impacts.push(ImpactEvent {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                    speed,
                });
                outputs.fx.push(FxEvent::ImpactDustAndThud {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                    speed,
                });
                if entry.config.camera_shake {
                    self.shake = self.shake.max(CAMERA_SHAKE_IMPULSE);
                }
            }
            if events.ready_to_break_up {
                outputs.completed.push(CompletedTree {
                    tree_id: entry.id,
                    hinge: entry.tree.hinge,
                    species: entry.species,
                    variant: entry.variant,
                    drops: plan_breakup_drops(&entry.tree, &entry.config, self.max_stack_size),
                });
            } else {
                survivors.push(entry);
            }
        }
        self.trees = survivors;
        outputs
    }

    /// Rotated local transforms for every block of every active tree.
    pub fn collect_render_instances(&self) -> Vec<RenderBlockInstance> {
        let mut instances = Vec::new();
        for entry in &self.trees {
            collect_tree_instances(&entry.tree, entry.tree.angle_rad(), &mut instances);
        }
        instances
    }

    /// The decaying shake the client adds to the camera this frame.
    pub fn camera_shake_offset(&self) -> f32 {
        self.shake
    }

    /// Every active tree's blocks as immediate drops, for a quit or unload.
    pub fn drain_all_midfall_drops(&mut self) -> Vec<DropPlan> {
        let trees = std::mem::take(&mut self.trees);
        trees
            .iter()
            .map(|entry| plan_midfall_drops(&entry.tree, &entry.config, self.max_stack_size))
            .collect()
    }

    fn add_tree(
        &mut self,
        scanned: &ScannedTree,
        cuts: &[BlockPos],
        hinge: BlockPos,
        dimension_id: u64,
        player: [f32; 2],
        config: &TimberConfig,
    ) -> ActiveTreeId {
        let direction = FallDirection::away_from_player(player[0], player[1], hinge);
        // The cut logs are gone, so the tree drops onto the stump before it tips.
        let is_cut = |log: &BlockPos| log.y == hinge.y && cuts.contains(log);
        let dropping = scanned.logs.iter().any(is_cut);
        let trunk_height = scanned
            .logs
            .iter()
            .filter(|log| !is_cut(log))
            .map(|log| (log.y - hinge.y) as f32)
            .fold(1.0, f32::max);

        let mut blocks = Vec::with_capacity(scanned.logs.len() + scanned.leaves.len());
        for log in &scanned.logs {
            let mut block = FallingBlock::relative_to(hinge, *log, false, scanned.variant);
            block.vanished = is_cut(log);
            blocks.push(block);
        }
        for leaf in &scanned.leaves {
            blocks.push(FallingBlock::relative_to(
                hinge,
                leaf.pos,
                true,
                leaf.variant,
            ));
        }

        let id = ActiveTreeId(self.next_id);
        self.next_id += 1;
        self.trees.push(ActiveTreeEntry {
            id,
            tree: FallingTree::new(hinge, direction, trunk_height, config.fall_speed, blocks)
                .with_drop(if dropping { 1.0 } else { 0.0 }),
            config: *config,
            species: scanned.species,
            variant: scanned.variant,
            dimension_id,
        });
        id
    }
}

fn removal_plan(scanned: &ScannedTree) -> Vec<BlockPos> {
    let mut blocks = scanned.logs.clone();
    blocks.extend(scanned.leaves.iter().map(|leaf| leaf.pos));
    blocks
}

fn collect_tree_instances(tree: &FallingTree, angle_rad: f32, out: &mut Vec<RenderBlockInstance>) {
    let origin = tree.hinge_center();
    let shift = tree.base_shift();
    for block in tree.blocks.iter().filter(|block| !block.vanished) {
        out.push(RenderBlockInstance {
            variant: block.variant,
            is_leaf: block.is_leaf,
            origin,
            transform_3x4: block_transform_3x4(
                [0.0, 0.0, 0.0],
                tree.direction,
                angle_rad,
                [block.offset[0], block.offset[1] + shift, block.offset[2]],
            ),
            block_pos: block.pos,
        });
    }
}

/// How long a partial 2x2 cut stays live before it is forgotten.
pub const TWO_BY_TWO_CUT_EXPIRY_TICKS: u64 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecordedCut {
    dimension_id: u64,
    pos: BlockPos,
    tick_time: u64,
}

/// Remembers recent base-log cuts so a 2x2 trunk falls once all four are cut in sequence.
#[derive(Clone, Debug)]
pub struct TwoByTwoCutTracker {
    expiry_ticks: u64,
    cuts: Vec<RecordedCut>,
}

impl Default for TwoByTwoCutTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TwoByTwoCutTracker {
    pub fn new() -> Self {
        Self::with_expiry(TWO_BY_TWO_CUT_EXPIRY_TICKS)
    }

    pub fn with_expiry(expiry_ticks: u64) -> Self {
        Self {
            expiry_ticks,
            cuts: Vec::new(),
        }
    }

    /// Records one cut and returns all four base logs once they form a 2x2 square.
    ///
    /// Cuts older than the expiry are dropped first, as is an earlier record of the
    /// same position (a re-cut refreshes it). A completed square is consumed, so the
    /// same four positions do not complete twice.
    pub fn record(
        &mut self,
        dimension_id: u64,
        pos: BlockPos,
        tick_time: u64,
    ) -> Option<[BlockPos; 4]> {
        self.cuts.retain(|cut| {
            tick_time.saturating_sub(cut.tick_time) <= self.expiry_ticks
                && !(cut.dimension_id == dimension_id && cut.pos == pos)
        });
        self.cuts.push(RecordedCut {
            dimension_id,
            pos,
            tick_time,
        });

        for dx in -1..=0 {
            for dz in -1..=0 {
                let corner = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
                let mut base = [corner; 4];
                let mut index = 0;
                let mut complete = true;
                for ox in 0..2 {
                    for oz in 0..2 {
                        let candidate = BlockPos::new(corner.x + ox, corner.y, corner.z + oz);
                        let recorded = self
                            .cuts
                            .iter()
                            .any(|cut| cut.dimension_id == dimension_id && cut.pos == candidate);
                        if recorded {
                            base[index] = candidate;
                            index += 1;
                        } else {
                            complete = false;
                        }
                    }
                }
                if complete {
                    self.cuts.retain(|cut| {
                        !(cut.dimension_id == dimension_id && base.contains(&cut.pos))
                    });
                    return Some(base);
                }
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.cuts.clear();
    }

    pub fn pending(&self) -> usize {
        self.cuts.len()
    }
}

/// The fall phase a client receives. Same shape as [`FallPhase`] so a snapshot is a copy.
pub use crate::falling::FallPhase as SyncPhase;

/// A tree snapshot for the client: geometry, canopy ids and the current phase.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncFallingTree {
    pub id: ActiveTreeId,
    pub dimension_id: u64,
    pub hinge: BlockPos,
    pub direction: FallDirection,
    pub trunk_height: f32,
    pub fall_speed: f32,
    pub blocks: Vec<FallingBlock>,
    pub species: TreeSpecies,
    pub variant: u16,
    pub phase: SyncPhase,
    pub drop_height: f32,
}

impl SyncFallingTree {
    pub fn from_entry(entry: &ActiveTreeEntry) -> Self {
        Self {
            id: entry.id,
            dimension_id: entry.dimension_id,
            hinge: entry.tree.hinge,
            direction: entry.tree.direction,
            trunk_height: entry.tree.trunk_height,
            fall_speed: entry.tree.fall_speed,
            blocks: entry.tree.blocks.clone(),
            species: entry.species,
            variant: entry.variant,
            phase: entry.tree.phase,
            drop_height: entry.tree.drop_height,
        }
    }
}

/// Server to client messages for one falling tree.
#[derive(Clone, Debug, PartialEq)]
pub enum SyncMessage {
    StartFall(SyncFallingTree),
    UpdatePhase {
        tree_id: ActiveTreeId,
        phase: SyncPhase,
    },
    RemoveTree {
        tree_id: ActiveTreeId,
    },
}

/// How fast the client catches up to the last server angle, in radians per second.
const CLIENT_ANGLE_RATE: f32 = 12.0;

#[derive(Clone, Debug)]
struct MirrorEntry {
    id: ActiveTreeId,
    tree: FallingTree,
    target_angle: f32,
    render_angle: f32,
}

/// The client's copy of the server's falling trees, smoothing angles between ticks.
#[derive(Clone, Debug, Default)]
pub struct TimberClientMirror {
    trees: Vec<MirrorEntry>,
}

impl TimberClientMirror {
    pub fn apply(&mut self, message: SyncMessage) {
        match message {
            SyncMessage::StartFall(sync) => {
                let mut tree = FallingTree::new(
                    sync.hinge,
                    sync.direction,
                    sync.trunk_height,
                    sync.fall_speed,
                    sync.blocks,
                )
                .with_drop(sync.drop_height);
                tree.phase = sync.phase;
                let angle = tree.angle_rad();
                self.trees.retain(|entry| entry.id != sync.id);
                self.trees.push(MirrorEntry {
                    id: sync.id,
                    tree,
                    target_angle: angle,
                    render_angle: angle,
                });
            }
            SyncMessage::UpdatePhase { tree_id, phase } => {
                let Some(entry) = self.trees.iter_mut().find(|entry| entry.id == tree_id) else {
                    return;
                };
                entry.tree.phase = phase;
                entry.target_angle = entry.tree.angle_rad();
                if !matches!(phase, SyncPhase::Falling { .. }) {
                    entry.render_angle = entry.target_angle;
                }
            }
            SyncMessage::RemoveTree { tree_id } => {
                self.trees.retain(|entry| entry.id != tree_id);
            }
        }
    }

    /// Moves the rendered angles towards the last server tick.
    pub fn advance(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let factor = (dt * CLIENT_ANGLE_RATE).min(1.0);
        for entry in &mut self.trees {
            if matches!(entry.tree.phase, SyncPhase::Falling { .. }) {
                entry.render_angle += (entry.target_angle - entry.render_angle) * factor;
            } else {
                entry.render_angle = entry.target_angle;
            }
        }
    }

    pub fn collect_render_instances(&self) -> Vec<RenderBlockInstance> {
        let mut instances = Vec::new();
        for entry in &self.trees {
            collect_tree_instances(&entry.tree, entry.render_angle, &mut instances);
        }
        instances
    }

    pub fn tree_count(&self) -> usize {
        self.trees.len()
    }

    pub fn render_angle(&self, id: ActiveTreeId) -> Option<f32> {
        self.trees
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.render_angle)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::chop::{SingleBlockReason, ToolState};
    use crate::falling::FallPhase;
    use crate::scanner::BlockKind;

    const OAK: u16 = 10;
    const LEAF: u16 = 60;
    const DIMENSION: u64 = 7;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    #[derive(Default)]
    struct TestWorld {
        blocks: HashMap<BlockPos, BlockKind>,
    }

    impl TestWorld {
        fn log(&mut self, at: BlockPos) {
            self.blocks.insert(
                at,
                BlockKind::Log {
                    species: TreeSpecies::Standard,
                    variant: OAK,
                },
            );
        }

        fn leaf(&mut self, at: BlockPos) {
            self.blocks.insert(
                at,
                BlockKind::Leaf {
                    species: TreeSpecies::Standard,
                    persistent: false,
                    variant: LEAF,
                },
            );
        }
    }

    impl BlockReader for TestWorld {
        fn block_at(&self, at: BlockPos) -> BlockKind {
            self.blocks.get(&at).copied().unwrap_or(BlockKind::Air)
        }
    }

    /// Nothing is solid.
    struct NoObstacles;

    impl CollisionGrid for NoObstacles {
        fn is_solid_obstacle(&self, _pos: BlockPos) -> bool {
            false
        }
    }

    struct EverythingSolid;

    impl CollisionGrid for EverythingSolid {
        fn is_solid_obstacle(&self, _pos: BlockPos) -> bool {
            true
        }
    }

    /// A five-log oak with seven natural leaves, hinged at `hinge`.
    fn oak(hinge: BlockPos, blocks: &mut TestWorld) -> Vec<BlockPos> {
        for y in 0..5 {
            blocks.log(pos(hinge.x, hinge.y + y, hinge.z));
        }
        let leaves = [
            pos(hinge.x + 1, hinge.y + 4, hinge.z),
            pos(hinge.x - 1, hinge.y + 4, hinge.z),
            pos(hinge.x, hinge.y + 4, hinge.z + 1),
            pos(hinge.x, hinge.y + 4, hinge.z - 1),
            pos(hinge.x + 1, hinge.y + 5, hinge.z),
            pos(hinge.x - 1, hinge.y + 5, hinge.z),
            pos(hinge.x, hinge.y + 5, hinge.z + 1),
        ];
        for leaf in leaves {
            blocks.leaf(leaf);
        }
        leaves.to_vec()
    }

    fn cutter() -> ChopContext {
        ChopContext {
            sneaking: false,
            has_permission: true,
            tool: ToolState {
                is_axe: true,
                remaining_durability: 100,
                unbreaking_level: 0,
                creative_mode: false,
            },
        }
    }

    fn start_in(
        runtime: &mut TimberWorldRuntime,
        world: &TestWorld,
        hinge: BlockPos,
        dimension_id: u64,
    ) -> ActiveTreeId {
        match runtime.try_start_fall(StartFallRequest {
            world,
            cuts: &[hinge],
            ctx: &cutter(),
            config: &TimberConfig::default(),
            wear_rolls: &[],
            dimension_id,
            player: [-3.5, 0.5],
        }) {
            StartFallOutcome::FellTree { tree_id, .. } => tree_id,
            other => panic!("expected the tree to fall, got {other:?}"),
        }
    }

    fn start(runtime: &mut TimberWorldRuntime, world: &TestWorld, hinge: BlockPos) -> ActiveTreeId {
        start_in(runtime, world, hinge, DIMENSION)
    }

    #[test]
    fn starting_a_fall_returns_every_block_and_the_costs() {
        let mut world = TestWorld::default();
        let leaves = oak(pos(0, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();

        let outcome = runtime.try_start_fall(StartFallRequest {
            world: &world,
            cuts: &[pos(0, 0, 0)],
            ctx: &cutter(),
            config: &TimberConfig::default(),
            wear_rolls: &[],
            dimension_id: DIMENSION,
            player: [-3.5, 0.5],
        });

        let mut expected: Vec<BlockPos> = (0..5).map(|y| pos(0, y, 0)).collect();
        expected.extend(leaves);
        expected.sort();

        match outcome {
            StartFallOutcome::FellTree {
                tree_id,
                blocks_to_remove,
                tool_damage,
                hunger_exhaustion,
            } => {
                assert_eq!(tree_id, ActiveTreeId(0));
                assert_eq!(blocks_to_remove.len(), 12);
                let mut actual = blocks_to_remove.clone();
                actual.sort();
                assert_eq!(actual, expected);
                assert_eq!(tool_damage, 5);
                assert!((hunger_exhaustion - 0.025).abs() < 1e-6);
            }
            other => panic!("expected the tree to fall, got {other:?}"),
        }

        let entries = runtime.active_trees();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tree.hinge, pos(0, 0, 0));
        assert_eq!(entries[0].species, TreeSpecies::Standard);
        assert_eq!(entries[0].tree.blocks.len(), 12);
        assert!(entries[0].tree.direction.dx > 0.99);
        assert!(entries[0].tree.direction.dz.abs() < 1e-6);
        assert!((entries[0].tree.trunk_height - 4.0).abs() < 1e-6);
        assert_eq!(entries[0].dimension_id, DIMENSION);
        let leaf_variants: Vec<u16> = entries[0]
            .tree
            .blocks
            .iter()
            .filter(|block| block.is_leaf)
            .map(|block| block.variant)
            .collect();
        assert_eq!(leaf_variants.len(), 7);
        assert!(leaf_variants.iter().all(|variant| *variant == LEAF));
    }

    #[test]
    fn a_refused_cut_leaves_the_runtime_untouched() {
        let mut world = TestWorld::default();
        for y in 0..4 {
            world.log(pos(0, y, 0));
        }
        let mut runtime = TimberWorldRuntime::default();

        let outcome = runtime.try_start_fall(StartFallRequest {
            world: &world,
            cuts: &[pos(0, 0, 0)],
            ctx: &cutter(),
            config: &TimberConfig::default(),
            wear_rolls: &[],
            dimension_id: DIMENSION,
            player: [-3.5, 0.5],
        });

        assert_eq!(
            outcome,
            StartFallOutcome::VanillaSingleBlock {
                reason: SingleBlockReason::NotATree(ScanRejectReason::InsufficientNaturalLeaves)
            }
        );
        assert!(runtime.active_trees().is_empty());
        assert_eq!(runtime.camera_shake_offset(), 0.0);
    }

    #[test]
    fn tick_completes_every_active_tree_and_removes_it() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        oak(pos(100, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let first = start(&mut runtime, &world, pos(0, 0, 0));
        let second = start(&mut runtime, &world, pos(100, 0, 0));
        assert_eq!(runtime.active_trees().len(), 2);

        for id in [first, second] {
            let entry = runtime.active_tree_mut(id).expect("an active tree");
            entry.tree.phase = crate::falling::FallPhase::Impact {
                impact_angle_rad: FRAC_PI_2,
                impact_speed: 2.0,
                bounce_elapsed: 0.99,
            };
        }

        let outputs = runtime.tick(0.05, &NoObstacles);

        assert_eq!(outputs.completed.len(), 2);
        assert!(outputs.entered_falling.is_empty());
        assert!(outputs.impacts.is_empty());
        assert!(runtime.active_trees().is_empty());
        for completed in outputs.completed {
            let logs: u32 = completed
                .drops
                .stacks
                .iter()
                .filter(|stack| !stack.is_leaf)
                .map(|stack| stack.count)
                .sum();
            assert_eq!(logs, 5);
            let leaf_entries = completed
                .drops
                .stacks
                .iter()
                .filter(|stack| stack.is_leaf)
                .count();
            assert_eq!(leaf_entries, 7);
        }
    }

    #[test]
    fn render_instances_rotate_every_block_through_the_phases() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let id = start(&mut runtime, &world, pos(0, 0, 0));

        let warning = runtime.collect_render_instances();
        // The cut log is gone, so it is not drawn.
        assert_eq!(warning.len(), 11);
        assert_eq!(
            warning.iter().filter(|instance| instance.is_leaf).count(),
            7
        );
        let top_log = warning
            .iter()
            .find(|instance| instance.block_pos == pos(0, 4, 0))
            .expect("the top log");
        assert_eq!(top_log.origin, [0.5, 0.0, 0.5]);
        let warning_x = top_log.transform_3x4[3];

        let entry = runtime.active_tree_mut(id).expect("an active tree");
        entry.tree.phase = crate::falling::FallPhase::Falling {
            angle_rad: FRAC_PI_2 / 2.0,
            angular_vel: 0.0,
        };
        let falling = runtime.collect_render_instances();
        let falling_x = falling
            .iter()
            .find(|instance| instance.block_pos == pos(0, 4, 0))
            .expect("the top log")
            .transform_3x4[3];
        assert!(falling_x > warning_x);

        let entry = runtime.active_tree_mut(id).expect("an active tree");
        entry.tree.phase = crate::falling::FallPhase::Impact {
            impact_angle_rad: FRAC_PI_2,
            impact_speed: 2.0,
            bounce_elapsed: 0.0,
        };
        let impact = runtime.collect_render_instances();
        let matrix = impact
            .iter()
            .find(|instance| instance.block_pos == pos(0, 4, 0))
            .expect("the top log")
            .transform_3x4;
        assert!((matrix[3] - 3.0).abs() < 1e-5, "x was {}", matrix[3]);
        assert!((matrix[7] - 0.5).abs() < 1e-5, "y was {}", matrix[7]);
        assert_eq!(impact.len(), 11);
    }

    #[test]
    fn camera_shake_spikes_on_impact_and_decays_to_zero() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let id = start(&mut runtime, &world, pos(0, 0, 0));
        let entry = runtime.active_tree_mut(id).expect("an active tree");
        entry.tree.phase = crate::falling::FallPhase::Falling {
            angle_rad: 0.0,
            angular_vel: 0.0,
        };

        let outputs = runtime.tick(0.05, &EverythingSolid);

        assert_eq!(outputs.impacts.len(), 1);
        assert!((runtime.camera_shake_offset() - CAMERA_SHAKE_IMPULSE).abs() < 1e-6);
        for _ in 0..10 {
            runtime.tick(0.05, &NoObstacles);
        }
        assert_eq!(runtime.camera_shake_offset(), 0.0);
    }

    #[test]
    fn camera_shake_stays_zero_when_the_option_is_off() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        let config = TimberConfig {
            camera_shake: false,
            ..TimberConfig::default()
        };
        let mut runtime = TimberWorldRuntime::default();
        let id = match runtime.try_start_fall(StartFallRequest {
            world: &world,
            cuts: &[pos(0, 0, 0)],
            ctx: &cutter(),
            config: &config,
            wear_rolls: &[],
            dimension_id: DIMENSION,
            player: [-3.5, 0.5],
        }) {
            StartFallOutcome::FellTree { tree_id, .. } => tree_id,
            other => panic!("expected the tree to fall, got {other:?}"),
        };
        let entry = runtime.active_tree_mut(id).expect("an active tree");
        entry.tree.phase = crate::falling::FallPhase::Falling {
            angle_rad: 0.0,
            angular_vel: 0.0,
        };

        let outputs = runtime.tick(0.05, &EverythingSolid);

        assert_eq!(outputs.impacts.len(), 1);
        assert_eq!(runtime.camera_shake_offset(), 0.0);
    }

    #[test]
    fn midfall_drain_recovers_every_block_and_clears_the_runtime() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        oak(pos(100, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        start(&mut runtime, &world, pos(0, 0, 0));
        start(&mut runtime, &world, pos(100, 0, 0));

        let plans = runtime.drain_all_midfall_drops();

        assert_eq!(plans.len(), 2);
        for plan in &plans {
            let logs: u32 = plan
                .stacks
                .iter()
                .filter(|stack| !stack.is_leaf)
                .map(|stack| stack.count)
                .sum();
            assert_eq!(logs, 5);
            assert_eq!(plan.stacks.iter().filter(|stack| stack.is_leaf).count(), 7);
        }
        assert!(runtime.active_trees().is_empty());
        assert!(runtime.drain_all_midfall_drops().is_empty());
    }

    #[test]
    fn tick_dimension_leaves_other_levels_alone() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        oak(pos(100, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let first = start_in(&mut runtime, &world, pos(0, 0, 0), 1);
        let second = start_in(&mut runtime, &world, pos(100, 0, 0), 2);
        for id in [first, second] {
            let entry = runtime.active_tree_mut(id).expect("an active tree");
            entry.tree.phase = FallPhase::Impact {
                impact_angle_rad: FRAC_PI_2,
                impact_speed: 2.0,
                bounce_elapsed: 0.99,
            };
        }

        let outputs = runtime.tick_dimension(1, 0.05, &NoObstacles);

        assert_eq!(outputs.completed.len(), 1);
        assert_eq!(outputs.completed[0].tree_id, first);
        assert_eq!(runtime.active_trees().len(), 1);
        let remaining = &runtime.active_trees()[0];
        assert_eq!(remaining.id, second);
        assert_eq!(remaining.dimension_id, 2);
        assert!(matches!(
            remaining.tree.phase,
            FallPhase::Impact { bounce_elapsed, .. } if (bounce_elapsed - 0.99).abs() < 1e-6
        ));

        let outputs = runtime.tick_dimension(2, 0.05, &NoObstacles);
        assert_eq!(outputs.completed.len(), 1);
        assert!(runtime.active_trees().is_empty());
    }

    #[test]
    fn the_two_by_two_tracker_finishes_on_the_fourth_cut() {
        let mut tracker = TwoByTwoCutTracker::new();
        let base = [pos(0, 0, 0), pos(1, 0, 0), pos(0, 0, 1), pos(1, 0, 1)];

        assert_eq!(tracker.record(1, base[0], 0), None);
        assert_eq!(tracker.record(1, base[1], 1), None);
        assert_eq!(tracker.record(1, base[2], 2), None);
        let mut completed = tracker
            .record(1, base[3], 3)
            .expect("the fourth cut completes the square");
        completed.sort();
        let mut expected = base;
        expected.sort();
        assert_eq!(completed, expected);
        assert_eq!(tracker.pending(), 0);
    }

    #[test]
    fn the_two_by_two_tracker_expires_stale_cuts() {
        let base = [pos(0, 0, 0), pos(1, 0, 0), pos(0, 0, 1), pos(1, 0, 1)];

        let mut fresh = TwoByTwoCutTracker::new();
        assert_eq!(fresh.record(1, base[0], 0), None);
        assert_eq!(fresh.record(1, base[1], 1), None);
        assert_eq!(fresh.record(1, base[2], 2), None);
        assert!(
            fresh
                .record(1, base[3], TWO_BY_TWO_CUT_EXPIRY_TICKS)
                .is_some()
        );

        let mut stale = TwoByTwoCutTracker::new();
        assert_eq!(stale.record(1, base[0], 0), None);
        assert_eq!(stale.record(1, base[1], 1), None);
        assert_eq!(stale.record(1, base[2], 2), None);
        assert_eq!(
            stale.record(1, base[3], TWO_BY_TWO_CUT_EXPIRY_TICKS + 3),
            None
        );
        assert_eq!(stale.pending(), 1);
    }

    #[test]
    fn the_client_mirror_interpolates_and_removes() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let id = start(&mut runtime, &world, pos(0, 0, 0));
        let entry = runtime.active_trees()[0].clone();

        let mut mirror = TimberClientMirror::default();
        mirror.apply(SyncMessage::StartFall(SyncFallingTree::from_entry(&entry)));

        let instances = mirror.collect_render_instances();
        assert_eq!(instances.len(), 11);
        assert_eq!(
            instances.iter().filter(|instance| instance.is_leaf).count(),
            7
        );
        assert!(
            instances
                .iter()
                .filter(|instance| instance.is_leaf)
                .all(|instance| instance.variant == LEAF)
        );
        assert!(
            instances
                .iter()
                .filter(|instance| !instance.is_leaf)
                .all(|instance| instance.variant == OAK)
        );

        let start_angle = mirror.render_angle(id).expect("a mirrored tree");
        mirror.apply(SyncMessage::UpdatePhase {
            tree_id: id,
            phase: FallPhase::Falling {
                angle_rad: 1.0,
                angular_vel: 0.0,
            },
        });
        mirror.advance(0.05);
        let mid_angle = mirror.render_angle(id).expect("a mirrored tree");
        assert!(mid_angle > start_angle && mid_angle < 1.0);

        mirror.advance(1.0);
        let settled = mirror.render_angle(id).expect("a mirrored tree");
        assert!((settled - 1.0).abs() < 1e-6);

        let top_log = mirror
            .collect_render_instances()
            .into_iter()
            .find(|instance| instance.block_pos == pos(0, 4, 0))
            .expect("the top log");
        assert!(top_log.transform_3x4[3] > 0.0);

        mirror.apply(SyncMessage::RemoveTree { tree_id: id });
        assert_eq!(mirror.tree_count(), 0);
        assert!(mirror.collect_render_instances().is_empty());
    }

    #[test]
    fn fx_events_follow_the_phase_transitions() {
        let mut world = TestWorld::default();
        oak(pos(0, 0, 0), &mut world);
        let mut runtime = TimberWorldRuntime::default();
        let id = start(&mut runtime, &world, pos(0, 0, 0));

        let mut entered = false;
        for _ in 0..40 {
            let outputs = runtime.tick(0.05, &NoObstacles);
            if outputs.entered_falling.contains(&id) {
                entered = true;
                assert!(outputs.fx.contains(&FxEvent::CreakSound {
                    tree_id: id,
                    hinge: pos(0, 0, 0),
                }));
                assert!(outputs.fx.contains(&FxEvent::LeafBurstParticles {
                    tree_id: id,
                    hinge: pos(0, 0, 0),
                }));
                break;
            }
        }
        assert!(entered);

        let entry = runtime.active_tree_mut(id).expect("an active tree");
        entry.tree.phase = FallPhase::Falling {
            angle_rad: 0.0,
            angular_vel: 0.0,
        };
        let outputs = runtime.tick(0.05, &EverythingSolid);
        assert!(outputs.fx.iter().any(|fx| matches!(
            fx,
            FxEvent::ImpactDustAndThud { tree_id, .. } if *tree_id == id
        )));
    }
}

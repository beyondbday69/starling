//! Tree detection: a breadth-first search for logs from the cut upward, then the
//! validity checks the concept requires before a structure may fall.
//!
//! The scanner never touches the world directly; a [`BlockReader`] answers one
//! position at a time, so the rules run on a plain map in tests.

use std::collections::{HashSet, VecDeque};

use crate::config::TimberConfig;

/// A block position in integer world coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// The tree families whose canopy and shape rules differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeSpecies {
    Standard,
    NetherFungus,
    Mangrove,
    Cherry,
}

/// What the scanner needs to know about one block.
///
/// `variant` is the block id the game reports for the log or wart family, so two
/// logs count as the same tree only when their variant matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Air,
    Log {
        species: TreeSpecies,
        variant: u16,
    },
    Leaf {
        species: TreeSpecies,
        persistent: bool,
        variant: u16,
    },
    WartBlock {
        variant: u16,
    },
    MangroveRoot,
    Solid,
    Passable,
}

/// Reads the world one block at a time.
pub trait BlockReader {
    fn block_at(&self, pos: BlockPos) -> BlockKind;
}

/// Why a structure may not fall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanRejectReason {
    /// The position the player cut is not a log.
    NotALog,
    /// More logs than the configured maximum.
    TooLarge,
    /// Fewer than [`MIN_NATURAL_LEAVES`] naturally grown leaves touch the logs.
    InsufficientNaturalLeaves,
    /// A 2x2 trunk was not cut at all four logs of its cutting height.
    IncompleteTwoByTwo,
}

/// A leaf (or, for a nether fungus, a wart block) that belongs to the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeLeaf {
    pub pos: BlockPos,
    /// The block id of this leaf, so the adapter can resolve its model and loot.
    pub variant: u16,
}

/// A structure that passed every check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScannedTree {
    pub species: TreeSpecies,
    pub variant: u16,
    /// Logs from the cutting height upward; logs below it stay as a stump.
    pub logs: Vec<BlockPos>,
    /// Natural leaves (for a nether fungus, wart blocks) that belong to the tree.
    pub leaves: Vec<TreeLeaf>,
    /// The four logs of a 2x2 trunk at cutting height, when the cut is part of one.
    pub trunk_2x2: Option<[BlockPos; 4]>,
}

pub type ScanResult = Result<ScannedTree, ScanRejectReason>;

/// Leaves a structure needs before it counts as a tree.
pub const MIN_NATURAL_LEAVES: usize = 5;

/// How many blocks away from the nearest log a leaf may sit and still belong to the tree.
const MAX_LEAF_DEPTH: i32 = 6;

/// Upper bound on collected leaves, so a huge canopy cannot stall the server.
const MAX_LEAVES: usize = 4096;

/// Scans the tree the player cut at `cut_pos`.
///
/// Logs below the cutting height are left out, so they remain as a stump. A 2x2
/// trunk is reported in [`ScannedTree::trunk_2x2`]; [`scan_tree_with_cuts`] is the
/// variant that also requires all four base logs to have been cut.
pub fn scan_tree(world: &dyn BlockReader, cut_pos: BlockPos, config: &TimberConfig) -> ScanResult {
    let config = config.sanitized();
    let (species, variant) = match world.block_at(cut_pos) {
        BlockKind::Log { species, variant } => (species, variant),
        _ => return Err(ScanRejectReason::NotALog),
    };

    let logs = collect_logs(world, cut_pos, species, variant, config.max_logs)?;
    let trunk_2x2 = trunk_2x2_at(world, cut_pos, variant);
    let leaves = collect_leaves(world, &logs, species);

    if leaves.len() < MIN_NATURAL_LEAVES {
        return Err(ScanRejectReason::InsufficientNaturalLeaves);
    }

    Ok(ScannedTree {
        species,
        variant,
        logs,
        leaves,
        trunk_2x2,
    })
}

/// Scans from `cuts[0]` and, when it is a 2x2 trunk, requires every log of the
/// square at cutting height to be in `cuts`, as the concept's "all 4 logs" rule says.
pub fn scan_tree_with_cuts(
    world: &dyn BlockReader,
    cuts: &[BlockPos],
    config: &TimberConfig,
) -> ScanResult {
    let Some(&cut_pos) = cuts.first() else {
        return Err(ScanRejectReason::NotALog);
    };

    let tree = scan_tree(world, cut_pos, config)?;
    if let Some(base) = tree.trunk_2x2
        && !base.iter().all(|pos| cuts.contains(pos))
    {
        return Err(ScanRejectReason::IncompleteTwoByTwo);
    }
    Ok(tree)
}

/// Breadth-first walk over matching logs, including diagonal neighbours. Mangrove
/// roots bridge logs for mangrove trees only, because that trunk is split by them.
fn collect_logs(
    world: &dyn BlockReader,
    cut_pos: BlockPos,
    species: TreeSpecies,
    variant: u16,
    max_logs: usize,
) -> Result<Vec<BlockPos>, ScanRejectReason> {
    let mut logs = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(cut_pos);
    queue.push_back(cut_pos);

    while let Some(log) = queue.pop_front() {
        logs.push(log);
        if logs.len() > max_logs {
            return Err(ScanRejectReason::TooLarge);
        }

        for neighbour in neighbours(log) {
            if neighbour.y < cut_pos.y {
                continue;
            }
            match world.block_at(neighbour) {
                BlockKind::Log {
                    variant: neighbour_variant,
                    ..
                } if neighbour_variant == variant => {
                    if visited.insert(neighbour) {
                        queue.push_back(neighbour);
                    }
                }
                BlockKind::MangroveRoot if species == TreeSpecies::Mangrove => {
                    for bridged in neighbours(neighbour) {
                        if bridged.y < cut_pos.y || visited.contains(&bridged) {
                            continue;
                        }
                        if let BlockKind::Log {
                            variant: bridged_variant,
                            ..
                        } = world.block_at(bridged)
                            && bridged_variant == variant
                            && visited.insert(bridged)
                        {
                            queue.push_back(bridged);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(logs)
}

/// The 2x2 square of matching logs at the cutting height that contains `cut_pos`.
fn trunk_2x2_at(world: &dyn BlockReader, cut_pos: BlockPos, variant: u16) -> Option<[BlockPos; 4]> {
    for dx in -1..=0 {
        for dz in -1..=0 {
            let corner = BlockPos::new(cut_pos.x + dx, cut_pos.y, cut_pos.z + dz);
            let mut base = [corner; 4];
            let mut index = 0;
            let mut complete = true;
            for ox in 0..2 {
                for oz in 0..2 {
                    let pos = BlockPos::new(corner.x + ox, corner.y, corner.z + oz);
                    match world.block_at(pos) {
                        BlockKind::Log {
                            variant: found_variant,
                            ..
                        } if found_variant == variant => {
                            base[index] = pos;
                            index += 1;
                        }
                        _ => complete = false,
                    }
                }
            }
            if complete {
                return Some(base);
            }
        }
    }
    None
}

/// Natural leaves (and, for a nether fungus, wart blocks) reachable from the logs.
/// Player-placed leaves are neither counted nor walked through.
fn collect_leaves(
    world: &dyn BlockReader,
    logs: &[BlockPos],
    species: TreeSpecies,
) -> Vec<TreeLeaf> {
    let mut leaves = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    for log in logs {
        for neighbour in neighbours(*log) {
            if let Some(variant) = canopy_variant(world.block_at(neighbour), species)
                && visited.insert(neighbour)
            {
                leaves.push(TreeLeaf {
                    pos: neighbour,
                    variant,
                });
                queue.push_back((neighbour, 1));
            }
        }
    }

    while let Some((leaf, depth)) = queue.pop_front() {
        if depth >= MAX_LEAF_DEPTH || leaves.len() >= MAX_LEAVES {
            continue;
        }
        for neighbour in neighbours(leaf) {
            if let Some(variant) = canopy_variant(world.block_at(neighbour), species)
                && visited.insert(neighbour)
            {
                leaves.push(TreeLeaf {
                    pos: neighbour,
                    variant,
                });
                queue.push_back((neighbour, depth + 1));
            }
        }
    }

    leaves
}

/// The block id of a canopy block that belongs to this tree, if it is one.
fn canopy_variant(kind: BlockKind, species: TreeSpecies) -> Option<u16> {
    match kind {
        BlockKind::Leaf {
            species: leaf_species,
            persistent: false,
            variant,
        } if leaf_species == species => Some(variant),
        BlockKind::WartBlock { variant } if species == TreeSpecies::NetherFungus => Some(variant),
        _ => None,
    }
}

/// The 26 positions around `pos`, diagonals included.
fn neighbours(pos: BlockPos) -> impl Iterator<Item = BlockPos> {
    (-1..=1).flat_map(move |dy| {
        (-1..=1).flat_map(move |dz| {
            (-1..=1)
                .filter(move |&dx| (dx, dy, dz) != (0, 0, 0))
                .map(move |dx| BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    const OAK: u16 = 10;
    const SPRUCE: u16 = 11;
    const CRIMSON: u16 = 20;
    const MANGROVE: u16 = 30;
    const CHERRY: u16 = 40;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    #[derive(Default)]
    struct TestWorld {
        blocks: HashMap<BlockPos, BlockKind>,
    }

    impl TestWorld {
        fn put(&mut self, at: BlockPos, kind: BlockKind) {
            self.blocks.insert(at, kind);
        }

        fn log(&mut self, at: BlockPos, species: TreeSpecies, variant: u16) {
            self.put(at, BlockKind::Log { species, variant });
        }
    }

    impl BlockReader for TestWorld {
        fn block_at(&self, at: BlockPos) -> BlockKind {
            self.blocks.get(&at).copied().unwrap_or(BlockKind::Air)
        }
    }

    fn leaf_variant(species: TreeSpecies) -> u16 {
        match species {
            TreeSpecies::Standard => 60,
            TreeSpecies::NetherFungus => 61,
            TreeSpecies::Mangrove => 62,
            TreeSpecies::Cherry => 63,
        }
    }

    fn put_leaves(world: &mut TestWorld, at: &[BlockPos], species: TreeSpecies, persistent: bool) {
        for pos in at {
            world.put(
                *pos,
                BlockKind::Leaf {
                    species,
                    persistent,
                    variant: leaf_variant(species),
                },
            );
        }
    }

    fn sorted(mut positions: Vec<BlockPos>) -> Vec<BlockPos> {
        positions.sort();
        positions
    }

    fn log_positions(tree: &ScannedTree) -> Vec<BlockPos> {
        sorted(tree.logs.clone())
    }

    /// A five-log oak with seven natural leaves around its crown.
    fn standard_oak() -> (TestWorld, BlockPos) {
        let mut world = TestWorld::default();
        for y in 0..5 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        put_leaves(
            &mut world,
            &[
                pos(1, 4, 0),
                pos(-1, 4, 0),
                pos(0, 4, 1),
                pos(0, 4, -1),
                pos(1, 5, 0),
                pos(-1, 5, 0),
                pos(0, 5, 1),
            ],
            TreeSpecies::Standard,
            false,
        );
        (world, pos(0, 0, 0))
    }

    #[test]
    fn standard_tree_is_felled() {
        let (world, cut) = standard_oak();

        let tree = scan_tree(&world, cut, &TimberConfig::default())
            .expect("a trunk with a natural crown should fall");

        assert_eq!(tree.species, TreeSpecies::Standard);
        assert_eq!(tree.variant, OAK);
        assert_eq!(
            log_positions(&tree),
            [
                pos(0, 0, 0),
                pos(0, 1, 0),
                pos(0, 2, 0),
                pos(0, 3, 0),
                pos(0, 4, 0)
            ]
        );
        assert_eq!(tree.leaves.len(), 7);
        assert_eq!(tree.trunk_2x2, None);
    }

    #[test]
    fn a_cut_that_is_not_a_log_is_rejected() {
        let (world, _) = standard_oak();
        let config = TimberConfig::default();

        assert_eq!(
            scan_tree(&world, pos(0, 4, 1), &config).err(),
            Some(ScanRejectReason::NotALog)
        );
        assert_eq!(
            scan_tree(&world, pos(9, 9, 9), &config).err(),
            Some(ScanRejectReason::NotALog)
        );
    }

    #[test]
    fn a_log_pile_without_leaves_is_rejected() {
        let mut world = TestWorld::default();
        for y in 0..4 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn fewer_than_five_natural_leaves_is_rejected() {
        let mut world = TestWorld::default();
        for y in 0..5 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        put_leaves(
            &mut world,
            &[pos(1, 4, 0), pos(-1, 4, 0), pos(0, 4, 1)],
            TreeSpecies::Standard,
            false,
        );

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn player_placed_leaves_do_not_count() {
        let mut world = TestWorld::default();
        for y in 0..5 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        put_leaves(
            &mut world,
            &[
                pos(1, 4, 0),
                pos(-1, 4, 0),
                pos(0, 4, 1),
                pos(0, 4, -1),
                pos(1, 5, 0),
                pos(-1, 5, 0),
                pos(0, 5, 1),
            ],
            TreeSpecies::Standard,
            true,
        );

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn a_wooden_house_is_rejected() {
        let mut world = TestWorld::default();
        world.log(pos(0, 0, 0), TreeSpecies::Standard, OAK);
        world.log(pos(0, 1, 0), TreeSpecies::Standard, OAK);
        world.log(pos(1, 1, 0), TreeSpecies::Standard, OAK);
        world.log(pos(2, 1, 0), TreeSpecies::Standard, OAK);
        for x in 0..3 {
            for z in -1..=1 {
                world.put(pos(x, 2, z), BlockKind::Solid);
            }
        }

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn leaves_of_another_species_do_not_count() {
        let mut world = TestWorld::default();
        for y in 0..5 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        put_leaves(
            &mut world,
            &[
                pos(1, 4, 0),
                pos(-1, 4, 0),
                pos(0, 4, 1),
                pos(0, 4, -1),
                pos(1, 5, 0),
                pos(-1, 5, 0),
                pos(0, 5, 1),
            ],
            TreeSpecies::Cherry,
            false,
        );

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn logs_below_the_cut_stay_as_a_stump() {
        let (world, _) = standard_oak();
        let cut = pos(0, 2, 0);

        let tree = scan_tree(&world, cut, &TimberConfig::default())
            .expect("the upper trunk and crown should fall");

        assert_eq!(
            log_positions(&tree),
            [pos(0, 2, 0), pos(0, 3, 0), pos(0, 4, 0)]
        );
        assert!(tree.logs.iter().all(|log| log.y >= cut.y));
    }

    #[test]
    fn diagonal_logs_connect_like_vanilla_branches() {
        let mut world = TestWorld::default();
        world.log(pos(0, 0, 0), TreeSpecies::Standard, OAK);
        world.log(pos(0, 1, 0), TreeSpecies::Standard, OAK);
        world.log(pos(0, 2, 0), TreeSpecies::Standard, OAK);
        world.log(pos(1, 3, 1), TreeSpecies::Standard, OAK);
        world.log(pos(2, 4, 2), TreeSpecies::Standard, OAK);
        put_leaves(
            &mut world,
            &[
                pos(1, 2, 0),
                pos(-1, 2, 0),
                pos(0, 2, 1),
                pos(2, 4, 1),
                pos(3, 4, 2),
                pos(2, 5, 2),
            ],
            TreeSpecies::Standard,
            false,
        );

        let tree = scan_tree(&world, pos(0, 0, 0), &TimberConfig::default())
            .expect("branches one block off the trunk should connect");

        assert_eq!(
            log_positions(&tree),
            [
                pos(0, 0, 0),
                pos(0, 1, 0),
                pos(0, 2, 0),
                pos(1, 3, 1),
                pos(2, 4, 2),
            ]
        );
        assert_eq!(tree.leaves.len(), 6);
    }

    #[test]
    fn too_many_logs_is_rejected_at_the_configured_cap() {
        let mut world = TestWorld::default();
        for y in 0..10 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, SPRUCE);
        }
        put_leaves(
            &mut world,
            &[
                pos(1, 8, 0),
                pos(-1, 8, 0),
                pos(0, 8, 1),
                pos(0, 8, -1),
                pos(1, 9, 0),
                pos(-1, 9, 0),
            ],
            TreeSpecies::Standard,
            false,
        );
        let cut = pos(0, 0, 0);

        let too_small = TimberConfig {
            max_logs: 4,
            ..TimberConfig::default()
        };
        assert_eq!(
            scan_tree(&world, cut, &too_small).err(),
            Some(ScanRejectReason::TooLarge)
        );

        let exact = TimberConfig {
            max_logs: 10,
            ..TimberConfig::default()
        };
        assert_eq!(
            scan_tree(&world, cut, &exact)
                .expect("exactly max_logs logs is allowed")
                .logs
                .len(),
            10
        );
    }

    /// A three-layer 2x2 spruce, cut at one corner of its base.
    fn two_by_two_spruce() -> (TestWorld, BlockPos) {
        let mut world = TestWorld::default();
        for y in 0..3 {
            for x in 0..2 {
                for z in 0..2 {
                    world.log(pos(x, y, z), TreeSpecies::Standard, SPRUCE);
                }
            }
        }
        put_leaves(
            &mut world,
            &[
                pos(2, 2, 0),
                pos(2, 2, 1),
                pos(2, 3, 0),
                pos(-1, 2, 0),
                pos(-1, 2, 1),
                pos(0, 2, 2),
                pos(1, 2, 2),
                pos(1, 3, 2),
            ],
            TreeSpecies::Standard,
            false,
        );
        (world, pos(0, 0, 0))
    }

    #[test]
    fn a_two_by_two_trunk_is_detected_at_cutting_height() {
        let (world, cut) = two_by_two_spruce();

        let tree = scan_tree(&world, cut, &TimberConfig::default())
            .expect("the square trunk and crown should be detected");

        let mut base = tree.trunk_2x2.expect("a 2x2 trunk should be reported");
        base.sort();
        assert_eq!(
            base,
            [pos(0, 0, 0), pos(0, 0, 1), pos(1, 0, 0), pos(1, 0, 1)]
        );
        assert_eq!(tree.logs.len(), 12);
    }

    #[test]
    fn a_two_by_two_trunk_needs_all_four_logs_cut() {
        let (world, cut) = two_by_two_spruce();
        let config = TimberConfig::default();

        assert_eq!(
            scan_tree_with_cuts(&world, &[cut], &config).err(),
            Some(ScanRejectReason::IncompleteTwoByTwo)
        );
        assert_eq!(
            scan_tree_with_cuts(&world, &[cut, pos(1, 0, 0), pos(0, 0, 1)], &config).err(),
            Some(ScanRejectReason::IncompleteTwoByTwo)
        );

        let all_four = [pos(0, 0, 0), pos(1, 0, 0), pos(0, 0, 1), pos(1, 0, 1)];
        let tree = scan_tree_with_cuts(&world, &all_four, &config)
            .expect("all four base logs cut should fell the 2x2 tree");
        assert_eq!(tree.logs.len(), 12);
    }

    /// A crimson fungus: three stem logs under a cap of wart blocks.
    fn nether_fungus() -> (TestWorld, BlockPos) {
        let mut world = TestWorld::default();
        for y in 0..3 {
            world.log(pos(0, y, 0), TreeSpecies::NetherFungus, CRIMSON);
        }
        for wart in [
            pos(1, 2, 0),
            pos(-1, 2, 0),
            pos(0, 2, 1),
            pos(0, 2, -1),
            pos(1, 1, 0),
            pos(0, 1, 1),
            pos(0, 3, 0),
        ] {
            world.put(wart, BlockKind::WartBlock { variant: CRIMSON });
        }
        (world, pos(0, 0, 0))
    }

    #[test]
    fn nether_fungus_counts_wart_blocks_as_leaves() {
        let (world, cut) = nether_fungus();

        let tree = scan_tree(&world, cut, &TimberConfig::default())
            .expect("wart blocks are the fungus canopy");

        assert_eq!(tree.species, TreeSpecies::NetherFungus);
        assert_eq!(tree.variant, CRIMSON);
        assert_eq!(tree.leaves.len(), 7);
    }

    #[test]
    fn wart_blocks_do_not_count_for_standard_trees() {
        let mut world = TestWorld::default();
        for y in 0..3 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        for wart in [pos(1, 2, 0), pos(-1, 2, 0), pos(0, 2, 1), pos(0, 2, -1)] {
            world.put(wart, BlockKind::WartBlock { variant: CRIMSON });
        }

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    /// A mangrove trunk split by a root block, with leaves on the upper logs.
    fn mangrove_tree() -> (TestWorld, BlockPos) {
        let mut world = TestWorld::default();
        world.log(pos(0, 0, 0), TreeSpecies::Mangrove, MANGROVE);
        world.put(pos(0, 1, 0), BlockKind::MangroveRoot);
        world.log(pos(0, 2, 0), TreeSpecies::Mangrove, MANGROVE);
        world.log(pos(0, 3, 0), TreeSpecies::Mangrove, MANGROVE);
        put_leaves(
            &mut world,
            &[
                pos(1, 3, 0),
                pos(-1, 3, 0),
                pos(0, 3, 1),
                pos(0, 3, -1),
                pos(1, 2, 0),
                pos(0, 2, 1),
            ],
            TreeSpecies::Mangrove,
            false,
        );
        (world, pos(0, 0, 0))
    }

    #[test]
    fn mangrove_trunk_connects_through_roots() {
        let (world, cut) = mangrove_tree();

        let tree = scan_tree(&world, cut, &TimberConfig::default())
            .expect("roots bridge the mangrove trunk");

        assert_eq!(tree.species, TreeSpecies::Mangrove);
        assert_eq!(
            log_positions(&tree),
            [pos(0, 0, 0), pos(0, 2, 0), pos(0, 3, 0)]
        );
        assert_eq!(tree.leaves.len(), 6);
    }

    #[test]
    fn roots_do_not_connect_standard_trees() {
        let mut world = TestWorld::default();
        world.log(pos(0, 0, 0), TreeSpecies::Standard, OAK);
        world.put(pos(0, 1, 0), BlockKind::MangroveRoot);
        world.log(pos(0, 2, 0), TreeSpecies::Standard, OAK);
        world.log(pos(0, 3, 0), TreeSpecies::Standard, OAK);
        put_leaves(
            &mut world,
            &[
                pos(1, 3, 0),
                pos(-1, 3, 0),
                pos(0, 3, 1),
                pos(0, 3, -1),
                pos(1, 2, 0),
                pos(0, 2, 1),
            ],
            TreeSpecies::Standard,
            false,
        );

        assert_eq!(
            scan_tree(&world, pos(0, 0, 0), &TimberConfig::default()).err(),
            Some(ScanRejectReason::InsufficientNaturalLeaves)
        );
    }

    #[test]
    fn cherry_trunk_branches_and_leaves_are_detected() {
        let mut world = TestWorld::default();
        world.log(pos(0, 0, 0), TreeSpecies::Cherry, CHERRY);
        world.log(pos(0, 1, 0), TreeSpecies::Cherry, CHERRY);
        world.log(pos(0, 2, 0), TreeSpecies::Cherry, CHERRY);
        world.log(pos(1, 2, 1), TreeSpecies::Cherry, CHERRY);
        world.log(pos(2, 3, 2), TreeSpecies::Cherry, CHERRY);
        put_leaves(
            &mut world,
            &[
                pos(1, 2, 0),
                pos(-1, 2, 0),
                pos(0, 2, -1),
                pos(2, 3, 1),
                pos(3, 3, 2),
                pos(2, 4, 2),
                pos(1, 3, 1),
            ],
            TreeSpecies::Cherry,
            false,
        );

        let tree = scan_tree(&world, pos(0, 0, 0), &TimberConfig::default())
            .expect("a cherry with a diagonal branch and natural leaves should fall");

        assert_eq!(tree.species, TreeSpecies::Cherry);
        assert_eq!(
            log_positions(&tree),
            [
                pos(0, 0, 0),
                pos(0, 1, 0),
                pos(0, 2, 0),
                pos(1, 2, 1),
                pos(2, 3, 2),
            ]
        );
        assert_eq!(tree.leaves.len(), 7);
    }

    #[test]
    fn leaves_keep_their_own_variants() {
        let mut world = TestWorld::default();
        for y in 0..5 {
            world.log(pos(0, y, 0), TreeSpecies::Standard, OAK);
        }
        let leaves = [
            (pos(1, 4, 0), 101u16),
            (pos(-1, 4, 0), 102),
            (pos(0, 4, 1), 103),
            (pos(0, 4, -1), 104),
            (pos(1, 5, 0), 105),
        ];
        for (at, variant) in leaves {
            world.put(
                at,
                BlockKind::Leaf {
                    species: TreeSpecies::Standard,
                    persistent: false,
                    variant,
                },
            );
        }

        let tree = scan_tree(&world, pos(0, 0, 0), &TimberConfig::default())
            .expect("five natural leaves are enough");

        let mut actual: Vec<(BlockPos, u16)> = tree
            .leaves
            .iter()
            .map(|leaf| (leaf.pos, leaf.variant))
            .collect();
        actual.sort();
        let mut expected = leaves.to_vec();
        expected.sort();
        assert_eq!(actual, expected);
    }
}

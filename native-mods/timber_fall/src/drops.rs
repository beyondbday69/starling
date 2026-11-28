//! Where a felled tree's blocks turn into items, and the mid-fall recovery plan.
//!
//! Block ids stay opaque `u16` values, as in the scanner: the adapter maps a log or
//! leaf variant to its item and evaluates leaf loot tables per leaf entry.

use crate::config::TimberConfig;
use crate::falling::{FallingBlock, FallingTree};
use crate::impact::rotate_offset;
use crate::scanner::BlockPos;

/// Logs landing within this horizontal distance of a stack's anchor merge into it.
/// Height is ignored: the tree lies on the ground, so vertical noise is not a gap.
pub const DROP_MERGE_RADIUS: f64 = 2.0;

/// One item stack to spawn. A leaf entry stands for one leaf block's loot-table roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropStack {
    pub item_variant: u16,
    pub is_leaf: bool,
    pub count: u32,
    pub landing_pos: [f64; 3],
}

/// Where to plant a sapling after the tree is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplantPlan {
    pub pos: BlockPos,
    /// The tree's log variant; the adapter maps it to the matching sapling.
    pub sapling_variant: u16,
}

/// The items and replant for one felled tree.
#[derive(Clone, Debug, PartialEq)]
pub struct DropPlan {
    pub stacks: Vec<DropStack>,
    pub replant: Option<ReplantPlan>,
}

/// Drops for a tree that finished falling: logs merge into stacks near where each
/// section lands, and every leaf block gets its own entry for its loot roll.
///
/// `max_stack_size` caps a merged stack; a value below 1 is treated as 1.
pub fn plan_breakup_drops(
    tree: &FallingTree,
    config: &TimberConfig,
    max_stack_size: u32,
) -> DropPlan {
    collect_drops(tree, config, max_stack_size, |block| {
        landing_position(tree, block)
    })
}

/// Recovery for a quit or chunk unload mid-fall: every remaining block drops at the
/// hinge, so nothing is lost or duplicated when the animation never finishes.
pub fn plan_midfall_drops(
    tree: &FallingTree,
    config: &TimberConfig,
    max_stack_size: u32,
) -> DropPlan {
    collect_drops(tree, config, max_stack_size, |_block| tree.hinge_center())
}

fn collect_drops(
    tree: &FallingTree,
    config: &TimberConfig,
    max_stack_size: u32,
    landing_of: impl Fn(&FallingBlock) -> [f64; 3],
) -> DropPlan {
    let max_stack_size = max_stack_size.max(1);
    let mut stacks = Vec::new();

    for block in &tree.blocks {
        let landing = landing_of(block);
        if block.is_leaf {
            stacks.push(DropStack {
                item_variant: block.variant,
                is_leaf: true,
                count: 1,
                landing_pos: landing,
            });
        } else {
            push_log_stack(&mut stacks, block, landing, max_stack_size);
        }
    }

    DropPlan {
        stacks,
        replant: replant_plan(tree, config),
    }
}

/// The world position of a block's centre once the tree is rotated to `angle_rad`.
fn landing_position(tree: &FallingTree, block: &FallingBlock) -> [f64; 3] {
    let hinge = tree.hinge_center();
    let rotated = rotate_offset(tree.block_center(block), tree.direction, tree.angle_rad());
    [
        hinge[0] + f64::from(rotated[0]),
        hinge[1] + f64::from(rotated[1]),
        hinge[2] + f64::from(rotated[2]),
    ]
}

/// Merges into the first nearby stack of the same variant that still has room, or
/// starts a new stack anchored at the block's own landing position.
fn push_log_stack(
    stacks: &mut Vec<DropStack>,
    block: &FallingBlock,
    landing: [f64; 3],
    max_stack_size: u32,
) {
    let nearby = stacks.iter_mut().find(|stack| {
        !stack.is_leaf
            && stack.item_variant == block.variant
            && stack.count < max_stack_size
            && distance(stack.landing_pos, landing) <= DROP_MERGE_RADIUS
    });

    match nearby {
        Some(stack) => stack.count += 1,
        None => stacks.push(DropStack {
            item_variant: block.variant,
            is_leaf: false,
            count: 1,
            landing_pos: landing,
        }),
    }
}

fn replant_plan(tree: &FallingTree, config: &TimberConfig) -> Option<ReplantPlan> {
    config.auto_replant_sapling.then(|| ReplantPlan {
        pos: tree.hinge,
        sapling_variant: tree
            .blocks
            .iter()
            .find(|block| !block.is_leaf)
            .map_or(0, |block| block.variant),
    })
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dz = a[2] - b[2];
    (dx * dx + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::falling::{FallDirection, FallPhase};

    const OAK: u16 = 10;
    const SPRUCE: u16 = 11;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    fn block(hinge: BlockPos, at: BlockPos, is_leaf: bool, variant: u16) -> FallingBlock {
        FallingBlock::relative_to(hinge, at, is_leaf, variant)
    }

    /// A tree that has finished falling and rests at `angle_rad`.
    fn breakup_tree(blocks: Vec<FallingBlock>, angle_rad: f32) -> FallingTree {
        let mut tree = FallingTree::new(
            pos(0, 0, 0),
            FallDirection { dx: 1.0, dz: 0.0 },
            5.0,
            1.5,
            blocks,
        );
        tree.phase = FallPhase::BreakUp { elapsed: 0.0 };
        tree.rest_angle_rad = angle_rad;
        tree
    }

    fn log_stacks(plan: &DropPlan) -> Vec<DropStack> {
        plan.stacks
            .iter()
            .copied()
            .filter(|stack| !stack.is_leaf)
            .collect()
    }

    fn leaf_stacks(plan: &DropPlan) -> Vec<DropStack> {
        plan.stacks
            .iter()
            .copied()
            .filter(|stack| stack.is_leaf)
            .collect()
    }

    fn logs_at_horizontal_positions(tree: &FallingTree) -> Vec<f64> {
        tree.blocks
            .iter()
            .filter(|block| !block.is_leaf)
            .map(|block| landing_position(tree, block)[0])
            .collect()
    }

    #[test]
    fn nearby_logs_merge_while_every_leaf_stays_its_own_entry() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(
            vec![
                block(hinge, pos(0, 0, 0), false, OAK),
                block(hinge, pos(0, 1, 0), false, OAK),
                block(hinge, pos(0, 2, 0), false, OAK),
                block(hinge, pos(0, 3, 0), true, OAK),
                block(hinge, pos(1, 3, 0), true, OAK),
            ],
            FRAC_PI_2,
        );

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 64);

        let logs = log_stacks(&plan);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].count, 3);
        assert_eq!(logs[0].item_variant, OAK);

        let leaves = leaf_stacks(&plan);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.iter().all(|stack| stack.count == 1));
        assert_eq!(plan.replant, None);
    }

    #[test]
    fn drops_land_near_the_rotated_block_positions() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(vec![block(hinge, pos(0, 0, 0), false, OAK)], FRAC_PI_2);

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 64);
        let landing = plan.stacks[0].landing_pos;

        // The base log's centre starts half a block above the hinge, so at 90 degrees
        // it lies one block along +X, on the hinge's height.
        assert!((landing[0] - 1.0).abs() < 1e-5, "x was {}", landing[0]);
        assert!(landing[1].abs() < 1e-5, "y was {}", landing[1]);
        assert!((landing[2] - 0.5).abs() < 1e-5, "z was {}", landing[2]);
    }

    #[test]
    fn a_merged_stack_never_exceeds_max_stack_size() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(
            vec![
                block(hinge, pos(0, 0, 0), false, OAK),
                block(hinge, pos(0, 1, 0), false, OAK),
                block(hinge, pos(0, 2, 0), false, OAK),
            ],
            FRAC_PI_2,
        );

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 2);
        let mut counts: Vec<u32> = log_stacks(&plan).iter().map(|stack| stack.count).collect();
        counts.sort_unstable();

        assert_eq!(counts, [1, 2]);
        assert!(log_stacks(&plan).iter().all(|stack| stack.count <= 2));
    }

    #[test]
    fn different_log_variants_never_merge() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(
            vec![
                block(hinge, pos(0, 0, 0), false, OAK),
                block(hinge, pos(0, 1, 0), false, SPRUCE),
            ],
            FRAC_PI_2,
        );

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 64);
        let logs = log_stacks(&plan);

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].count, 1);
        assert_eq!(logs[1].count, 1);
        assert_ne!(logs[0].item_variant, logs[1].item_variant);
    }

    #[test]
    fn logs_that_land_farther_than_the_merge_radius_stay_separate() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(
            vec![
                block(hinge, pos(0, 0, 0), false, OAK),
                block(hinge, pos(0, 4, 0), false, OAK),
            ],
            FRAC_PI_2,
        );

        let positions = logs_at_horizontal_positions(&tree);
        assert!((positions[1] - positions[0]) > DROP_MERGE_RADIUS);

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 64);
        assert_eq!(log_stacks(&plan).len(), 2);
    }

    #[test]
    fn replant_is_planned_only_when_the_option_is_on() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(vec![block(hinge, pos(0, 0, 0), false, OAK)], 0.0);

        let off = plan_breakup_drops(&tree, &TimberConfig::default(), 64);
        assert_eq!(off.replant, None);

        let on = plan_breakup_drops(
            &tree,
            &TimberConfig {
                auto_replant_sapling: true,
                ..TimberConfig::default()
            },
            64,
        );
        assert_eq!(
            on.replant,
            Some(ReplantPlan {
                pos: hinge,
                sapling_variant: OAK,
            })
        );
    }

    #[test]
    fn midfall_recovery_drops_every_block_without_loss_or_duplication() {
        let hinge = pos(0, 0, 0);
        let mut blocks: Vec<FallingBlock> = (0..5)
            .map(|y| block(hinge, pos(0, y, 0), false, OAK))
            .collect();
        blocks.push(block(hinge, pos(0, 5, 0), false, SPRUCE));
        blocks.push(block(hinge, pos(0, 6, 0), true, OAK));
        blocks.push(block(hinge, pos(1, 6, 0), true, OAK));

        let mut tree = breakup_tree(blocks, 0.0);
        tree.phase = FallPhase::Falling {
            angle_rad: 0.4,
            angular_vel: 1.2,
        };

        let config = TimberConfig {
            auto_replant_sapling: true,
            ..TimberConfig::default()
        };
        let plan = plan_midfall_drops(&tree, &config, 2);

        let logs = log_stacks(&plan);
        assert!(logs.iter().all(|stack| stack.count <= 2));
        assert_eq!(logs.iter().map(|stack| stack.count).sum::<u32>(), 6);
        let oak: u32 = logs
            .iter()
            .filter(|stack| stack.item_variant == OAK)
            .map(|stack| stack.count)
            .sum();
        let spruce: u32 = logs
            .iter()
            .filter(|stack| stack.item_variant == SPRUCE)
            .map(|stack| stack.count)
            .sum();
        assert_eq!(oak, 5);
        assert_eq!(spruce, 1);

        let leaves = leaf_stacks(&plan);
        assert_eq!(leaves.len(), 2);
        assert!(leaves.iter().all(|stack| stack.count == 1));

        let hinge_center = tree.hinge_center();
        assert!(
            plan.stacks
                .iter()
                .all(|stack| stack.landing_pos == hinge_center)
        );
        assert_eq!(
            plan.replant,
            Some(ReplantPlan {
                pos: hinge,
                sapling_variant: OAK,
            })
        );
    }

    #[test]
    fn leaf_drops_keep_the_leafs_own_variant() {
        let hinge = pos(0, 0, 0);
        let tree = breakup_tree(
            vec![
                block(hinge, pos(0, 0, 0), false, OAK),
                block(hinge, pos(0, 1, 0), true, 77),
                block(hinge, pos(1, 1, 0), true, 78),
            ],
            FRAC_PI_2,
        );

        let plan = plan_breakup_drops(&tree, &TimberConfig::default(), 64);
        let leaves = leaf_stacks(&plan);

        assert_eq!(leaves.len(), 2);
        assert!(leaves.iter().any(|stack| stack.item_variant == 77));
        assert!(leaves.iter().any(|stack| stack.item_variant == 78));
    }
}

//! Whether a cut may fell the whole tree, and what it costs the axe and the player.

use crate::config::TimberConfig;
use crate::scanner::{ScanRejectReason, ScanResult};

/// Vanilla block-break exhaustion per block.
pub const BLOCK_BREAK_EXHAUSTION: f32 = 0.005;
/// The chop-time multiplier is capped here, so a huge tree is never slower than 5x.
pub const MAX_CHOP_TIME_MULTIPLIER: f32 = 5.0;

/// The tool that made the cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolState {
    pub is_axe: bool,
    pub remaining_durability: u32,
    pub unbreaking_level: u8,
    pub creative_mode: bool,
}

/// Everything the decision needs besides the scanned tree and the config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChopContext {
    pub sneaking: bool,
    pub has_permission: bool,
    pub tool: ToolState,
}

/// Why only the cut log breaks, as vanilla would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SingleBlockReason {
    Sneaking,
    AxeRequired,
    NoPermission,
    InsufficientDurability,
    NotATree(ScanRejectReason),
}

/// What the cut should do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChopDecision {
    FellTree {
        tool_damage: u32,
        hunger_exhaustion: f32,
    },
    VanillaSingleBlock {
        reason: SingleBlockReason,
    },
}

/// How much longer the cut takes: `1 + logs / 10`, capped at 5x.
pub fn chop_time_multiplier(log_count: usize) -> f32 {
    (1.0 + log_count as f32 / 10.0).min(MAX_CHOP_TIME_MULTIPLIER)
}

/// The reciprocal of [`chop_time_multiplier`]: how fast the cut progresses.
pub fn chop_speed_multiplier(log_count: usize) -> f32 {
    1.0 / chop_time_multiplier(log_count)
}

/// Vanilla block-break exhaustion for every log.
pub fn chop_exhaustion(log_count: usize) -> f32 {
    BLOCK_BREAK_EXHAUSTION * log_count as f32
}

/// Durability the axe loses for `log_count` logs.
///
/// Each point is ignored with probability `unbreaking / (unbreaking + 1)`, decided by
/// the next value of `rng_rolls` (a uniform value in `[0, 1)`); a roll below that
/// probability means the point is ignored. Missing rolls are treated as damage, so a
/// caller without a random source errs towards wearing the axe.
pub fn axe_wear(log_count: usize, unbreaking_level: u8, rng_rolls: &[f32]) -> u32 {
    if unbreaking_level == 0 {
        return log_count as u32;
    }
    let ignore_chance = f32::from(unbreaking_level) / (f32::from(unbreaking_level) + 1.0);
    rng_rolls
        .iter()
        .take(log_count)
        .filter(|roll| **roll >= ignore_chance)
        .count() as u32
        + log_count.saturating_sub(rng_rolls.len()) as u32
}

/// The expected number of logs `remaining_durability` covers, with Unbreaking.
///
/// Every log costs `1 / (unbreaking + 1)` points on average, so this is the expected
/// budget. [`decide_chop`] uses the concrete rolls instead, because a run of bad luck
/// can still exhaust the axe early.
pub fn max_coverable_logs(remaining_durability: u32, unbreaking_level: u8) -> u32 {
    remaining_durability.saturating_mul(u32::from(unbreaking_level) + 1)
}

/// Decides between felling the scanned tree and a plain vanilla single-block break.
///
/// Checks run in this order: permission, sneak, axe, tree validity, durability. The
/// durability check uses the wear from `wear_rolls`; creative mode neither wears the
/// axe nor needs durability.
pub fn decide_chop(
    ctx: &ChopContext,
    scan: &ScanResult,
    config: &TimberConfig,
    wear_rolls: &[f32],
) -> ChopDecision {
    if !ctx.has_permission {
        return ChopDecision::VanillaSingleBlock {
            reason: SingleBlockReason::NoPermission,
        };
    }
    if config.sneak_disables_felling && ctx.sneaking {
        return ChopDecision::VanillaSingleBlock {
            reason: SingleBlockReason::Sneaking,
        };
    }
    if config.require_axe && !ctx.tool.is_axe {
        return ChopDecision::VanillaSingleBlock {
            reason: SingleBlockReason::AxeRequired,
        };
    }

    let log_count = match scan {
        Ok(tree) => tree.logs.len(),
        Err(reason) => {
            return ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::NotATree(*reason),
            };
        }
    };

    let tool_damage = if ctx.tool.creative_mode {
        0
    } else {
        axe_wear(log_count, ctx.tool.unbreaking_level, wear_rolls)
    };
    if !ctx.tool.creative_mode && tool_damage > ctx.tool.remaining_durability {
        return ChopDecision::VanillaSingleBlock {
            reason: SingleBlockReason::InsufficientDurability,
        };
    }

    ChopDecision::FellTree {
        tool_damage,
        hunger_exhaustion: chop_exhaustion(log_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{BlockPos, ScannedTree, TreeSpecies};

    const OAK: u16 = 10;

    fn approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }

    fn axe(durability: u32, unbreaking_level: u8) -> ToolState {
        ToolState {
            is_axe: true,
            remaining_durability: durability,
            unbreaking_level,
            creative_mode: false,
        }
    }

    fn context(tool: ToolState) -> ChopContext {
        ChopContext {
            sneaking: false,
            has_permission: true,
            tool,
        }
    }

    fn scanned(log_count: usize) -> ScanResult {
        let logs = (0..log_count)
            .map(|y| BlockPos::new(0, y as i32, 0))
            .collect();
        Ok(ScannedTree {
            species: TreeSpecies::Standard,
            variant: OAK,
            logs,
            leaves: Vec::new(),
            trunk_2x2: None,
        })
    }

    fn fell(decision: ChopDecision) -> (u32, f32) {
        match decision {
            ChopDecision::FellTree {
                tool_damage,
                hunger_exhaustion,
            } => (tool_damage, hunger_exhaustion),
            other => panic!("expected the tree to fall, got {other:?}"),
        }
    }

    #[test]
    fn time_and_speed_multipliers_match_the_concept_formula() {
        approx(chop_time_multiplier(0), 1.0);
        approx(chop_time_multiplier(10), 2.0);
        approx(chop_time_multiplier(40), 5.0);
        approx(chop_time_multiplier(100), 5.0);

        approx(chop_speed_multiplier(0), 1.0);
        approx(chop_speed_multiplier(10), 0.5);
        approx(chop_speed_multiplier(40), 0.2);
        approx(chop_speed_multiplier(100), 0.2);
    }

    #[test]
    fn exhaustion_is_vanilla_block_break_exhaustion_per_log() {
        approx(chop_exhaustion(0), 0.0);
        approx(chop_exhaustion(1), 0.005);
        approx(chop_exhaustion(10), 0.05);
        approx(chop_exhaustion(5), 0.025);
    }

    #[test]
    fn sneaking_falls_back_when_the_option_is_on() {
        let config = TimberConfig::default();
        let mut ctx = context(axe(100, 0));
        ctx.sneaking = true;

        assert_eq!(
            decide_chop(&ctx, &scanned(5), &config, &[]),
            ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::Sneaking
            }
        );
    }

    #[test]
    fn sneaking_allows_felling_when_the_option_is_off() {
        let config = TimberConfig {
            sneak_disables_felling: false,
            ..TimberConfig::default()
        };
        let mut ctx = context(axe(100, 0));
        ctx.sneaking = true;

        fell(decide_chop(&ctx, &scanned(5), &config, &[]));
    }

    #[test]
    fn a_missing_axe_falls_back_when_the_option_is_on() {
        let config = TimberConfig::default();
        let mut tool = axe(100, 0);
        tool.is_axe = false;

        assert_eq!(
            decide_chop(&context(tool), &scanned(5), &config, &[]),
            ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::AxeRequired
            }
        );
    }

    #[test]
    fn a_missing_axe_allows_felling_when_the_option_is_off() {
        let config = TimberConfig {
            require_axe: false,
            ..TimberConfig::default()
        };
        let mut tool = axe(100, 0);
        tool.is_axe = false;

        fell(decide_chop(&context(tool), &scanned(5), &config, &[]));
    }

    #[test]
    fn a_player_without_permission_only_breaks_one_block() {
        let mut ctx = context(axe(100, 0));
        ctx.has_permission = false;

        assert_eq!(
            decide_chop(&ctx, &scanned(5), &TimberConfig::default(), &[]),
            ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::NoPermission
            }
        );
    }

    #[test]
    fn a_rejected_scan_only_breaks_one_block_with_its_reason() {
        let config = TimberConfig::default();
        let scan = Err(ScanRejectReason::TooLarge);

        assert_eq!(
            decide_chop(&context(axe(100, 0)), &scan, &config, &[]),
            ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::NotATree(ScanRejectReason::TooLarge)
            }
        );
    }

    #[test]
    fn insufficient_durability_falls_back_to_one_block() {
        let config = TimberConfig::default();
        let ctx = context(axe(4, 0));

        assert_eq!(
            decide_chop(&ctx, &scanned(5), &config, &[]),
            ChopDecision::VanillaSingleBlock {
                reason: SingleBlockReason::InsufficientDurability
            }
        );
    }

    #[test]
    fn durability_that_exactly_covers_the_tree_fells_it() {
        let config = TimberConfig::default();
        let ctx = context(axe(5, 0));

        let (damage, exhaustion) = fell(decide_chop(&ctx, &scanned(5), &config, &[]));

        assert_eq!(damage, 5);
        approx(exhaustion, 0.025);
    }

    #[test]
    fn unbreaking_reduces_wear_and_can_save_an_almost_broken_axe() {
        let rolls = [0.1, 0.9, 0.5, 0.8, 0.2];

        assert_eq!(axe_wear(5, 3, &rolls), 2);

        let config = TimberConfig::default();
        let ctx = context(axe(2, 3));
        let (damage, _) = fell(decide_chop(&ctx, &scanned(5), &config, &rolls));
        assert_eq!(damage, 2);
    }

    #[test]
    fn missing_rolls_are_treated_as_damage() {
        assert_eq!(axe_wear(4, 3, &[0.0]), 3);
        assert_eq!(axe_wear(4, 0, &[]), 4);
    }

    #[test]
    fn creative_mode_ignores_durability_and_wear() {
        let config = TimberConfig::default();
        let mut tool = axe(0, 0);
        tool.creative_mode = true;

        let (damage, _) = fell(decide_chop(&context(tool), &scanned(5), &config, &[]));

        assert_eq!(damage, 0);
    }

    #[test]
    fn max_coverable_logs_counts_the_expected_budget() {
        assert_eq!(max_coverable_logs(3, 0), 3);
        assert_eq!(max_coverable_logs(3, 3), 12);
        assert_eq!(max_coverable_logs(0, 9), 0);
    }
}

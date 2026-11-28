//! The fall itself: a uniform pole hinged at the stump.
//!
//! When the cut log is gone from under the tree (`drop_height > 0`), the tree first
//! stands still, then drops straight down onto the stump under gravity, and only then
//! tips over around the stump top.
//!
//! The tree rotates around the cut as one rigid body. `fall_speed` scales *time*
//! (simulated time advances `fall_speed` times faster than real time), which is what
//! the concept's table means by "1.5x speed": it matches the realistic pole times
//! divided by 1.5. Scaling the angular acceleration by `fall_speed` instead would
//! only shorten the fall by `sqrt(fall_speed)` and miss that table.

use std::f32::consts::PI;

use crate::config::TimberConfig;
use crate::impact::{CollisionGrid, hits_solid};
use crate::scanner::BlockPos;

/// How long the warning creak lasts before the tree starts to fall.
pub const WARNING_DURATION_SECS: f32 = 0.5;
/// The lean at the start of the warning, in degrees.
pub const WARNING_START_DEG: f32 = 1.0;
/// The lean the warning ends on; a tree that drops first stands still instead.
pub const WARNING_END_DEG: f32 = 3.0;
/// Gravity, `g` in the pole equation.
pub const GRAVITY: f32 = 9.8;
/// How far the tree springs back after the first impact, in degrees.
pub const BOUNCE_DEG: f32 = 3.0;
/// Time from the first impact to the break-up, in seconds.
pub const BREAKUP_DELAY_SECS: f32 = 1.0;
/// The tree is stopped at this angle even when nothing solid is in the way.
pub const MAX_FALL_ANGLE_RAD: f32 = 120.0 * PI / 180.0;

const START_ANGLE_RAD: f32 = WARNING_END_DEG * PI / 180.0;
/// Starting lean for a tree that has just landed on the stump.
const DROP_START_ANGLE_RAD: f32 = 1.5 * PI / 180.0;
const BOUNCE_RAD: f32 = BOUNCE_DEG * PI / 180.0;
const BOUNCE_DURATION_SECS: f32 = 0.2;
const SUBSTEP_SECS: f32 = 1.0 / 120.0;
const MAX_STEP_SECS: f32 = 0.25;

/// Horizontal unit vector the tree tips towards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallDirection {
    pub dx: f32,
    pub dz: f32,
}

impl FallDirection {
    /// The deterministic fallback when the player stands exactly on the hinge.
    pub const DEFAULT: Self = Self { dx: 1.0, dz: 0.0 };

    /// The horizontal direction from the player to the hinge, normalized; a player
    /// standing on the hinge gets [`FallDirection::DEFAULT`].
    pub fn away_from_player(player_x: f32, player_z: f32, hinge: BlockPos) -> Self {
        let dx = hinge.x as f32 + 0.5 - player_x;
        let dz = hinge.z as f32 + 0.5 - player_z;
        let length = (dx * dx + dz * dz).sqrt();
        if !length.is_finite() || length < 1e-6 {
            Self::DEFAULT
        } else {
            Self {
                dx: dx / length,
                dz: dz / length,
            }
        }
    }
}

/// Where the tree is in the fall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FallPhase {
    Warning {
        elapsed: f32,
    },
    /// The tree falls straight down onto the stump after the cut log is gone.
    Dropping {
        elapsed: f32,
    },
    Falling {
        angle_rad: f32,
        angular_vel: f32,
    },
    Impact {
        impact_angle_rad: f32,
        impact_speed: f32,
        bounce_elapsed: f32,
    },
    BreakUp {
        elapsed: f32,
    },
}

/// What happened during one [`FallingTree::step`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepEvents {
    pub entered_falling: bool,
    /// Speed (blocks per second) at which the tree landed on the stump, once.
    pub landed: Option<f32>,
    /// Angular speed at the first impact, in radians per second.
    pub impacted: Option<f32>,
    pub ready_to_break_up: bool,
}

/// One block of the falling structure.
///
/// `offset` is the block's minimum corner relative to the hinge point, so at angle 0
/// `block_transform_3x4(hinge, dir, 0.0, offset)` places the block's model origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallingBlock {
    pub offset: [f32; 3],
    pub pos: BlockPos,
    pub is_leaf: bool,
    pub variant: u16,
    /// The cut log: gone from the world, so it is neither drawn nor part of the collision,
    /// but it still counts for the drops.
    pub vanished: bool,
}

impl FallingBlock {
    /// Builds a block of the tree hinged at `hinge`.
    pub fn relative_to(hinge: BlockPos, pos: BlockPos, is_leaf: bool, variant: u16) -> Self {
        Self {
            offset: [
                pos.x as f32 - (hinge.x as f32 + 0.5),
                pos.y as f32 - hinge.y as f32,
                pos.z as f32 - (hinge.z as f32 + 0.5),
            ],
            pos,
            is_leaf,
            variant,
            vanished: false,
        }
    }

    /// The block's centre relative to the hinge point.
    pub fn center_offset(&self) -> [f32; 3] {
        [
            self.offset[0] + 0.5,
            self.offset[1] + 0.5,
            self.offset[2] + 0.5,
        ]
    }
}

/// A felled tree and its fall.
#[derive(Clone, Debug)]
pub struct FallingTree {
    pub hinge: BlockPos,
    pub direction: FallDirection,
    pub trunk_height: f32,
    pub blocks: Vec<FallingBlock>,
    pub phase: FallPhase,
    pub fall_speed: f32,
    /// The angle the tree rests at after impact; `BreakUp` has no angle of its own.
    pub rest_angle_rad: f32,
    /// How far the tree drops onto the stump before tipping; 0 tips it straight away.
    pub drop_height: f32,
}

impl FallingTree {
    pub fn new(
        hinge: BlockPos,
        direction: FallDirection,
        trunk_height: f32,
        fall_speed: f32,
        blocks: Vec<FallingBlock>,
    ) -> Self {
        Self {
            hinge,
            direction,
            trunk_height,
            blocks,
            phase: FallPhase::Warning { elapsed: 0.0 },
            fall_speed,
            rest_angle_rad: 0.0,
            drop_height: 0.0,
        }
    }

    /// Makes the tree drop `height` blocks onto the stump before it tips over.
    pub fn with_drop(mut self, height: f32) -> Self {
        self.drop_height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        self
    }

    /// Vertical shift of the tree's blocks relative to the hinge: 0 until the drop
    /// starts, then down by `drop_height` as it lands.
    pub fn base_shift(&self) -> f32 {
        if self.drop_height <= 0.0 {
            return 0.0;
        }
        match self.phase {
            FallPhase::Warning { .. } => 0.0,
            FallPhase::Dropping { elapsed } => {
                -(0.5 * GRAVITY * elapsed * elapsed).min(self.drop_height)
            }
            _ => -self.drop_height,
        }
    }

    /// The block's centre relative to the hinge, with the drop applied.
    pub fn block_center(&self, block: &FallingBlock) -> [f32; 3] {
        let c = block.center_offset();
        if block.vanished {
            c
        } else {
            [c[0], c[1] + self.base_shift(), c[2]]
        }
    }

    /// The world position the tree rotates around: the centre of the stump's top face.
    pub fn hinge_center(&self) -> [f64; 3] {
        [
            self.hinge.x as f64 + 0.5,
            self.hinge.y as f64,
            self.hinge.z as f64 + 0.5,
        ]
    }

    /// The angle from vertical currently shown for the tree.
    pub fn angle_rad(&self) -> f32 {
        match self.phase {
            FallPhase::Warning { .. } if self.drop_height > 0.0 => 0.0,
            FallPhase::Dropping { .. } => 0.0,
            FallPhase::Warning { elapsed } => {
                let progress = (elapsed / WARNING_DURATION_SECS).clamp(0.0, 1.0);
                let degrees = WARNING_START_DEG + (WARNING_END_DEG - WARNING_START_DEG) * progress;
                degrees * PI / 180.0
            }
            FallPhase::Falling { angle_rad, .. } => angle_rad,
            FallPhase::Impact {
                impact_angle_rad,
                bounce_elapsed,
                ..
            } => impact_angle_rad - bounce_offset(bounce_elapsed),
            FallPhase::BreakUp { .. } => self.rest_angle_rad,
        }
    }

    /// Advances the state machine by `dt` seconds of real time.
    ///
    /// Collision is checked against the trunk only: leaves pass through blocks, as
    /// the concept's "solid blocks stop the trunk" rule says. The grid is expected
    /// to leave out the felled tree's own blocks, which are removed with the cut.
    pub fn step(&mut self, dt: f32, grid: &dyn CollisionGrid) -> StepEvents {
        let mut events = StepEvents::default();
        if !dt.is_finite() || dt <= 0.0 {
            return events;
        }

        let mut remaining = dt.min(MAX_STEP_SECS);
        while remaining > f32::EPSILON {
            let substep = remaining.min(SUBSTEP_SECS);
            remaining -= substep;
            self.advance(substep, grid, &mut events);
        }
        events
    }

    /// All blocks ordered from the base to the tip, for the break-up burst.
    pub fn break_up_order(&self) -> Vec<FallingBlock> {
        let mut blocks = self.blocks.clone();
        blocks.sort_by(|a, b| {
            a.offset[1].total_cmp(&b.offset[1]).then_with(|| {
                horizontal_distance(a.offset).total_cmp(&horizontal_distance(b.offset))
            })
        });
        blocks
    }

    fn advance(&mut self, dt: f32, grid: &dyn CollisionGrid, events: &mut StepEvents) {
        match self.phase {
            FallPhase::Warning { elapsed } => {
                let total = elapsed + dt;
                if total < WARNING_DURATION_SECS {
                    self.phase = FallPhase::Warning { elapsed: total };
                    return;
                }
                let overflow = total - WARNING_DURATION_SECS;
                if self.drop_height > 0.0 {
                    self.phase = FallPhase::Dropping { elapsed: 0.0 };
                } else {
                    self.phase = FallPhase::Falling {
                        angle_rad: START_ANGLE_RAD,
                        angular_vel: 0.0,
                    };
                    events.entered_falling = true;
                }
                if overflow > 0.0 {
                    self.advance(overflow, grid, events);
                }
            }
            FallPhase::Dropping { elapsed } => {
                let total = elapsed + dt * self.time_scale();
                if 0.5 * GRAVITY * total * total >= self.drop_height {
                    events.landed = Some(GRAVITY * total);
                    events.entered_falling = true;
                    self.phase = FallPhase::Falling {
                        angle_rad: DROP_START_ANGLE_RAD,
                        angular_vel: 0.0,
                    };
                } else {
                    self.phase = FallPhase::Dropping { elapsed: total };
                }
            }
            FallPhase::Falling {
                angle_rad,
                angular_vel,
            } => {
                let step = dt * self.time_scale();
                let length = self.fall_length();
                let alpha = 3.0 * GRAVITY / (2.0 * length) * angle_rad.sin();
                let angular_vel = angular_vel + alpha * step;
                let mut angle_rad = angle_rad + angular_vel * step;

                if angle_rad >= MAX_FALL_ANGLE_RAD {
                    angle_rad = MAX_FALL_ANGLE_RAD;
                    self.enter_impact(angle_rad, angular_vel, events);
                    return;
                }
                if hits_solid(self, angle_rad, grid) {
                    self.enter_impact(angle_rad, angular_vel, events);
                    return;
                }
                self.phase = FallPhase::Falling {
                    angle_rad,
                    angular_vel,
                };
            }
            FallPhase::Impact {
                impact_angle_rad,
                impact_speed,
                bounce_elapsed,
            } => {
                let total = bounce_elapsed + dt;
                if total >= BREAKUP_DELAY_SECS {
                    self.phase = FallPhase::BreakUp {
                        elapsed: total - BREAKUP_DELAY_SECS,
                    };
                    events.ready_to_break_up = true;
                } else {
                    self.phase = FallPhase::Impact {
                        impact_angle_rad,
                        impact_speed,
                        bounce_elapsed: total,
                    };
                }
            }
            FallPhase::BreakUp { elapsed } => {
                self.phase = FallPhase::BreakUp {
                    elapsed: elapsed + dt,
                };
            }
        }
    }

    fn enter_impact(&mut self, angle_rad: f32, angular_vel: f32, events: &mut StepEvents) {
        let impact_speed = angular_vel.max(0.0);
        self.rest_angle_rad = angle_rad;
        self.phase = FallPhase::Impact {
            impact_angle_rad: angle_rad,
            impact_speed,
            bounce_elapsed: 0.0,
        };
        events.impacted = Some(impact_speed);
    }

    fn fall_length(&self) -> f32 {
        self.trunk_height.max(1.0)
    }

    fn time_scale(&self) -> f32 {
        if self.fall_speed.is_finite() && self.fall_speed > 0.0 {
            self.fall_speed
        } else {
            TimberConfig::DEFAULT_FALL_SPEED
        }
    }
}

/// The height of the single rebound after impact: 0 at rest, `BOUNCE_DEG` at its peak.
fn bounce_offset(elapsed: f32) -> f32 {
    if !(0.0..BOUNCE_DURATION_SECS).contains(&elapsed) {
        return 0.0;
    }
    BOUNCE_RAD * (PI * elapsed / BOUNCE_DURATION_SECS).sin()
}

fn horizontal_distance(offset: [f32; 3]) -> f32 {
    (offset[0] * offset[0] + offset[2] * offset[2]).sqrt()
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const OAK: u16 = 10;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    /// Nothing is solid: grass, flowers and air everywhere.
    struct NoObstacles;

    impl CollisionGrid for NoObstacles {
        fn is_solid_obstacle(&self, _pos: BlockPos) -> bool {
            false
        }
    }

    /// Flat ground whose surface is the hinge height.
    struct FlatGround;

    impl CollisionGrid for FlatGround {
        fn is_solid_obstacle(&self, at: BlockPos) -> bool {
            at.y < 10
        }
    }

    /// Ground three blocks below the hinge, as on a downhill slope.
    struct DroppedGround;

    impl CollisionGrid for DroppedGround {
        fn is_solid_obstacle(&self, at: BlockPos) -> bool {
            at.y < 7
        }
    }

    /// One solid block, as on an uphill step.
    struct BlockAt(BlockPos);

    impl CollisionGrid for BlockAt {
        fn is_solid_obstacle(&self, at: BlockPos) -> bool {
            at == self.0
        }
    }

    /// A straight trunk of `length` logs hinged at `(0, 10, 0)`, falling towards +X.
    fn trunk_tree(length: i32, fall_speed: f32) -> FallingTree {
        let hinge = pos(0, 10, 0);
        let blocks = (0..length)
            .map(|y| FallingBlock::relative_to(hinge, pos(0, 10 + y, 0), false, OAK))
            .collect();
        FallingTree::new(
            hinge,
            FallDirection { dx: 1.0, dz: 0.0 },
            length as f32,
            fall_speed,
            blocks,
        )
    }

    fn simulate_to_impact(tree: &mut FallingTree, grid: &dyn CollisionGrid, dt: f32) -> f32 {
        for _ in 0..10_000 {
            let events = tree.step(dt, grid);
            if let Some(speed) = events.impacted {
                assert!(speed >= 0.0);
                return tree.angle_rad();
            }
        }
        panic!("tree never reached an impact");
    }

    #[test]
    fn fall_direction_points_away_from_the_player() {
        let hinge = pos(0, 0, 0);
        let east = FallDirection::away_from_player(8.5, 0.5, hinge);
        assert!(east.dx < -0.99 && east.dz.abs() < 1e-6);

        let south = FallDirection::away_from_player(0.5, -9.5, hinge);
        assert!(south.dz > 0.99 && south.dx.abs() < 1e-6);

        let diagonal = FallDirection::away_from_player(-2.5, -2.5, hinge);
        let length = (diagonal.dx * diagonal.dx + diagonal.dz * diagonal.dz).sqrt();
        assert!((length - 1.0).abs() < 1e-6);
        assert!(diagonal.dx > 0.0 && diagonal.dz > 0.0);
    }

    #[test]
    fn fall_direction_falls_back_when_the_player_is_on_the_hinge() {
        let hinge = pos(0, 0, 0);

        let direction = FallDirection::away_from_player(0.5, 0.5, hinge);

        assert_eq!(direction, FallDirection::DEFAULT);
        assert_eq!(direction.dx, 1.0);
        assert_eq!(direction.dz, 0.0);
    }

    #[test]
    fn warning_lasts_half_a_second_and_leans_one_to_three_degrees() {
        let mut tree =
            FallingTree::new(pos(0, 0, 0), FallDirection::DEFAULT, 10.0, 1.5, Vec::new());

        assert!((tree.angle_rad().to_degrees() - WARNING_START_DEG).abs() < 1e-4);

        tree.step(0.25, &NoObstacles);
        assert!(matches!(tree.phase, FallPhase::Warning { .. }));
        assert!((tree.angle_rad().to_degrees() - 2.0).abs() < 0.01);

        let start = 0.25;
        let mut elapsed = start;
        let dt = 1.0 / 120.0;
        loop {
            let events = tree.step(dt, &NoObstacles);
            elapsed += dt;
            if events.entered_falling {
                break;
            }
            assert!(elapsed < WARNING_DURATION_SECS + 0.1);
        }

        assert!((elapsed - WARNING_DURATION_SECS).abs() < 0.02);
        assert!(matches!(tree.phase, FallPhase::Falling { .. }));
        assert!((tree.angle_rad().to_degrees() - WARNING_END_DEG).abs() < 0.01);
    }

    /// Real seconds from entering the fall to reaching horizontal, warning excluded.
    fn time_to_horizontal(length: f32) -> f32 {
        let mut tree = FallingTree::new(
            pos(0, 0, 0),
            FallDirection::DEFAULT,
            length,
            1.5,
            Vec::new(),
        );
        let dt = 1.0 / 60.0;
        let mut elapsed = 0.0;
        let mut falling_since = None;

        for _ in 0..20_000 {
            let events = tree.step(dt, &NoObstacles);
            elapsed += dt;
            if events.entered_falling {
                falling_since = Some(elapsed);
            }
            if let Some(start) = falling_since
                && matches!(tree.phase, FallPhase::Falling { .. })
                && tree.angle_rad() >= FRAC_PI_2
            {
                return elapsed - start;
            }
        }
        panic!("tree never reached horizontal");
    }

    #[test]
    fn fall_times_match_the_concept_table_at_one_and_a_half_speed() {
        // Concept table, game default column: L=5 -> 1.6 s, 10 -> 2.3 s,
        // 20 -> 3.2 s, 30 -> 4.0 s.
        for (length, expected) in [(5.0, 1.6), (10.0, 2.3), (20.0, 3.2), (30.0, 4.0)] {
            let actual = time_to_horizontal(length);
            assert!(
                (actual - expected).abs() <= 0.25,
                "L={length}: expected about {expected} s, got {actual} s"
            );
        }
    }

    #[test]
    fn impact_bounces_three_degrees_then_breaks_up_one_second_later() {
        let mut tree = trunk_tree(10, 1.5);
        let dt = 0.01;

        let mut impact_angle = None;
        let mut impact_events = 0;
        for _ in 0..2_000 {
            let events = tree.step(dt, &FlatGround);
            if events.impacted.is_some() {
                impact_events += 1;
                if let FallPhase::Impact {
                    impact_angle_rad, ..
                } = tree.phase
                {
                    impact_angle = Some(impact_angle_rad.to_degrees());
                }
                break;
            }
        }
        assert_eq!(impact_events, 1);
        let impact_angle = impact_angle.expect("the flat ground should stop the trunk");
        assert!(
            impact_angle > 90.0 && impact_angle < 96.0,
            "flat ground stopped at {impact_angle} degrees"
        );

        let mut after_impact = 0.0;
        while after_impact < 0.1 {
            tree.step(dt, &FlatGround);
            after_impact += dt;
        }
        let peak = tree.angle_rad().to_degrees();
        assert!(
            (impact_angle - peak - BOUNCE_DEG).abs() < 0.2,
            "bounce should reach {BOUNCE_DEG} degrees, peak was {peak} degrees"
        );

        while after_impact < 0.25 {
            tree.step(dt, &FlatGround);
            after_impact += dt;
        }
        assert!((tree.angle_rad().to_degrees() - impact_angle).abs() < 0.01);

        let mut ready_events = 0;
        while after_impact < 1.1 {
            let events = tree.step(dt, &FlatGround);
            after_impact += dt;
            if events.ready_to_break_up {
                ready_events += 1;
            }
            if matches!(tree.phase, FallPhase::BreakUp { .. }) {
                break;
            }
            assert!(events.impacted.is_none(), "the tree must bounce only once");
        }
        assert_eq!(ready_events, 1);
        assert!((after_impact - BREAKUP_DELAY_SECS).abs() < 0.03);
        assert!(matches!(tree.phase, FallPhase::BreakUp { .. }));
        assert!((tree.angle_rad().to_degrees() - impact_angle).abs() < 0.01);
    }

    #[test]
    fn uphill_terrain_stops_the_fall_before_ninety_degrees() {
        let mut tree = trunk_tree(10, 1.5);

        let angle = simulate_to_impact(&mut tree, &BlockAt(pos(2, 12, 0)), 0.01);

        assert!(
            angle.to_degrees() < 90.0,
            "uphill should stop the trunk early, stopped at {} degrees",
            angle.to_degrees()
        );
        assert!(matches!(tree.phase, FallPhase::Impact { .. }));
    }

    #[test]
    fn flat_ground_stops_just_past_ninety_degrees() {
        let mut tree = trunk_tree(10, 1.5);

        let angle = simulate_to_impact(&mut tree, &FlatGround, 0.01);

        assert!(angle.to_degrees() > 90.0);
        assert!(angle.to_degrees() < 96.0);
    }

    #[test]
    fn downhill_terrain_continues_past_ninety_degrees() {
        let mut tree = trunk_tree(10, 1.5);

        let angle = simulate_to_impact(&mut tree, &DroppedGround, 0.01);

        assert!(
            angle > FRAC_PI_2,
            "downhill should pass horizontal, stopped at {} degrees",
            angle.to_degrees()
        );
        assert!(angle < MAX_FALL_ANGLE_RAD);
    }

    #[test]
    fn passable_blocks_do_not_stop_the_fall() {
        let mut tree = trunk_tree(10, 1.5);

        let angle = simulate_to_impact(&mut tree, &NoObstacles, 0.01);

        assert!((angle - MAX_FALL_ANGLE_RAD).abs() < 1e-4);
        assert!(angle.to_degrees() > 90.0);
    }

    /// A trunk of `length` logs whose bottom log is gone, hinged at `(0, 10, 0)`.
    fn dropping_tree(length: i32) -> FallingTree {
        let hinge = pos(0, 10, 0);
        let blocks = (0..length)
            .map(|y| {
                let mut block = FallingBlock::relative_to(hinge, pos(0, 10 + y, 0), false, OAK);
                block.vanished = y == 0;
                block
            })
            .collect();
        FallingTree::new(
            hinge,
            FallDirection { dx: 1.0, dz: 0.0 },
            (length - 1) as f32,
            1.5,
            blocks,
        )
        .with_drop(1.0)
    }

    #[test]
    fn a_tree_with_a_cut_log_stands_still_then_drops_then_tips() {
        let mut tree = dropping_tree(6);
        let dt = 1.0 / 120.0;

        // Still: no lean and no shift while it waits.
        tree.step(0.25, &FlatGround);
        assert!(matches!(tree.phase, FallPhase::Warning { .. }));
        assert_eq!(tree.angle_rad(), 0.0);
        assert_eq!(tree.base_shift(), 0.0);

        let mut landed = None;
        let mut last_shift = 0.0;
        let mut was_dropping = false;
        for _ in 0..2_000 {
            let events = tree.step(dt, &FlatGround);
            if matches!(tree.phase, FallPhase::Dropping { .. }) {
                was_dropping = true;
                assert_eq!(tree.angle_rad(), 0.0, "no tilt while dropping");
                assert!(tree.base_shift() <= last_shift, "it only moves down");
                last_shift = tree.base_shift();
            }
            if let Some(speed) = events.landed {
                landed = Some(speed);
                assert!(events.entered_falling);
                break;
            }
        }
        assert!(was_dropping);
        assert!(landed.expect("the tree lands on the stump") > 0.0);
        assert!(matches!(tree.phase, FallPhase::Falling { .. }));
        assert_eq!(tree.base_shift(), -1.0);
        assert!(tree.angle_rad() > 0.0 && tree.angle_rad().to_degrees() < 3.0);
    }

    #[test]
    fn the_drop_takes_the_time_a_block_needs_to_fall_one_block() {
        let mut tree = dropping_tree(6);
        let dt = 1.0 / 240.0;
        let mut elapsed = 0.0;
        let mut drop_started = None;
        loop {
            let events = tree.step(dt, &FlatGround);
            elapsed += dt;
            if drop_started.is_none() && matches!(tree.phase, FallPhase::Dropping { .. }) {
                drop_started = Some(elapsed);
            }
            if events.landed.is_some() {
                break;
            }
            assert!(elapsed < 3.0);
        }
        // sqrt(2 * 1 / 9.8) = 0.452 simulated seconds, 1.5x faster in real time.
        let real = elapsed - drop_started.expect("the drop starts");
        assert!((real - 0.452 / 1.5).abs() < 0.03, "dropped in {real} s");
    }

    #[test]
    fn a_dropped_tree_tips_around_the_stump_top_and_rests_on_flat_ground() {
        let mut tree = dropping_tree(6);

        let angle = simulate_to_impact(&mut tree, &FlatGround, 0.01);

        assert!(angle.to_degrees() > 90.0 && angle.to_degrees() < 96.0);
    }

    #[test]
    fn the_cut_log_is_not_part_of_the_collision_and_keeps_its_place_for_drops() {
        let tree = dropping_tree(3);
        assert!(tree.blocks[0].vanished);
        assert_eq!(
            tree.block_center(&tree.blocks[0]),
            tree.blocks[0].center_offset()
        );
        // The first remaining log sits on the hinge once the tree has dropped.
        let mut landed = tree.clone();
        landed.phase = FallPhase::Falling {
            angle_rad: 0.0,
            angular_vel: 0.0,
        };
        let centre = landed.block_center(&landed.blocks[1]);
        assert!((centre[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn break_up_runs_from_the_base_to_the_tip() {
        let hinge = pos(0, 0, 0);
        let tree = FallingTree::new(
            hinge,
            FallDirection::DEFAULT,
            2.0,
            1.5,
            vec![
                FallingBlock::relative_to(hinge, pos(2, 2, 0), false, OAK),
                FallingBlock::relative_to(hinge, pos(0, 0, 0), false, OAK),
                FallingBlock::relative_to(hinge, pos(1, 1, 0), true, OAK),
            ],
        );

        let order: Vec<BlockPos> = tree
            .break_up_order()
            .iter()
            .map(|block| block.pos)
            .collect();

        assert_eq!(order, [pos(0, 0, 0), pos(1, 1, 0), pos(2, 2, 0)]);
    }
}

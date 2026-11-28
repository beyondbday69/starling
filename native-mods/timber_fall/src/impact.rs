//! Collision, rotation math and impact damage for the falling tree.
//!
//! The tree rotates around a horizontal axis perpendicular to [`FallDirection`], so
//! vertical local offsets (a trunk block above the hinge) swing towards the fall
//! direction. A row-major 3x4 transform is produced for Horizon's `ModelInstance`.

use crate::falling::{FallDirection, FallingTree};
use crate::scanner::BlockPos;

/// Impact damage at the highest speed the model allows.
pub const MAX_IMPACT_DAMAGE: f32 = 20.0;
/// Damage per unit of linear impact speed (rad/s times blocks from the hinge).
pub const DAMAGE_PER_SPEED: f32 = 2.0;

/// Answers whether a world position would stop the falling trunk.
///
/// Grass, flowers, air and other passable blocks return `false`; solid terrain
/// returns `true`. The grid must leave out the felled tree's own blocks, which are
/// removed together with the cut.
pub trait CollisionGrid {
    fn is_solid_obstacle(&self, pos: BlockPos) -> bool;
}

/// Rotates a local offset around the hinge, tipping towards `dir`.
pub fn rotate_offset(offset: [f32; 3], dir: FallDirection, angle_rad: f32) -> [f32; 3] {
    let rotation = rotation(dir, angle_rad);
    let o = [offset[0] as f64, offset[1] as f64, offset[2] as f64];
    [
        dot(&rotation[0], &o) as f32,
        dot(&rotation[1], &o) as f32,
        dot(&rotation[2], &o) as f32,
    ]
}

/// Row-major 3x4 transform taking a block's model space into world space.
///
/// The rotation tips the tree towards `dir`; the translation is the hinge plus the
/// rotated `offset`, so at 0 radians the transform is the identity rotation with the
/// block offset as translation.
pub fn block_transform_3x4(
    hinge: [f64; 3],
    dir: FallDirection,
    angle_rad: f32,
    offset: [f32; 3],
) -> [f32; 12] {
    let rotation = rotation(dir, angle_rad);
    let o = [offset[0] as f64, offset[1] as f64, offset[2] as f64];
    let r0 = dot(&rotation[0], &o);
    let r1 = dot(&rotation[1], &o);
    let r2 = dot(&rotation[2], &o);
    let t = [hinge[0] + r0, hinge[1] + r1, hinge[2] + r2];

    [
        rotation[0][0] as f32,
        rotation[0][1] as f32,
        rotation[0][2] as f32,
        t[0] as f32,
        rotation[1][0] as f32,
        rotation[1][1] as f32,
        rotation[1][2] as f32,
        t[1] as f32,
        rotation[2][0] as f32,
        rotation[2][1] as f32,
        rotation[2][2] as f32,
        t[2] as f32,
    ]
}

/// Whether any trunk block would end up inside a solid block at `angle_rad`.
///
/// Leaves are ignored: the concept stops the fall on solid ground with the trunk,
/// and lets the canopy pass through grass and flowers.
pub fn hits_solid(tree: &FallingTree, angle_rad: f32, grid: &dyn CollisionGrid) -> bool {
    let hinge = tree.hinge_center();
    tree.blocks
        .iter()
        .filter(|block| !block.is_leaf && !block.vanished)
        .any(|block| {
            let center = rotate_offset(tree.block_center(block), tree.direction, angle_rad);
            let at = BlockPos::new(
                (hinge[0] + center[0] as f64).floor() as i32,
                (hinge[1] + center[1] as f64).floor() as i32,
                (hinge[2] + center[2] as f64).floor() as i32,
            );
            grid.is_solid_obstacle(at)
        })
}

/// Damage a creature takes where the trunk lands, scaled by how far it stood from
/// the hinge (the linear speed there is angular speed times distance).
pub fn impact_damage(
    impact_angular_vel: f32,
    distance_from_hinge: f32,
    falling_damage_enabled: bool,
) -> f32 {
    if !falling_damage_enabled {
        return 0.0;
    }
    let linear_speed = impact_angular_vel.abs() * distance_from_hinge.abs();
    if !linear_speed.is_finite() {
        return 0.0;
    }
    (linear_speed * DAMAGE_PER_SPEED).min(MAX_IMPACT_DAMAGE)
}

/// The rotation matrix for tipping towards `dir` by `angle_rad`.
///
/// The axis is `(dz, 0, -dx)`, which turns the up vector onto the fall direction.
fn rotation(dir: FallDirection, angle_rad: f32) -> [[f64; 3]; 3] {
    let angle = angle_rad as f64;
    let (sin, cos) = angle.sin_cos();
    let (dx, dz) = (dir.dx as f64, dir.dz as f64);
    let one_minus_cos = 1.0 - cos;
    [
        [
            cos + one_minus_cos * dz * dz,
            sin * dx,
            -one_minus_cos * dx * dz,
        ],
        [-sin * dx, cos, -sin * dz],
        [
            -one_minus_cos * dx * dz,
            sin * dz,
            cos + one_minus_cos * dx * dx,
        ],
    ]
}

fn dot(row: &[f64; 3], vector: &[f64; 3]) -> f64 {
    row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2]
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::falling::FallingBlock;

    const OAK: u16 = 10;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new(x, y, z)
    }

    fn x_direction() -> FallDirection {
        FallDirection { dx: 1.0, dz: 0.0 }
    }

    struct EverythingSolid;

    impl CollisionGrid for EverythingSolid {
        fn is_solid_obstacle(&self, _pos: BlockPos) -> bool {
            true
        }
    }

    #[test]
    fn transform_at_zero_angle_is_identity_rotation_plus_offset() {
        let matrix = block_transform_3x4([0.0, 0.0, 0.0], x_direction(), 0.0, [1.0, 2.0, 3.0]);

        assert_eq!(
            matrix,
            [
                1.0, 0.0, 0.0, 1.0, //
                0.0, 1.0, 0.0, 2.0, //
                0.0, 0.0, 1.0, 3.0,
            ]
        );
    }

    #[test]
    fn transform_places_the_hinge_in_world_space() {
        let matrix = block_transform_3x4([10.0, 20.0, 30.0], x_direction(), 0.0, [1.0, 2.0, 3.0]);

        assert_eq!(&matrix[3], &11.0);
        assert_eq!(&matrix[7], &22.0);
        assert_eq!(&matrix[11], &33.0);
    }

    #[test]
    fn ninety_degrees_turns_vertical_onto_the_fall_direction() {
        let matrix =
            block_transform_3x4([0.0, 0.0, 0.0], x_direction(), FRAC_PI_2, [0.0, 5.0, 0.0]);

        assert!(
            (matrix[3] - 5.0).abs() < 1e-5,
            "x translation {:?}",
            matrix[3]
        );
        assert!(matrix[7].abs() < 1e-5, "y translation {:?}", matrix[7]);
        assert!(matrix[11].abs() < 1e-5, "z translation {:?}", matrix[11]);
    }

    #[test]
    fn ninety_degrees_on_a_diagonal_reaches_the_diagonal_axis() {
        let direction = FallDirection { dx: 0.6, dz: 0.8 };
        let matrix = block_transform_3x4([0.0, 0.0, 0.0], direction, FRAC_PI_2, [0.0, 5.0, 0.0]);

        assert!((matrix[3] - 3.0).abs() < 1e-5);
        assert!(matrix[7].abs() < 1e-5);
        assert!((matrix[11] - 4.0).abs() < 1e-5);
    }

    #[test]
    fn rotate_offset_matches_the_transform_translation() {
        let offset = [1.0, 6.0, -2.0];
        let rotated = rotate_offset(offset, x_direction(), 0.7);
        let matrix = block_transform_3x4([0.0, 0.0, 0.0], x_direction(), 0.7, offset);

        assert!((matrix[3] - rotated[0]).abs() < 1e-6);
        assert!((matrix[7] - rotated[1]).abs() < 1e-6);
        assert!((matrix[11] - rotated[2]).abs() < 1e-6);
    }

    #[test]
    fn collision_checks_the_trunk_and_ignores_leaves() {
        let hinge = pos(0, 0, 0);
        let leaf_only = FallingTree::new(
            hinge,
            x_direction(),
            1.0,
            1.5,
            vec![FallingBlock::relative_to(hinge, pos(0, 1, 0), true, OAK)],
        );
        assert!(!hits_solid(&leaf_only, 0.0, &EverythingSolid));
        assert!(!hits_solid(&leaf_only, FRAC_PI_2, &EverythingSolid));

        let with_trunk = FallingTree::new(
            hinge,
            x_direction(),
            1.0,
            1.5,
            vec![
                FallingBlock::relative_to(hinge, pos(0, 1, 0), false, OAK),
                FallingBlock::relative_to(hinge, pos(0, 2, 0), true, OAK),
            ],
        );
        assert!(hits_solid(&with_trunk, 0.0, &EverythingSolid));
    }

    #[test]
    fn an_empty_tree_hits_nothing() {
        let tree = FallingTree::new(pos(0, 0, 0), x_direction(), 1.0, 1.5, Vec::new());

        assert!(!hits_solid(&tree, 0.0, &EverythingSolid));
        assert!(!hits_solid(&tree, FRAC_PI_2, &EverythingSolid));
    }

    #[test]
    fn impact_damage_is_zero_when_disabled() {
        assert_eq!(impact_damage(4.0, 5.0, false), 0.0);
        assert_eq!(impact_damage(0.0, 0.0, false), 0.0);
    }

    #[test]
    fn impact_damage_grows_with_speed_and_is_capped() {
        assert_eq!(impact_damage(0.0, 5.0, true), 0.0);

        let slow = impact_damage(1.0, 1.0, true);
        let fast = impact_damage(2.0, 1.0, true);
        let faster = impact_damage(3.0, 1.0, true);
        assert!(slow < fast && fast < faster);

        assert_eq!(impact_damage(100.0, 100.0, true), MAX_IMPACT_DAMAGE);
        assert_eq!(impact_damage(f32::NAN, 5.0, true), 0.0);
    }
}

// Punchy Moves for Horizon (native mixin runtime): third-person character
// animation, a shoulder camera, expressions and emotes.
//
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game as `game::punchy_moves`. It is pure Rust: no engine
// dependency, so its tests run without a Horizon build.

/// The mod's id: its settings folder and its row in Options > Mods.
pub const ID: &str = "punchy_moves";
pub const VERSION: &str = "1.0.9";
pub const DESCRIPTION: &str =
    "Third-person character animation, shoulder camera, expressions and emotes. Horizon's own.";

pub mod camera;
pub mod easing;
pub mod engine;
pub mod expressions;
pub mod faces;
pub mod json;
pub mod library;
pub mod signals;
pub mod ui;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_id_is_the_mods_folder_name() {
        assert_eq!(ID, "punchy_moves");
        assert_eq!(VERSION.split('.').count(), 3);
        assert!(!DESCRIPTION.is_empty());
    }
}

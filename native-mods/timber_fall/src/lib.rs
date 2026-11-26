//! Timber Fall: felling a whole tree from one cut.
//!
//! The modules here are pure Rust with no Horizon engine dependency, so the rules
//! that decide whether a structure may fall are unit tested without the game build.
//!
//! `runtime` coordinates active trees on either side of the game. `hooks` is the
//! Horizon adapter and is compiled only with the `horizon-sdk` feature.

pub mod chop;
pub mod config;
pub mod drops;
pub mod falling;
#[cfg(feature = "horizon-sdk")]
pub mod hooks;
pub mod impact;
pub mod runtime;
pub mod scanner;

#[cfg(feature = "horizon-sdk")]
pub use hooks::TimberFallMod;
// The launcher calls `hooks()` at the crate root; the macro defines it in `hooks`.
#[cfg(feature = "horizon-sdk")]
pub use hooks::hooks;

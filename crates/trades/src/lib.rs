//! Two learned agents, and the line between them.
//!
//! `design/the-two-trades.md` is the mission. The split is **by screen**,
//! which is how a person meets the game: one agent plays the loadout and the
//! shop, the other plays the road, and `pack` is the second's one macro-action
//! into the first.
//!
//! That split is the whole reason this is learnable. THE APPRENTICE's pilot is
//! 195,000 presses a run and 77% of them are trial-seat-and-undo; no
//! temporal-difference method assigns credit across that. Thirty to sixty
//! decisions and two to five hundred are horizons that work.

pub mod brief;
pub mod env;
pub mod feature;
pub mod partition;
pub mod pathfinder;
pub mod qnet;

pub use env::{Goal, Move, Packing, Step, Walking};
pub use partition::{Trade, PATHFINDER, QUARTERMASTER};
pub use qnet::QNet;

//! Gear Master rules. No rendering dependencies — everything here is testable
//! with `cargo test` and no window.
//!
//! The shape of the game:
//!   * The character has five equipment slots, each a 6x8 grid.
//!   * Gear is not bought whole — it is *assembled* out of component pieces
//!     (polyominoes) dropped into a slot.
//!   * A slot whose contents satisfy its recipe AND form one connected group
//!     becomes assembled gear. Placed pieces always contribute their base
//!     stats; a piece's **assembly bonus** fires only once its slot assembles.
//!   * Combat is fully deterministic and simulated to completion up front
//!     (`combat::simulate`), producing a log the GUI replays.

pub mod bestiary;
pub mod class;
pub mod combat;
pub mod county;
pub mod dungeon;
pub mod event;
pub mod curse;
pub mod loadout;
pub mod naming;
pub mod pedestal;
pub mod piece;
pub mod quest;
pub mod rating;
pub mod relic;
pub mod rng;
pub mod rumour;
pub mod route;
pub mod run;
pub mod share;
pub mod shop;
pub mod shape;
pub mod slot;
pub mod stats;
pub mod theme;
pub mod town;

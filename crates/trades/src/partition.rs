//! Which agent owns which verb.
//!
//! **Exhaustive and disjoint**, and `tests/partition.rs` is what says so.
//! A verb owned by neither is an action nobody can take; a verb owned by both
//! is a decision two learners will fight over and neither will be blamed for.
//! Both faults are silent, and both are the shape of `CLAUDE.md` §6 trap 19 -
//! half a lint is not a lint - so the test asserts the partition in both
//! directions.
//!
//! ## Where the line falls, and why
//!
//! The quartermaster owns everything that is **the loadout screen and the
//! shop**: what you carry and where it sits. The pathfinder owns everything
//! that is **the road**: where you go and what you answer.
//!
//! Two calls are worth arguing with:
//!
//! * **The shop is the quartermaster's**, not the pathfinder's. Buying and
//!   placing are one decision - you buy a piece *because* of where it will go -
//!   and THE APPRENTICE's A6 measured the point: every gain in that mission
//!   came from acquisition, and a run at the wall carried six hundred gold and
//!   seven items. Splitting them would put the two halves of one decision in
//!   two agents.
//! * **`Crush` and `Pedestal` are the pathfinder's**, though they take a piece
//!   as their argument. Neither is about the board: crushing a relic buys a
//!   second town door or steps over a rung, and feeding an orb to a pedestal
//!   sends the run somewhere. They are road decisions that happen to be spelled
//!   with an item.

use gearmaster_console::Verb;

/// Which of the two owns a decision.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Trade {
    /// Buys and packs. Plays the loadout and the shop.
    Quartermaster,
    /// Walks and answers. Plays the road.
    Pathfinder,
}

/// The quartermaster's, named rather than matched, so the list can be read.
pub const QUARTERMASTER: &[&str] = &[
    "Place",
    "PlaceLocked",
    "Unequip",
    "UnequipLocked",
    "Rotate",
    "RotateLocked",
    "Lock",
    "ClearSlot",
    "ClearAll",
    "Undo",
    "Grow",
    "Buy",
    "Sell",
    "Barter",
    "Reroll",
    "Pin",
];

/// The pathfinder's.
pub const PATHFINDER: &[&str] = &[
    "Answer",
    "AnswerWith",
    "Fight",
    "FightParty",
    "Town",
    "WalkOn",
    "ThrowPoints",
    "Leave",
    "Walk",
    "Out",
    "Perambulate",
    "Drink",
    "DrinkChoosing",
    "Double",
    "Pedestal",
    "Crush",
];

/// The variant's name, which is the key both lists are written in.
pub fn name_of(v: Verb) -> &'static str {
    match v {
        Verb::Place { .. } => "Place",
        Verb::PlaceLocked { .. } => "PlaceLocked",
        Verb::Unequip { .. } => "Unequip",
        Verb::UnequipLocked { .. } => "UnequipLocked",
        Verb::Rotate { .. } => "Rotate",
        Verb::RotateLocked { .. } => "RotateLocked",
        Verb::Lock { .. } => "Lock",
        Verb::ClearSlot { .. } => "ClearSlot",
        Verb::ClearAll => "ClearAll",
        Verb::Undo => "Undo",
        Verb::Grow { .. } => "Grow",
        Verb::Buy { .. } => "Buy",
        Verb::Sell { .. } => "Sell",
        Verb::Barter { .. } => "Barter",
        Verb::Reroll => "Reroll",
        Verb::Pin { .. } => "Pin",
        Verb::Answer { .. } => "Answer",
        Verb::AnswerWith { .. } => "AnswerWith",
        Verb::Fight => "Fight",
        Verb::FightParty => "FightParty",
        Verb::Town { .. } => "Town",
        Verb::WalkOn => "WalkOn",
        Verb::ThrowPoints { .. } => "ThrowPoints",
        Verb::Leave => "Leave",
        Verb::Walk { .. } => "Walk",
        Verb::Out => "Out",
        Verb::Perambulate { .. } => "Perambulate",
        Verb::Drink => "Drink",
        Verb::DrinkChoosing { .. } => "DrinkChoosing",
        Verb::Double { .. } => "Double",
        Verb::Pedestal { .. } => "Pedestal",
        Verb::Crush { .. } => "Crush",
    }
}

/// Who owns this verb.
pub fn owner(v: Verb) -> Trade {
    let n = name_of(v);
    if QUARTERMASTER.contains(&n) {
        Trade::Quartermaster
    } else {
        Trade::Pathfinder
    }
}

/// Split a menu into the two agents' halves.
///
/// What each learner is offered at a decision point, and the only place either
/// of them ever sees a `Verb` it does not own.
pub fn split(menu: &[Verb]) -> (Vec<Verb>, Vec<Verb>) {
    let mut q = Vec::new();
    let mut p = Vec::new();
    for &v in menu {
        match owner(v) {
            Trade::Quartermaster => q.push(v),
            Trade::Pathfinder => p.push(v),
        }
    }
    (q, p)
}

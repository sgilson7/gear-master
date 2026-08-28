//! The reconstruction fault, guarded.
//!
//! `CLAUDE.md` §6 trap 4: a name list is not a board, and a board rebuilt
//! without locking each item as it completes comes back as loose pieces. It
//! has been learned four times in this repo, the last when a reference build
//! came back as zero items. Everything in this crate rebuilds through
//! `Board::rebuild`, and this is what says that path is the right one.

use gearmaster_oracle::gate::References;
use gearmaster_oracle::Board;

#[test]
fn the_owners_board_comes_back_as_the_items_its_owner_built() {
    let refs = References::standard();
    let (label, stats, items, board) = &refs.boards[2];
    assert_eq!(*label, "owner");
    assert_eq!(board.gear.len(), 75, "seventy-five pieces went into that code");
    assert_eq!(
        items.len(),
        19,
        "the owner's board is nineteen items; anything less is the reconstruction fault"
    );
    assert!(stats.health > 500, "and it is a finished build");
}

#[test]
fn every_reference_board_assembled_something() {
    // The four-piece board is the one most likely to come back empty, and it
    // is the yardstick the bottom of the ladder is written for.
    for (label, _, items, board) in &References::standard().boards {
        assert!(
            !items.is_empty(),
            "the {} board assembled nothing out of {} pieces",
            label,
            board.gear.len()
        );
    }
}

#[test]
fn an_empty_board_is_empty_and_says_so() {
    let b = Board::default();
    let (stats, items) = b.profiles();
    assert!(items.is_empty());
    assert_eq!(b.cells(), 0);
    // **Not zero health.** `Loadout::total_stats` starts a character at 100,
    // which is the other side of `CLAUDE.md` trap 28: a combatant built from
    // `Stats::ZERO` has no maximum health and is dead on the first tick, so
    // every measurement off that fight reads as "the mechanic does nothing".
    // An empty *board* is a hundred-hit-point player with nothing to swing,
    // and that is a fight - a short one.
    assert_eq!(stats.health, 100, "an empty board is not a dead player");
}

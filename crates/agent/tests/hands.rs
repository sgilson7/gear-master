//! The hands, on trays small enough to reason about.
//!
//! Sub-second, and none of them fights anything: what is being checked is that
//! the pilot builds a board out of what the screen says, and that it stops
//! rather than filling cells for the sake of it.

use gearmaster_agent::hands;
use gearmaster_agent::sense::Sense;
use gearmaster_console::{Console, Difficulty, Mode, Verb};

fn fresh() -> Console {
    Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium)
}

#[test]
fn it_finishes_the_item_the_starter_kit_makes() {
    // A handle and a blade are a weapon if they touch, and two loose pieces if
    // they do not. The A1 control seated the first thing that fitted and got
    // the second answer.
    let mut c = fresh();
    assert_eq!(Sense::of(&c.view()).items, 0, "nothing is assembled to begin with");
    let packed = hands::pack(&mut c, 5_000);
    assert_eq!(packed.seated, 2);
    assert_eq!(packed.items, 1, "a handle and a blade make one weapon");
    assert!(packed.presses < 1_500, "{} presses for two pieces", packed.presses);
}

#[test]
fn a_board_it_has_finished_is_worth_more_than_the_empty_one() {
    let mut c = fresh();
    let before = Sense::of(&c.view()).worth();
    hands::pack(&mut c, 5_000);
    let after = Sense::of(&c.view());
    assert!(after.worth() > before, "packing made the board worse");
    assert!(!after.inert(), "and something acts at the end of it");
}

#[test]
fn it_leaves_the_board_where_it_says_it_did() {
    // Every trial seat is taken back with the player's own undo, so the board
    // at the end is the board it chose rather than the last one it tried.
    // That is the fault `CLAUDE.md` §6 trap 33 describes - an inner break
    // leaves the loop, not the search - in the shape it takes here.
    let mut c = fresh();
    hands::pack(&mut c, 5_000);
    let v = c.view();
    let seated: usize = v.grids.iter().map(|g| g.cells.iter().filter(|c| c.piece.is_some()).count()).sum();
    assert!(seated > 0, "it kept nothing");
    assert!(v.tray.is_empty() || seated > 0);
    // And the undo history is not holding a board it meant to discard.
    let worth = Sense::of(&v).worth();
    c.apply(Verb::Undo);
    assert!(
        Sense::of(&c.view()).worth() <= worth,
        "undoing once improved the board, so the pilot stopped one press early"
    );
}

#[test]
fn packing_twice_from_one_seed_builds_one_board() {
    let (mut a, mut b) = (fresh(), fresh());
    hands::pack(&mut a, 5_000);
    hands::pack(&mut b, 5_000);
    assert_eq!(a.screen(), b.screen());
}

#[test]
fn a_budget_of_nothing_presses_nothing() {
    let mut c = fresh();
    let packed = hands::pack(&mut c, 0);
    assert_eq!(packed.seated, 0);
    assert_eq!(Sense::of(&c.view()).items, 0);
}

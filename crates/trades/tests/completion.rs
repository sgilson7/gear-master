//! The three numbers Q7 said were missing.
//!
//! `crates/lab/src/bin/completes.rs` is the measurement - it presses the key
//! and counts items, and reports 100% recall at 87% precision. This is the
//! cheaper guard beside it: that the numbers exist, that they are read off
//! things the panel draws, and that the widening which took recall from 64%
//! to 100% is not quietly narrowed again.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::feature::{self, MOVE};

#[test]
fn a_move_has_room_for_the_completion_numbers() {
    assert_eq!(
        MOVE, 38,
        "the completion numbers, where the piece is going - which a move did not \
         say until the same seat at (3,2) and (4,2) was found to describe \
         identically - and, since `analysis/the-collapse.md` M1, a shape apiece \
         for the four verbs that shared a bucket and two numbers saying what a \
         lock would fix"
    );
}

#[test]
fn the_completion_numbers_are_only_ever_about_a_placement() {
    // Buying, selling and rerolling do not seat anything, so nothing they do
    // can finish an item and the feature must not claim otherwise.
    let c = Console::start(0xC0FFEE, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    for m in Packing::new(60).moves(&c) {
        let Move::Press(verb) = m else { continue };
        if matches!(verb, Verb::Place { .. }) {
            continue;
        }
        let f = feature::mv(&v, verb);
        assert_eq!(
            (f[26], f[27], f[28]),
            (0.0, 0.0, 0.0),
            "{:?} is not a placement and said something about completing one",
            verb
        );
    }
}

#[test]
fn a_placement_onto_an_empty_grid_cannot_finish_a_two_piece_recipe() {
    // The property the first version of `completes` was blind to: from an
    // empty board no single placement completes anything, so measuring the
    // opening move measured nothing. It reported 2,640 placements and zero
    // completions, which was the harness rather than the feature.
    // Cleared through the interface, because that is the only way this crate
    // can clear anything - it cannot see `Run`, which is the boundary working.
    let mut c = Console::start(0xC0FFEE, Mode::Grinder, Difficulty::Medium);
    c.apply(Verb::ClearAll);
    let v = c.view();
    let mut placements = 0;
    for m in Packing::new(60).moves(&c) {
        let Move::Press(verb @ Verb::Place { .. }) = m else { continue };
        placements += 1;
        assert_eq!(
            feature::mv(&v, verb)[27],
            0.0,
            "nothing is seated yet, so {:?} cannot finish an item",
            verb
        );
    }
    assert!(placements > 0, "there was nothing to place, so this proved nothing");
}

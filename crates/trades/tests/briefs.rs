//! What a brief is, and what it must never be.
//!
//! Q8's conditioning is thirteen numbers with no name on them. These pin the
//! two properties the milestone actually rests on - that a brief is
//! descriptive rather than categorical, and that it rides on the state side of
//! the pair - because both are the kind of thing a later refactor undoes
//! without noticing.

use gearmaster_trades::brief::{Brief, BRIEF};
use gearmaster_trades::feature::{self, BOARD, MOVE, PAIR};

#[test]
fn the_brief_rides_on_the_state_and_not_on_the_move() {
    // The same move is worth different amounts depending on what was asked
    // for, which is the whole claim Q8 measures. If the brief were appended to
    // the move instead, a network could not condition its estimate of the
    // *situation* on it at all.
    assert_eq!(PAIR, BOARD + BRIEF + MOVE);
    let b = feature::briefed(&[0.5; BOARD], &Brief([0.25; BRIEF]));
    assert_eq!(&b[..BOARD], &[0.5; BOARD]);
    assert_eq!(&b[BOARD..], &[0.25; BRIEF]);
    let p = feature::pair(&b, &[1.0; MOVE]);
    assert_eq!(&p[BOARD + BRIEF..], &[1.0; MOVE], "the move comes last");
}

#[test]
fn the_brief_that_asks_for_nothing_is_a_real_value_and_not_a_missing_one() {
    // Q8's control is a network trained on zeros, not a network with the
    // conditioning switched off - same shape, same weights, same everything.
    // A `None` here would have made the two arms structurally different and
    // the comparison meaningless.
    assert!(Brief::NONE.is_none());
    assert_eq!(Brief::NONE.0.len(), BRIEF);
    assert_eq!(Brief::NONE.likeness(&Brief([1.0; BRIEF])), 0.0);
}

#[test]
fn likeness_is_a_cosine_and_ignores_how_loud_a_brief_is() {
    // A theme that allows four hundred pieces and one that allows forty
    // should not differ mostly in how many pieces they allow.
    let a = Brief([1.0; BRIEF]);
    let mut twice = [0.0; BRIEF];
    twice[..].copy_from_slice(&[2.0; BRIEF]);
    assert!((a.likeness(&Brief(twice)) - 1.0).abs() < 1e-5);
}

#[test]
fn a_brief_says_which_grids_and_which_pools() {
    let mut f = [0.0f32; BRIEF];
    f[0] = 1.0;
    f[5] = 0.5;
    let w = Brief(f);
    assert_eq!(w.slots().len(), 5);
    assert_eq!(w.pools().len(), 8);
    assert_eq!(w.slots()[0], 1.0);
    assert_eq!(w.pools()[0], 0.5);
}

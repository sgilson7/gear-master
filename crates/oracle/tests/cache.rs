//! The cache must never lie.
//!
//! A memoised oracle is the difference between local search and resampling -
//! remove a piece, put it back, try the next seat, and the same board comes
//! round constantly. All of that is worthless if a hit ever differs from a
//! miss, and the failure would be silent: a search would climb a hill that is
//! not there.

use gearmaster_engine::combat::{Difficulty, ALTERNATES, LADDER};
use gearmaster_oracle::gate::References;
use gearmaster_oracle::{Oracle, Reading};

#[test]
fn a_hit_is_always_what_a_miss_would_have_said() {
    let refs = References::standard();
    let oracle = Oracle::new();
    // Twenty creatures by four boards by four settings is 272 distinct
    // fights; the lookups are what is being counted, so each is asked for
    // forty times. Distinct fights are the expensive half and repeats
    // are the half the cache exists for - which is also the shape a local
    // search has.
    let mut checked = 0u64;
    for spec in LADDER.iter().step_by(4).chain(ALTERNATES.iter().step_by(9)) {
        for (_, stats, items, board) in &refs.boards {
            for &d in Difficulty::ALL.iter() {
                let want = Oracle::fight_uncached(*stats, items, spec, d);
                for _ in 0..40 {
                    assert_eq!(
                        oracle.fight(board, *stats, items, spec, d),
                        want,
                        "the cache disagreed with the engine"
                    );
                    checked += 1;
                }
            }
        }
    }
    let (hits, misses) = oracle.tally();
    assert!(checked >= 10_000, "only checked {}", checked);
    assert_eq!(hits + misses, checked);
    let rate = hits as f64 / checked as f64;
    println!("{} lookups, {} misses, hit rate {:.1}%", checked, misses, rate * 100.0);
    assert!(rate > 0.96, "hit rate {:.3} - the cache is not holding", rate);
}

#[test]
fn two_boards_that_differ_are_not_one_key() {
    // The key is over sorted placements, so a re-ordering is the same board -
    // and a moved piece is not. If this ever fails, every number a search
    // produces is the number for whichever board got there first.
    let refs = References::standard();
    let owner = &refs.boards[2].3;
    let friend = &refs.boards[3].3;
    assert_ne!(owner.key(), friend.key());

    let mut shuffled = owner.clone();
    shuffled.gear.reverse();
    assert_eq!(owner.key(), shuffled.key(), "an order is not a board");

    let mut moved = owner.clone();
    moved.gear[0].2 = moved.gear[0].2.wrapping_add(1);
    assert_ne!(owner.key(), moved.key(), "a moved piece is a different board");
}

#[test]
fn a_reading_is_the_same_reading_twice() {
    // The environment has no noise in it, and the meter reads the log rather
    // than the run - so this is really a check that nothing in the reader
    // depends on iteration order.
    let refs = References::standard();
    let (_, stats, items, _) = &refs.boards[0];
    let spec = &LADDER[19];
    let a = Reading::of(&gearmaster_engine::combat::simulate_at(*stats, items, spec, Difficulty::Medium));
    let b = Reading::of(&gearmaster_engine::combat::simulate_at(*stats, items, spec, Difficulty::Medium));
    assert_eq!(a, b);
}

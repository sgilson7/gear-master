//! Nothing on the road gets walked past.
//!
//! A town gate, an event and a fountain are all drawn on the loadout screen,
//! and for a while there was one way to start a fight that never went back to
//! it: REMATCH, straight off the battle replay. By then the rung had already
//! advanced, so it was not a rematch at all - it was the next creature, begun
//! from a screen that never asked whether anything was waiting.
//!
//! A board good enough to keep pressing it stood on rung seven with the
//! fountain due and the town set, and arrived at rung ten with no class at
//! all. `Run::road_is_blocked` is the answer and this file is the guard.

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::share;

/// A board that can actually clear the early rungs. The auto-builder cannot -
/// it oscillates around rung six and never reaches the first fountain, which
/// makes it useless for asking questions about rung seven.
fn a_climbing_run(difficulty: Difficulty) -> Run {
    let sh = share::import(share::A_WINNING_RUN).expect("reads");
    let mut run = Run::new();
    run.difficulty = difficulty;
    run.mode = Mode::Grinder;
    for (d, sl, x, y, rot) in &sh.placed {
        let id = run.registry.alloc(*d);
        run.owned.push(id);
        run.registry.set_rotation(id, *rot);
        if run.equip(id, *sl, *x, *y).is_err() {
            run.owned.pop();
        }
    }
    run
}

#[test]
fn a_fountain_that_is_due_blocks_the_road() {
    let mut run = a_climbing_run(Difficulty::Easy);
    run.rung = Run::FOUNTAINS[0];
    assert!(run.at_fountain(), "the fixture is not standing where it thinks it is");
    assert_eq!(run.road_is_blocked(), Some("a fountain"));
}

#[test]
fn a_town_gate_blocks_the_road_even_mid_replay() {
    // The phase gate is the whole point: `pending_town` says "should this
    // screen be drawn", which is no during a fight. `road_is_blocked` says
    // "may a fight start", which has to be answerable from the battle screen.
    let mut run = a_climbing_run(Difficulty::Easy);
    run.rung = gearmaster_engine::town::TOWNS[0].after;
    run.force_win();
    run.settle();
    assert!(run.town.is_some(), "clearing the rung before a town did not reach it");
    assert_eq!(run.road_is_blocked(), Some("a town"));
    // Still blocked while the replay is up, which `pending_town` would deny.
    run.fight_next();
    assert!(run.pending_town().is_none(), "the gate should not be drawn over a fight");
    assert!(run.road_is_blocked().is_some(), "and it must still stop the next one starting");
}

#[test]
fn an_open_road_is_open() {
    let mut run = a_climbing_run(Difficulty::Easy);
    run.rung = 4;
    assert_eq!(run.road_is_blocked(), None, "rung four has nothing on it");
}

#[test]
fn a_run_that_only_ever_fights_still_meets_its_first_fountain() {
    // The reproduction, as a test. Fight, settle, fight again - never going
    // back to the loadout, which is what REMATCH did. The run must come to a
    // stop in front of the fountain rather than walking through it.
    let mut run = a_climbing_run(Difficulty::Easy);
    let mut fought = 0;
    for _ in 0..12 {
        if run.road_is_blocked().is_some() {
            break;
        }
        run.fight_next();
        run.settle();
        fought += 1;
    }
    let stopped_by = run.road_is_blocked();
    assert!(stopped_by.is_some(), "fought {fought} times and nothing ever stopped it");
    assert!(
        run.rung <= Run::FOUNTAINS[0],
        "walked to rung {} before anything stopped it; the first fountain is on {}",
        run.rung,
        Run::FOUNTAINS[0]
    );
}

#[test]
fn the_road_clears_once_the_thing_on_it_is_answered() {
    // A guard that stops the fix becoming a soft-lock: whatever blocks has to
    // be answerable, and answering it has to let the run move again.
    let mut run = a_climbing_run(Difficulty::Easy);
    run.rung = Run::FOUNTAINS[0];
    assert!(run.road_is_blocked().is_some());

    let pick = run.class_outlook().into_iter().find(|m| m.eligible).expect("a fountain always offers");
    run.drink_choosing(pick.class).expect("and it can always be drunk");
    assert_eq!(run.road_is_blocked(), None, "drank and the fountain is still standing there");
}

#[test]
fn every_fountain_rung_can_actually_be_stood_on() {
    // The quiet version of the same bug: a fountain scheduled onto a rung the
    // ladder does not have, or onto the same rung as a town, would never be
    // offered and nothing would say so.
    for (n, &rung) in Run::FOUNTAINS.iter().enumerate() {
        assert!(
            rung < gearmaster_engine::combat::LADDER.len(),
            "fountain {n} stands past the end of the road"
        );
    }
    assert!(
        Run::DOUBLING_FOUNTAIN < gearmaster_engine::combat::LADDER.len(),
        "the deep fountain stands past the end of the road"
    );
}

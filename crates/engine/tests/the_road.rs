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
use gearmaster_engine::route::Fill;
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
    // Named, not merely non-empty. Sump Bottom's gate stands at rung seven and
    // so does the first fountain, so "something is blocking the road" was
    // answerable by the wrong one of the two - and was, the first time the
    // road stack read the phase-gated question here.
    assert!(
        run.road_stack().iter().any(|i| i.kind() == "town"),
        "the gate itself has to still be on the stack, not just something else"
    );
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

// ------------------------------------------- the map says what is happening
//
// Reported from a real run: on rung three the yellow dot was on TWO BY TWO
// while the door actually being answered was THE CASINO, and the casino's dot
// was nine rungs away. Both halves of that are the map reading `LadderEvent::at`
// and `fill_for` as if an earned event stood on one rung, which it does not.

fn a_shallow_run() -> Run {
    let mut run = Run::seeded(0x51DE_0001);
    run.difficulty = Difficulty::Medium;
    // A quick kill anywhere in the shallow end opens the casino.
    run.best_fight_ms = Some(1_000);
    run.rung = 2;
    run
}

fn node<'a>(map: &'a gearmaster_engine::route::RouteMap, id: &str) -> &'a gearmaster_engine::route::Node {
    map.nodes.iter().find(|n| n.id == id).unwrap_or_else(|| panic!("{id} is not on the map"))
}

#[test]
fn an_earned_door_is_drawn_on_the_rung_it_is_standing_on() {
    let run = a_shallow_run();
    let map = gearmaster_engine::route::route(&run);
    let casino = node(&map, "the-casino");
    assert_eq!(
        casino.at, 2,
        "the casino is standing on rung three; its `at` is 8, which is its deadline"
    );
    assert_eq!(casino.fill, Fill::Current, "and it is one of the doors being asked");
}

#[test]
fn only_a_door_that_is_standing_is_ringed() {
    let mut run = a_shallow_run();
    // Both stand on rung three. The toad is asked first, so both are Current.
    let map = gearmaster_engine::route::route(&run);
    assert_eq!(node(&map, "the-toads-offer").fill, Fill::Current);
    assert_eq!(node(&map, "the-casino").fill, Fill::Current);

    // A door on a rung behind you that never happened is not "cleared".
    run.rung = 6;
    run.best_fight_ms = None;
    let map = gearmaster_engine::route::route(&run);
    assert_eq!(
        node(&map, "the-toads-offer").fill,
        Fill::Ahead,
        "an unanswered door did not happen, whichever rung it was on"
    );
    assert_eq!(
        node(&map, "back-in-a-minute").fill,
        Fill::Ahead,
        "and nor did this one, which is two rungs behind"
    );
}

#[test]
fn an_answered_door_is_drawn_where_it_was_answered() {
    let mut run = a_shallow_run();
    let casino = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == "the-casino").unwrap();
    let toad = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == "the-toads-offer").unwrap();
    run.take_choice(toad.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").unwrap());
    run.take_choice(casino.choices.iter().find(|c| c.label == "Keep out of it").unwrap());
    assert!(run.answered.contains(&"the-casino"));

    // Walk on. The casino stays where it happened rather than jumping to its
    // deadline or following the run up the road.
    run.rung = 9;
    let map = gearmaster_engine::route::route(&run);
    let n = node(&map, "the-casino");
    assert_eq!(n.at, 2, "answered on rung three, drawn on rung three");
    assert_eq!(n.fill, Fill::Cleared);
}

#[test]
fn a_door_nobody_has_earned_is_drawn_where_it_could_first_appear() {
    let mut run = Run::seeded(0x51DE_0001);
    run.difficulty = Difficulty::Medium;
    let map = gearmaster_engine::route::route(&run);
    // The casino's window is rungs two to nine. Its `at` is the deadline.
    assert_eq!(
        node(&map, "the-casino").at,
        1,
        "an unearned window is drawn at its opening, not at the rung it shuts on"
    );
    // A scheduled door has one rung and it is `at`.
    assert_eq!(node(&map, "the-toads-offer").at, 2);
}

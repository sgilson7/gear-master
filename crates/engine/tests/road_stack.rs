//! Nothing on the road gets walked past, and now there is a thing that says so.
//!
//! `the_road.rs` holds the doctrine: a fountain, a town gate, an event or a
//! dungeon standing on a rung all stop the next fight from starting. Four
//! separate predicates enforced it and the interface enforced it again in its
//! own words, and the two agreed because somebody kept them agreeing.
//!
//! `Run::road_stack` is that order written down once. The rung's own fight is
//! not in it: the fight is the floor the stack stands on, and it begins when
//! the stack is empty.
//!
//! **Derived rather than stored.** The spec asks for a `Vec<Interrupt>` field
//! pushed on arrival and popped on resolution; it is a function over run state
//! instead, because every entry is already decided by a field that exists and
//! a second copy is a second thing to keep true. Two of this project's bugs
//! were exactly that shape - `at_fountain` counting classes it had not poured,
//! and a fountain schedule keyed on a number something else was adding to.

mod common;

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::{Interrupt, Mode, Run};

fn a_run() -> Run {
    let mut run = Run::seeded(0x51DE_0001);
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

/// Walk to `rung` without fighting anything, so the fixture is about the road
/// rather than about whether a board can win.
fn stand_at(run: &mut Run, rung: usize) {
    run.rung = rung;
}

#[test]
fn an_empty_rung_has_an_empty_stack() {
    let mut run = a_run();
    stand_at(&mut run, 4);
    assert!(run.road_stack().is_empty());
    assert_eq!(run.road_is_blocked(), None);
}

#[test]
fn everything_that_blocks_the_road_is_on_the_stack_and_nothing_else_is() {
    // The two questions are the same question: `road_is_blocked` is the first
    // entry that stops a replay, and there is no third source of truth.
    let mut run = a_run();
    for rung in 0..gearmaster_engine::combat::LADDER.len() {
        stand_at(&mut run, rung);
        let stack = run.road_stack();
        let blocked = run.road_is_blocked();
        let first = stack.iter().find(|i| i.blocks_a_rematch());
        assert_eq!(blocked, first.map(|i| i.blocking_name()), "rung {}", rung + 1);
    }
}

#[test]
fn the_gate_comes_before_the_fountain_and_the_fountain_before_the_event() {
    // Rung seven holds both a town gate and the first fountain, which is not a
    // coincidence anybody arranged and is exactly why the order has to be
    // written down. The spec asks for fountain first; the game has always
    // asked the gate first, and the shipped towns' tests read it that way.
    let mut run = a_run();
    run.rung = gearmaster_engine::town::TOWNS[0].after;
    run.force_win();
    run.settle();
    let stack = run.road_stack();
    assert!(run.town.is_some(), "the fixture did not reach the gate");
    assert!(run.at_fountain(), "the fixture did not reach the fountain");
    let kinds: Vec<&str> = stack.iter().map(|i| i.kind()).collect();
    assert_eq!(&kinds[..2], &["town", "fountain"]);
    assert_eq!(run.road_is_blocked(), Some("a town"));
}

#[test]
fn a_dungeon_sits_on_top_of_whatever_it_was_entered_from() {
    // Being inside one is not something waiting for you - it is where you are.
    // And a dungeon does not block a replay, because a dungeon is where the
    // fighting happens while you are in one.
    let mut run = a_run();
    let d = gearmaster_engine::dungeon::by_id("the-crevice").expect("the shipped dungeon");
    run.dungeon = Some((d, 1));
    run.rung = gearmaster_engine::town::TOWNS[0].after + 1;
    let stack = run.road_stack();
    assert_eq!(stack.first().map(|i| i.kind()), Some("dungeon"));
    assert!(matches!(stack[0], Interrupt::Dungeon(_, 1)));
    assert!(!stack[0].blocks_a_rematch());
    assert!(stack[0].describe().contains("floor 2 of 3"));
}

#[test]
fn the_stack_says_what_it_holds_and_says_it_the_same_way_twice() {
    let mut run = a_run();
    for rung in 0..gearmaster_engine::combat::LADDER.len() {
        stand_at(&mut run, rung);
        for i in run.road_stack() {
            assert!(!i.kind().is_empty());
            assert!(!i.name().is_empty(), "an interrupt with no name at rung {}", rung + 1);
            assert!(i.describe().len() > 10, "{} does not explain itself", i.name());
        }
    }
}

#[test]
fn answering_an_event_takes_it_off_the_stack_for_good() {
    // Once per run, and a Grinder knock-back does not put it back. That is
    // `answered`, and the stack reads it rather than keeping a second list.
    let mut run = a_run();
    run.mode = Mode::Grinder;
    run.rung = 2;
    let before = run.road_stack();
    assert_eq!(before.len(), 1, "rung three has the toad's offer on it");
    let ev = run.pending_event().expect("the toad");
    let walk_on = ev.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").expect("authored");
    run.take_choice(walk_on);
    assert!(run.road_stack().is_empty());

    // Knocked back to it and it is still answered.
    run.rung = 1;
    run.rung = 2;
    assert!(run.road_stack().is_empty(), "an answered event came back");
}

#[test]
fn two_reads_of_the_same_road_are_the_same_road() {
    // E6.6, in the form the road can currently take: the whole ladder, twice,
    // from two runs built the same way. Push order comes from the tables, so
    // there is nothing seeded in here to drift.
    let (mut a, mut b) = (a_run(), a_run());
    for rung in 0..gearmaster_engine::combat::LADDER.len() {
        a.rung = rung;
        b.rung = rung;
        assert_eq!(a.road_stack(), b.road_stack(), "rung {}", rung + 1);
    }
}

#[test]
fn a_fight_an_event_arranged_stands_on_the_rung_too() {
    // The casino's table and the back room are not rungs and never move the
    // ladder, but they are between you and the rung's own creature, so they
    // are on the stack like everything else that is.
    let mut run = a_run();
    run.rung = 8;
    run.brawl = Some(&gearmaster_engine::event::TABLE_THREE);
    let stack = run.road_stack();
    assert_eq!(stack.last().map(|i| i.kind()), Some("brawl"));
    assert!(stack.last().unwrap().blocks_a_rematch());
}

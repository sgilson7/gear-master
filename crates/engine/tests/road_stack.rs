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

// ------------------------------------ what the strip promises, and the receipt

/// Two rumour doors on one rung both show in the stack.
///
/// `whispered_event` was a `find`, so the strip showed one. The second still
/// got asked - `standing_events` runs again after every answer - but nothing
/// could see it coming, and two doors resolving back to back reads as a bug
/// when the strip promised one.
#[test]
fn a_rung_carrying_two_words_says_it_carries_two() {
    let mut run = a_run();
    for r in gearmaster_engine::rumour::RUMOURS {
        run.give(r.name);
    }
    run.banked_all_run[gearmaster_engine::piece::Resource::Nature.index()] = 1_000;

    // Rung 23 carries the Green Ledger and the Locked Gate for a run holding
    // both words.
    run.rung = 22;
    let strip: Vec<&str> = run.road_stack().iter().map(|i| i.kind()).collect();
    assert!(
        strip.iter().filter(|k| **k == "event").count() >= 2,
        "the strip promised {:?}",
        strip
    );

    // And both are answerable, back to back, without the rung moving.
    let first = run.pending_event().expect("a door").id;
    let c = run.pending_event().unwrap().choices.iter().find(|c| run.choice_open(c)).unwrap();
    run.take_choice(c);
    run.take_receipt();
    let second = run.pending_event().expect("the second door").id;
    assert_ne!(first, second, "the same door twice");
}

/// The receipt says what a choice opened, not only what it cost.
///
/// A choice that unlocks a door and reports nothing but its price sends the
/// player back to the road thinking the answer was "go and fight the next
/// thing", which is the one reading that is wrong.
#[test]
fn a_receipt_names_the_door_it_opened() {
    // A word handed over says what it is a key to.
    let mut run = a_run();
    run.rung = 19;
    let e = run.pending_event().expect("the inspection");
    assert_eq!(e.id, "the-inspection");
    let c = e.choices.iter().find(|c| c.label == "Decline the inspection").unwrap();
    run.take_choice(c);
    let receipt = run.take_receipt().expect("a receipt").join(" | ");
    assert!(
        receipt.contains("THE PICKET LINE"),
        "the word was handed over and the receipt did not say what for: {}",
        receipt
    );

    // A flag set names the door that waits on it.
    let mut run = a_run();
    run.give("A Word About the Glow");
    run.rung = 44;
    let e = run.pending_event().expect("the glow");
    let c = e.choices.iter().find(|c| c.label == "Follow it").unwrap();
    run.take_choice(c);
    let receipt = run.take_receipt().expect("a receipt").join(" | ");
    assert!(receipt.contains("Opened:"), "a flag opened nothing anybody was told about: {}", receipt);
}

/// A town revealed behind you says so.
///
/// `describe` says "Revealed: X (after rung N)", which reads as good news even
/// when N is behind you and the road does not go back.
#[test]
fn a_town_revealed_too_late_says_it_is_behind_you() {
    let mut run = a_run();
    run.give("A Word About the Glow");
    run.rung = 44; // the Slagworks stands after rung 34
    let e = run.pending_event().expect("the glow");
    let c = e.choices.iter().find(|c| c.label == "Follow it").unwrap();
    run.take_choice(c);
    let receipt = run.take_receipt().expect("a receipt").join(" | ");
    assert!(receipt.contains("behind you"), "no warning at all: {}", receipt);
}

// ---------------------------------------------- two doors on one rung
//
// The bug the owner hit on rung three: a quick kill in the shallow end opens
// THE CASINO, whose window is rungs two to nine, and TWO BY TWO stands on rung
// three. `event::at` was a `find`, so the casino came back, the toad was never
// asked, and answering the casino left the rung empty. A scheduled event has
// one rung and no second chance.

/// Both stand, the one that expires is asked first, and neither is lost.
#[test]
fn an_earned_window_over_a_scheduled_rung_leaves_both_on_the_stack() {
    let mut run = a_run();
    run.best_fight_ms = Some(1_000);
    stand_at(&mut run, 2);

    let ids: Vec<&str> = run
        .road_stack()
        .iter()
        .filter_map(|i| match i {
            Interrupt::Event(e) => Some(e.id),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        vec!["the-toads-offer", "the-casino"],
        "rung three carries both; the toad expires here and the casino has seven more rungs"
    );
    assert_eq!(
        run.pending_event().map(|e| e.id),
        Some("the-toads-offer"),
        "the door about to be lost is the one asked"
    );
}

/// Answering the first leaves the second exactly where it stood.
///
/// Note the success test. `take_choice` returns `Option<&str>` and that option
/// is the *component handed over*, not whether the door was answered - most
/// choices hand over nothing and return `None` on the happy path. `answered`
/// is the fact.
#[test]
fn answering_one_of_two_on_a_rung_does_not_take_the_other_with_it() {
    let mut run = a_run();
    run.best_fight_ms = Some(1_000);
    stand_at(&mut run, 2);

    let toad = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == "the-toads-offer").unwrap();
    let fight_it = toad.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").expect("authored");
    run.take_choice(fight_it);
    assert!(run.answered.contains(&"the-toads-offer"), "the toad was not the door standing here");

    assert_eq!(
        run.pending_event().map(|e| e.id),
        Some("the-casino"),
        "the casino was underneath and is still underneath"
    );
    let ids: Vec<&str> = run
        .road_stack()
        .iter()
        .filter_map(|i| match i {
            Interrupt::Event(e) => Some(e.id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec!["the-casino"], "and the answered one is off the stack");
}

/// You answer the door you are standing at, and the one behind it waits.
#[test]
fn the_door_underneath_cannot_be_answered_over_the_top_of_the_one_in_front() {
    let mut run = a_run();
    run.best_fight_ms = Some(1_000);
    stand_at(&mut run, 2);
    let casino = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == "the-casino").unwrap();
    let out = casino.choices.iter().find(|c| c.label == "Keep out of it").expect("authored");

    // Refused while the toad is in front of it, which is the ownership guard
    // doing its job rather than a door being lost.
    run.take_choice(out);
    assert!(
        !run.answered.contains(&"the-casino"),
        "answered a door that was not the one being asked"
    );

    let toad = gearmaster_engine::event::EVENTS.iter().find(|e| e.id == "the-toads-offer").unwrap();
    run.take_choice(toad.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").unwrap());
    run.take_choice(out);
    assert!(
        run.answered.contains(&"the-casino") && run.answered.contains(&"the-toads-offer"),
        "both doors on rung three should end up answered, and neither should vanish"
    );
}

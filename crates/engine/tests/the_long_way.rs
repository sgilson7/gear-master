//! The other shallow-end door, and the pace you can learn at it.
//!
//! The two doors ask the same question - how is this run actually going - and
//! the failure that matters is the quiet one: a condition nothing can satisfy,
//! or two doors that both open when only one should.

use gearmaster_engine::class::{ClassPower, CLASSES};
use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side, LADDER};
use gearmaster_engine::event::{Outcome as ChoiceOutcome, EVENTS, SHALLOW};
use gearmaster_engine::run::{Mode, Run};

fn long_way() -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == "the-long-way").expect("authored")
}

fn slow_run() -> Run {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.worst_fight_ms = Some(12_000);
    run
}

#[test]
fn a_slow_win_in_the_shallow_end_opens_it() {
    let mut run = slow_run();
    assert_eq!(run.pending_event().map(|e| e.id), Some("the-long-way"));

    // Ten seconds is the line, and it is a floor rather than a ceiling.
    for (ms, open) in [(10_001u32, true), (10_000, false), (4_000, false)] {
        run.worst_fight_ms = Some(ms);
        assert_eq!(
            run.pending_event().map(|e| e.id) == Some("the-long-way"),
            open,
            "a {ms}ms win should {} the door",
            if open { "open" } else { "leave shut" }
        );
    }

    // And never before rung two.
    run.worst_fight_ms = Some(12_000);
    run.rung = 0;
    assert!(run.pending_event().map(|e| e.id) != Some("the-long-way"));
}

#[test]
fn the_casino_shuts_this_door_behind_it() {
    // Both earned: a run with one fast fight and one slow one qualifies for
    // each. The casino is offered, and answering it settles the question.
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(2_000);
    run.worst_fight_ms = Some(12_000);
    assert_eq!(
        run.pending_event().map(|e| e.id),
        Some("the-casino"),
        "with both earned, the casino is the one asked"
    );

    let ev = run.pending_event().expect("open");
    let walk = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Give(_)))
        .expect("the walk-away branch");
    run.take_choice(walk);

    assert!(
        run.pending_event().is_none(),
        "the long way opened after the casino was answered - they are alternatives"
    );
}

#[test]
fn the_long_way_alone_opens_when_the_casino_was_never_earned() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(9_000); // nowhere near quick enough
    run.worst_fight_ms = Some(12_000);
    assert_eq!(run.pending_event().map(|e| e.id), Some("the-long-way"));
}

#[test]
fn asking_how_it_manages_costs_nothing_and_is_remembered() {
    let mut run = slow_run();
    let ev = run.pending_event().expect("open");
    let ask = ev.choices.iter().find(|c| c.label.starts_with("Ask")).expect("the free branch");
    let (gold, classes, owned) = (run.gold, run.classes.len(), run.owned.len());
    run.take_choice(ask);

    assert_eq!(run.gold, gold, "the free branch charged for something");
    assert_eq!(run.classes.len(), classes, "the free branch handed over a class");
    assert_eq!(run.owned.len(), owned, "the free branch handed over a component");
    // What it does hand over is a note for later.
    assert!(run.took.contains(&ask.label), "nothing was remembered, so nothing can follow it");
}

#[test]
fn walking_with_it_hands_over_trundle() {
    let mut run = slow_run();
    let ev = run.pending_event().expect("open");
    let walk = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Claim("Trundle")))
        .expect("the class branch");
    run.take_choice(walk);
    assert!(run.classes.iter().any(|c| c.name == "Trundle"));
}

#[test]
fn no_fountain_offers_trundle() {
    assert!(gearmaster_engine::class::is_earned("Trundle"));
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    assert!(run.class_outlook().iter().all(|m| m.class.name != "Trundle"));
}

/// A board that both swings and picks up armour, so both halves of the trade
/// have something to act on.
fn a_trundling_run() -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.apply_preset();
    run
}

#[test]
fn trundle_halves_the_turns_and_doubles_the_wall() {
    let run = a_trundling_run();
    let (stats, items) = (run.player_stats(), run.combat_items());
    let trundle = *CLASSES.iter().find(|c| c.name == "Trundle").expect("authored");
    let ClassPower::Trundle { slower, armour } = trundle.power else {
        panic!("Trundle is not a Trundle");
    };
    assert_eq!((slower, armour), (50, 200));

    // A shallow rung, where the build survives long enough either way for its
    // armour to come round. Deeper in, the trundled run dies before its
    // slowest plate fires and there is nothing to compare - which is a real
    // property of the class, not a measurement problem, but it is not what
    // this test is about.
    let spec = LADDER[2];
    let read = |classes: &[gearmaster_engine::class::ClassDef]| -> (usize, Vec<i32>, u32) {
        let log = simulate_with_class(stats, &items, &spec, Difficulty::Medium, classes);
        let acts = log
            .entries
            .iter()
            .filter(|e| matches!(e.event, Event::Activate { side: Side::Player, .. }))
            .count();
        let armour = log
            .entries
            .iter()
            .filter_map(|e| match &e.event {
                Event::GainArmor { side: Side::Player, amount, .. } => Some(*amount),
                _ => None,
            })
            .collect();
        (acts, armour, log.duration_ms)
    };

    let (acts, plates, ms) = read(&[]);
    let (slow_acts, slow_plates, slow_ms) = read(&[trundle]);
    assert!(!plates.is_empty(), "the control never put any armour on; this proves nothing");

    // Every plate is worth exactly twice as much.
    assert_eq!(
        slow_plates,
        plates.iter().map(|a| a * 2).collect::<Vec<_>>(),
        "armour is not doubled"
    );
    // And it takes exactly twice as long to do the same work.
    assert_eq!(slow_acts, acts, "the same activations should still happen, just later");
    assert!(
        slow_ms >= ms * 2 - 200,
        "the fight should take about twice as long: {slow_ms}ms against {ms}ms"
    );
}

/// What the trade actually works out to, written down because it is not what
/// it looks like.
///
/// Half the activations, each plate worth double, means armour *per second* is
/// unchanged - while damage per second is halved. Trundle is a slower run at
/// the same wall, not a tougher one. Recorded as a test so that if the numbers
/// are ever retuned, whoever does it sees what the old ones came to.
#[test]
fn trundle_leaves_the_wall_where_it_was_and_halves_everything_else() {
    let run = a_trundling_run();
    let (stats, items) = (run.player_stats(), run.combat_items());
    let trundle = *CLASSES.iter().find(|c| c.name == "Trundle").expect("authored");

    let per_second = |classes: &[gearmaster_engine::class::ClassDef]| -> (f32, f32) {
        let log = simulate_with_class(stats, &items, &LADDER[2], Difficulty::Medium, classes);
        let secs = (log.duration_ms.max(1) as f32) / 1000.0;
        let armour: i32 = log
            .entries
            .iter()
            .filter_map(|e| match &e.event {
                Event::GainArmor { side: Side::Player, amount, .. } => Some(*amount),
                _ => None,
            })
            .sum();
        let hits: i32 = log
            .entries
            .iter()
            .filter_map(|e| match &e.event {
                Event::Hit { by: Side::Player, damage, .. } => Some(*damage),
                _ => None,
            })
            .sum();
        (armour as f32 / secs, hits as f32 / secs)
    };

    let (armour, damage) = per_second(&[]);
    let (slow_armour, slow_damage) = per_second(&[trundle]);
    assert!(
        (slow_armour - armour).abs() < armour * 0.2,
        "armour per second moved: {slow_armour:.1} against {armour:.1}"
    );
    assert!(
        slow_damage < damage * 0.7,
        "damage per second should be roughly halved: {slow_damage:.1} against {damage:.1}"
    );
}

#[test]
fn the_shallow_window_is_what_both_doors_watch() {
    // Both doors, one window, and it is the one the run records fights in.
    for id in ["the-casino", "the-long-way"] {
        let e = EVENTS.iter().find(|e| e.id == id).expect("authored");
        assert!(SHALLOW.contains(&e.trigger.from()), "{id} starts outside the shallow end");
        assert!(SHALLOW.contains(&e.at), "{id} ends outside the shallow end");
    }
    assert_eq!(long_way().at, 8);
}

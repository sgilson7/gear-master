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
    run.worst_fight_ms = Some(22_000);
    run
}

#[test]
fn a_slow_win_in_the_shallow_end_opens_it() {
    let mut run = slow_run();
    assert_eq!(run.pending_event().map(|e| e.id), Some("the-long-way"));

    // Fifteen seconds is the line, and it is a floor rather than a ceiling.
    //
    // It was ten, then twenty, and the reason for twenty was written down: "a
    // creature carries a piece a rung now, so ten seconds stopped meaning this
    // run is grinding". That is no longer true where this door is asked. The
    // density curve is deliberately **flat** across rungs 1-10 - four or five
    // pieces, to keep the casino reachable - and a piece a rung only starts
    // above that. So the shallow end got lighter again and the line came back
    // down with it.
    //
    // Fifteen because it is measured. A board blunted until it grinds takes
    // 18.0s at its slowest down there; a sharp board takes 8.0s. Anything
    // between those two separates them, and twenty separates nothing because
    // nothing reaches it.
    for (ms, open) in [(15_001u32, true), (15_000, false), (4_000, false)] {
        run.worst_fight_ms = Some(ms);
        assert_eq!(
            run.pending_event().map(|e| e.id) == Some("the-long-way"),
            open,
            "a {ms}ms win should {} the door",
            if open { "open" } else { "leave shut" }
        );
    }

    // And never before rung two.
    run.worst_fight_ms = Some(22_000);
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
    run.worst_fight_ms = Some(22_000);
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
    run.worst_fight_ms = Some(22_000);
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
fn trundle_slows_the_turns_and_doubles_the_wall() {
    let run = a_trundling_run();
    let (stats, items) = (run.player_stats(), run.combat_items());
    let trundle = *CLASSES.iter().find(|c| c.name == "Trundle").expect("authored");
    let ClassPower::Trundle { slower, armour } = trundle.power else {
        panic!("Trundle is not a Trundle");
    };
    assert_eq!((slower, armour), (25, 200));

    // A shallow rung, where the build survives long enough either way for its
    // armour to come round.
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
    //
    // Compared over as much as the two runs have in common rather than end to
    // end. Trundle's claim is about what a plate is *worth*; how many of them
    // there are is a property of how long the fight happens to run, and that is
    // not fixed - rung three gained a chest item and stopped dying at the same
    // moment in both runs. Requiring the counts to match was pinning the
    // fixture, not the class.
    let n = slow_plates.len().min(plates.len());
    assert!(n >= 8, "only {n} plates in common; the fixture is too short to say anything");
    assert_eq!(
        slow_plates[..n],
        plates[..n].iter().map(|a| a * 2).collect::<Vec<_>>()[..],
        "armour is not doubled"
    );
    // And the work comes slower, which is the half of the trade being paid.
    //
    // Was `slow_acts == acts` - the same turns, just later - which held while
    // the fight was short enough for the build to get through its whole cycle
    // either way. It is not any more, so the honest reading is the rate: a
    // quarter slower is fewer turns in the same second, however many seconds
    // there turn out to be.
    let rate = |a: usize, ms: u32| a as f64 / (ms.max(1) as f64 / 1000.0);
    assert!(
        rate(slow_acts, slow_ms) < rate(acts, ms),
        "slowed to {:.2} turns a second against {:.2} - that is not slower",
        rate(slow_acts, slow_ms),
        rate(acts, ms)
    );
    assert!(
        slow_ms > ms && slow_ms < ms * 2,
        "a quarter slower should stretch the fight, not double it: {slow_ms}ms against {ms}ms"
    );
}

/// What the trade works out to, now that it is one.
///
/// At a fifty percent slowdown it was not: half the activations for plates
/// worth double left armour per second exactly where it was and halved
/// everything else, which is a tax wearing a trade's clothes. At twenty-five
/// the wall goes up by about half while damage comes down by about a quarter -
/// a decision, and one a build that is being out-damaged might well make.
#[test]
fn trundle_buys_wall_with_tempo() {
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
    println!(
        "trundle: armour/s {armour:.1} -> {slow_armour:.1}, damage/s {damage:.1} -> {slow_damage:.1}"
    );
    assert!(
        slow_armour > armour * 1.2,
        "the wall barely moved: {slow_armour:.1} against {armour:.1} a second"
    );
    assert!(
        slow_damage < damage,
        "it should still cost tempo: {slow_damage:.1} against {damage:.1} a second"
    );
    assert!(
        slow_damage > damage * 0.6,
        "a quarter slower should not halve the damage: {slow_damage:.1} against {damage:.1}"
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

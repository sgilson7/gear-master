//! Events that turn up because of something you did.
//!
//! A scheduled event is easy to be sure of: it is on rung fifteen or it is a
//! bug. An earned one has a window and a condition, and the failure that
//! matters is the quiet one - a condition nothing can ever satisfy, so the
//! event simply never happens and nobody can tell it was meant to.

use gearmaster_engine::combat::LADDER;
use gearmaster_engine::event::{Requirement, Trigger, EVENTS};
use gearmaster_engine::run::Run;

#[test]
fn every_event_stands_on_the_creature_it_names() {
    for e in EVENTS {
        let at = LADDER.get(e.at).unwrap_or_else(|| panic!("{}: rung {} is off the end", e.id, e.at));
        assert_eq!(
            at.name, e.expects,
            "{} stands on rung {} expecting {} but the ladder has {} there",
            e.id, e.at, e.expects, at.name
        );
    }
}

#[test]
fn an_earned_event_needs_the_thing_that_earns_it() {
    let earned: Vec<&str> = EVENTS
        .iter()
        .filter(|e| !matches!(e.trigger, Trigger::Rung))
        .map(|e| e.id)
        .collect();
    if earned.is_empty() {
        return; // none authored yet
    }
    for id in earned {
        let e = EVENTS.iter().find(|e| e.id == id).expect("just listed");
        let Trigger::QuickKill { within_ms } = e.trigger else { continue };
        assert!(within_ms > 0, "{id}: a window of zero can never be met");

        // Inside the window and quick enough: it stands in front of you.
        let mut run = Run::with_all_pieces();
        run.rung = e.at;
        run.best_fight_ms = Some(within_ms);
        assert_eq!(
            run.pending_event().map(|p| p.id),
            Some(id),
            "{id}: earned it and it did not turn up"
        );

        // Quick enough, but past the last rung it stands on.
        run.rung = e.at + 1;
        assert!(run.pending_event().map(|p| p.id) != Some(id), "{id}: turned up too late");

        // Inside the window, never quick enough.
        run.rung = e.at;
        run.best_fight_ms = Some(within_ms + 1);
        assert!(
            run.pending_event().map(|p| p.id) != Some(id),
            "{id}: turned up without being earned"
        );

        // No win at all.
        run.best_fight_ms = None;
        assert!(run.pending_event().map(|p| p.id) != Some(id), "{id}: turned up before any win");
    }
}

#[test]
fn holding_a_component_opens_a_door_without_spending_it() {
    let mut run = Run::with_all_pieces();
    let name = run.registry.def(run.owned[0]).name;
    let before = run.owned.len();

    let open = gearmaster_engine::event::Choice {
        label: "in",
        blurb: "",
        requires: Requirement::Holding(name),
        outcome: gearmaster_engine::event::Outcome::FightAsWritten,
        unmet: "you have not got one",
    };
    assert!(run.choice_open(&open), "holding {name} should open the door");

    let shut = gearmaster_engine::event::Choice {
        requires: Requirement::Holding("Not A Real Component"),
        ..open
    };
    assert!(!run.choice_open(&shut), "a component nobody owns should not open it");

    // The key is not the toll: it is still yours afterwards.
    assert_eq!(run.owned.len(), before);
    assert!(run.owned.iter().any(|&i| run.registry.def(i).name == name));
}

#[test]
fn the_quickest_win_is_what_gets_remembered() {
    use gearmaster_engine::combat::Difficulty;
    use gearmaster_engine::run::Mode;

    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    assert_eq!(run.best_fight_ms, None, "no wins yet");

    // Wear something that flattens the first few rungs quickly.
    for name in ["Oak Handle", "Iron Blade", "Adamant Base", "Riveted Layer"] {
        let Some(id) = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == name && !run.is_equipped(i))
        else {
            continue;
        };
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
    }

    let mut seen: Vec<u32> = Vec::new();
    for rung in 0..6usize {
        run.rung = rung;
        let outcome = run.fight_next().outcome;
        let ms = run.log.as_ref().map(|l| l.duration_ms).unwrap_or(0);
        if outcome == gearmaster_engine::combat::Outcome::Victory {
            seen.push(ms);
        }
        run.settle();
    }
    assert!(!seen.is_empty(), "won nothing in six rungs of the shallow end");
    assert_eq!(
        run.best_fight_ms,
        seen.iter().copied().min(),
        "the run should remember its quickest win, not its latest"
    );
}

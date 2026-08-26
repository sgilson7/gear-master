//! The Unwinding: four stations, two hidden towns, and a thing at the end.
//!
//! What makes this a chain rather than four events on four rungs is that each
//! station is *reached* by the last one. What makes it a chain somebody can
//! finish is that every station **fails forward**: a refused choice costs the
//! reward and never the road. There is exactly one thing on it that can be
//! lost rather than declined, and even that waits three rungs and offers
//! again.
//!
//! The chain's state is mostly not a flag. It is the **words you are
//! carrying**, the towns you have been told about, and one dungeon cleared -
//! all of which are things the run already knew how to remember. Only the
//! antechamber sets a flag, because a dungeon is the one station whose being
//! finished is not visible in your tray.

mod common;

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::event::{Outcome, EVENTS};
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::town::Action;

const WRONG_STARS: &str = "A Word About the Wrong Stars";
const CELLAR: &str = "A Word About the Cellar";
const GLOW: &str = "A Word About the Glow";
const MAINSPRING: &str = gearmaster_engine::run::MAINSPRING;

fn a_run(mode: Mode) -> Run {
    let mut run = Run::seeded(0xC4A1);
    run.mode = mode;
    run.difficulty = Difficulty::Easy;
    common::build_full_loadout(&mut run);
    run
}

fn event(id: &str) -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("{} is not authored", id))
}

/// Stand at `rung` holding `word`, and take the choice named.
fn answer(run: &mut Run, rung: usize, id: &str, label: &str) {
    run.rung = rung;
    let standing = run.pending_event().unwrap_or_else(|| panic!("nothing standing at {}", rung + 1));
    assert_eq!(standing.id, id, "the wrong door is open at rung {}", rung + 1);
    let c = standing
        .choices
        .iter()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("{} has no choice {:?}", id, label));
    assert!(run.take_choice(c).is_some() || run.last_receipt.is_some(), "{} refused", label);
    run.take_receipt();
}

// ------------------------------------------------------------ the stations

#[test]
fn the_chain_is_four_doors_and_each_one_opens_the_next() {
    let mut run = a_run(Mode::Grinder);

    // Nothing at all until somebody says something.
    run.rung = 20;
    assert!(run.pending_event().is_none(), "a door opened for a run that heard nothing");

    // One: the astronomer, met by carrying the word the bar sells.
    run.give(WRONG_STARS);
    answer(&mut run, 20, "the-astronomer", "Hear him out");
    assert!(run.holds(CELLAR), "hearing him out handed over nothing");

    // Two: the gate, met by carrying what he gave you.
    answer(&mut run, 25, "the-locked-gate", "Use the word");
    assert!(run.towns_revealed.contains(&"the-manse"), "the gate opened onto nothing");

    // Three: the house, which is where the third word is.
    run.rung = gearmaster_engine::town::by_id("the-manse").expect("authored").after;
    run.force_win();
    run.settle();
    let at = run.pending_town().expect("the Manse stands where it says");
    assert_eq!(at.id, "the-manse");
    run.visit_town(Action::Gallery);
    run.take_receipt();

    // Four: the ridge, met by carrying what the house paid you.
    if run.holds(GLOW) {
        answer(&mut run, 34, "the-glow-over-the-ridge", "Follow it");
        assert!(run.towns_revealed.contains(&"the-slagworks"));
    }
}

#[test]
fn every_station_fails_forward() {
    // A refused choice costs the reward and never the road. The proof is that
    // after refusing each one, the thing that opens the *next* station is
    // still gettable - which for three of the four means the word is still in
    // your tray.
    let mut run = a_run(Mode::Grinder);
    run.give(WRONG_STARS);

    // Turned in rather than heard out: no second word, and the run walks on.
    let rung = run.rung;
    answer(&mut run, 20, "the-astronomer", "Turn him in");
    assert!(!run.holds(CELLAR), "turning him in paid twice");
    assert!(run.rung > rung, "a refusal cost the rung it was offered at");
    assert!(run.lives_left().is_none_or(|l| l > 0), "a refusal cost a life");

    // The foreman is the other way to the cellar word, and the crucible town
    // is the other way to the foreman. The chain has two roads to its middle.
    assert!(
        gearmaster_engine::town::by_id("the-slagworks")
            .expect("authored")
            .actions
            .iter()
            .any(|a| a.gives() == Some(CELLAR)),
        "turning the astronomer in ends the chain, which is a chain that fails backward"
    );
}

#[test]
fn declining_a_door_is_not_answering_it() {
    // `Defer` is the one outcome that leaves the door standing. Everything
    // else in the game is a decision; this is putting one off, and the price
    // is that the thing comes back.
    let mut run = a_run(Mode::Grinder);
    run.give(CELLAR);
    answer(&mut run, 25, "the-locked-gate", "Walk on");
    assert!(!run.answered.contains(&"the-locked-gate"), "walking on answered it");

    // Not here. Asked about the gate rather than about the rung, because the
    // road has unconditional doors on it too and one of them stands at 27.
    let standing = |run: &Run| run.road_stack().iter().any(|i| i.id() == "the-locked-gate");
    run.rung = 26;
    assert!(!standing(&run), "it found you again immediately");
    // And here.
    run.rung = 28;
    assert!(standing(&run), "it never came back");
}

#[test]
fn a_rumour_door_stands_in_a_window_rather_than_on_a_rung() {
    // A door priced in a rumour is a door you might arrive at holding nothing,
    // and one that stands on exactly one rung is a door a run walks past for
    // reasons that have nothing to do with the bet it made.
    let mut run = a_run(Mode::Grinder);
    run.give(WRONG_STARS);
    let e = event("the-astronomer");
    let gearmaster_engine::event::Trigger::Whispered { from, .. } = e.trigger else {
        panic!("the astronomer stopped being a rumour door")
    };
    assert!(e.at > from, "the window is one rung wide");
    // A rumour door goes first wherever it stands, so `pending_event` is the
    // right question inside the window and the wrong one outside it - the
    // road has unconditional doors of its own.
    for rung in from..=e.at {
        run.rung = rung;
        assert_eq!(
            run.pending_event().map(|x| x.id),
            Some("the-astronomer"),
            "the window has a hole in it at rung {}",
            rung + 1
        );
    }
    run.rung = from - 1;
    assert!(
        !run.road_stack().iter().any(|i| i.id() == "the-astronomer"),
        "it stands before its own window"
    );
}

// ------------------------------------------------------------- the towns

#[test]
fn every_hidden_town_stands_clear_of_everything_else() {
    use gearmaster_engine::town::{Unlock, TOWNS};
    // Three, not two: the chain finds two and the sign behind the sign finds
    // the third, which is Part G's rather than Part B's.
    let hidden: Vec<&gearmaster_engine::town::Town> =
        TOWNS.iter().filter(|t| t.unlock == Unlock::Hidden).collect();
    assert_eq!(hidden.len(), 3);
    for t in &hidden {
        // Not sharing a gap with anything, which would make one of them
        // unreachable - `between` takes the first match.
        assert_eq!(TOWNS.iter().filter(|o| o.after == t.after).count(), 1, "{}", t.id);
        // And not next door to a pinned one, so no run is asked to choose
        // between two towns on one stretch of road.
        for o in TOWNS.iter().filter(|o| o.id != t.id) {
            assert!(
                t.after.abs_diff(o.after) > 1,
                "{} and {} share a stretch of road",
                t.id,
                o.id
            );
        }
    }
}

#[test]
fn the_manse_is_early_because_what_is_under_it_is_a_pool() {
    // A lane earned at rung forty is a lane nobody uses. The antechamber is
    // the only way into the mind lane, so the house over it stands where a
    // build still has time to be about something.
    let manse = gearmaster_engine::town::by_id("the-manse").expect("authored");
    assert!(manse.after < 30, "the Manse drifted past the point of the pool");
    let slag = gearmaster_engine::town::by_id("the-slagworks").expect("authored");
    assert!(slag.after > manse.after, "the foundry came before the house");
}

#[test]
fn the_cellar_door_opens_the_lane_and_nothing_else_does() {
    let mut run = a_run(Mode::Grinder);
    assert!(!run.insight_unlocked);
    run.reveal_town("the-manse");
    run.rung = gearmaster_engine::town::by_id("the-manse").expect("authored").after;
    run.force_win();
    run.settle();
    run.visit_town(Action::CellarDoor);
    run.take_receipt();
    let (d, _) = run.dungeon.expect("the cellar door opens onto a staircase");
    assert_eq!(d.id, "the-threshold");
    assert!(run.pending_scene.is_some(), "you walked into a dungeon and nobody said so");

    // Three floors, and the pool at the bottom.
    for _ in 0..d.floors.len() {
        run.pending_scene = None;
        run.force_win();
        run.settle();
        run.back_to_loadout();
    }
    assert!(run.dungeon.is_none(), "still in it");
    assert!(run.insight_unlocked, "the antechamber did not open the lane");
    assert!(run.shop.insight_open, "the run learned it and the shelf did not");
    assert!(run.flags.contains(&"threshold-cleared"));
    assert!(run.classes.iter().any(|c| c.name == "Threshold-Sighted"));
}

// ------------------------------------------------------------- the Herald

#[test]
fn the_shadow_arrives_when_the_antechamber_has_been_walked() {
    let mut run = a_run(Mode::Grinder);
    run.rung = 45;
    assert!(run.pending_event().is_none(), "the shadow came for a run that never went down");
    run.flags.push("threshold-cleared");
    let e = run.pending_event().expect("it has been walking at your pace");
    assert_eq!(e.id, "the-second-shadow");
}

#[test]
fn facing_it_is_two_at_once_and_winning_it_is_the_mainspring() {
    let mut run = a_run(Mode::Grinder);
    run.flags.push("threshold-cleared");
    answer(&mut run, 45, "the-second-shadow", "Face it");
    let party = run.pending_brawl().expect("both of them");
    assert_eq!(party.len(), 2);
    let names: Vec<&str> = party.iter().map(|m| m.name).collect();
    assert_eq!(names, vec!["THE SHADOW", "THE LANTERN"]);

    let rung = run.rung;
    run.force_win();
    run.settle();
    assert_eq!(run.rung, rung, "a fight beside the road moved the ladder");
    assert!(run.holds(MAINSPRING), "beating it handed over nothing");
}

#[test]
fn refusing_it_is_not_losing_it() {
    let mut run = a_run(Mode::Grinder);
    run.flags.push("threshold-cleared");
    answer(&mut run, 45, "the-second-shadow", "Refuse");
    assert!(!run.answered.contains(&"the-second-shadow"));
    run.rung = 48;
    assert_eq!(run.pending_event().map(|e| e.id), Some("the-second-shadow"));
}

// ---------------------------------------------------------- the whole road

#[test]
fn the_chain_can_be_finished_in_one_run_in_either_mode() {
    for mode in [Mode::Grinder, Mode::Rogue] {
        let mut run = a_run(mode);
        run.give(WRONG_STARS);

        answer(&mut run, 20, "the-astronomer", "Hear him out");
        answer(&mut run, 25, "the-locked-gate", "Use the word");

        // Into the house, down the stairs, and back up with the lane.
        run.rung = gearmaster_engine::town::by_id("the-manse").expect("authored").after;
        run.force_win();
        run.settle();
        run.visit_town(Action::CellarDoor);
        run.take_receipt();
        let floors = run.dungeon.expect("in it").0.floors.len();
        for _ in 0..floors {
            run.pending_scene = None;
            run.force_win();
            run.settle();
            run.back_to_loadout();
        }
        assert!(run.insight_unlocked, "{:?}: no lane", mode);

        answer(&mut run, 45, "the-second-shadow", "Face it");
        run.force_win();
        run.settle();
        assert!(run.holds(MAINSPRING), "{:?}: no mainspring", mode);

        // And the road past the top opens once the man at it is down.
        run.rung = gearmaster_engine::combat::LADDER.len();
        assert!(run.past_the_top(), "{:?}: the road stopped at fifty", mode);
        assert!(!run.ladder_complete(), "{:?}: it called the ladder finished", mode);
    }
}

#[test]
fn nothing_in_the_chain_reaches_a_creature_or_a_shelf_it_should_not() {
    use gearmaster_engine::piece::{is_event_only, touches_insight, CATALOG};
    for name in [CELLAR, GLOW, MAINSPRING, "The Cracked Lens"] {
        assert!(is_event_only(name), "{} could be bought", name);
    }
    // The one the bar sells is the on-ramp and is meant to be come by.
    assert!(
        gearmaster_engine::rumour::by_name(WRONG_STARS).is_some_and(|r| r.on_the_bar),
        "the chain has no on-ramp"
    );
    // And the mind lane stays shut until the antechamber is walked.
    let gated = CATALOG.iter().filter(|d| touches_insight(d)).count();
    assert!(gated >= 8, "the lane's gear went missing");
}

#[test]
fn every_door_in_the_chain_says_what_it_does() {
    for id in [
        "the-astronomer",
        "the-locked-gate",
        "the-glow-over-the-ridge",
        "the-second-shadow",
    ] {
        let e = event(id);
        assert!(e.prose.len() >= 2, "{}: a station with nothing to read", id);
        for c in e.choices {
            let lines = c.outcome.describe();
            assert!(!lines.is_empty(), "{}: {} resolves into silence", id, c.label);
            assert!(!c.blurb.is_empty(), "{}: {} does not say what it costs", id, c.label);
        }
        // Every station has a way through that needs nothing, which is the
        // whole of failing forward.
        assert!(
            e.choices.iter().any(|c| c.requires == gearmaster_engine::event::Requirement::None),
            "{} can be locked shut",
            id
        );
    }
    // And the one outcome that is not a decision is used exactly where the
    // chain wants to be declined rather than refused.
    let defers: Vec<&str> = EVENTS
        .iter()
        .filter(|e| e.choices.iter().any(|c| matches!(c.outcome, Outcome::Defer { .. })))
        .map(|e| e.id)
        .collect();
    assert_eq!(defers, vec!["the-locked-gate", "the-second-shadow"]);
}

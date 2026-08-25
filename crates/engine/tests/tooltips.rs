//! Everything on the road explains itself, in the engine's own words.
//!
//! Three small systems with one idea behind them: **the engine owns the
//! sentence**. A greyed choice, a rumour in the tray and a receipt after a
//! resolution are all the game saying what just happened or what would have to
//! be true - and if the interface writes those sentences, the CLI has to write
//! them again, the theme layer has nothing to swap, and the three drift.
//!
//! So `Requirement::describe`, `Outcome::describe`, `TownVisit::receipt` and
//! `rumour::conditions_line` all return canonical prose, and the display layer
//! swaps the nouns on the way out.
//!
//! The distinction that matters, and the one that is easy to lose: `unmet` is
//! flavour for the moment *after* you have tried a door - "Merrik does not move
//! the rope, and has not moved it in eleven years" - and `describe` is the
//! plain statement *before* you try. Both ship, and neither replaces the other.

mod common;

use gearmaster_engine::event::{Outcome, Requirement, EVENTS, TABLE_THREE};
use gearmaster_engine::run::Run;
use gearmaster_engine::town::Action;

// --------------------------------------------------------- what a door wants

#[test]
fn a_requirement_says_what_would_open_it() {
    assert_eq!(Requirement::None.describe(), "");
    assert_eq!(
        Requirement::LooseItemOfSize { w: 2, h: 2 }.describe(),
        "Requires: a loose component 2 by 2"
    );
    assert!(Requirement::Holding("Platinum Chip").describe().contains("Platinum Chip"));
    assert!(Requirement::Took("Ask how he does it").describe().contains("Ask how he does it"));
}

#[test]
fn every_locked_choice_in_the_game_can_say_why() {
    // The lint the tooltip buys. A door with a requirement and no sentence is
    // a door somebody has to guess at.
    for e in EVENTS {
        for c in e.choices {
            if c.requires == Requirement::None {
                continue;
            }
            let said = c.requires.describe();
            assert!(!said.is_empty(), "{}: {} cannot say what it wants", e.id, c.label);
            assert!(said.starts_with("Requires: "), "{}: {}", e.id, said);
            // And the flavour line is still there, which is a different job.
            assert!(!c.unmet.is_empty(), "{}: {} lost its prose", e.id, c.label);
        }
    }
}

// ------------------------------------------------------ what a choice hands over

#[test]
fn every_outcome_in_the_game_lists_its_deltas() {
    for e in EVENTS {
        for c in e.choices {
            let lines = c.outcome.describe();
            assert!(!lines.is_empty(), "{}: {} resolves into silence", e.id, c.label);
            for l in &lines {
                assert!(!l.trim().is_empty(), "{}: {} has an empty receipt line", e.id, c.label);
            }
        }
    }
}

#[test]
fn a_fight_an_event_arranges_says_what_is_at_stake_and_what_losing_costs() {
    let lines = Outcome::Step(&TABLE_THREE).describe();
    let all = lines.join(" | ");
    assert!(all.contains("Bone Archer") && all.contains("Frost Wisp"));
    assert!(all.contains("Platinum Chip"), "the reason to step in is not stated");
    assert!(all.contains("no life"), "the reason it is safe to step in is not stated");
}

#[test]
fn a_curated_shop_names_what_is_on_the_table_and_what_it_costs() {
    let vip = EVENTS.iter().find(|e| e.id == "the-vip-area").expect("authored");
    let keep = vip.choices.iter().find(|c| c.label == "Keep your face still").expect("authored");
    let all = keep.outcome.describe().join(" | ");
    assert!(all.contains("Overseer's Circlet"));
    assert!(all.contains("Immense Guilt"), "a cost that is not on the receipt is a cost nobody sees");
}

// -------------------------------------------------------------- the receipt

#[test]
fn answering_a_door_leaves_a_receipt_with_the_run_s_own_numbers_in_it() {
    let mut run = Run::seeded(0x7001);
    common::build_full_loadout(&mut run);
    run.rung = 2;
    let ev = run.pending_event().expect("the toad's offer");
    let fight = ev.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").expect("authored");
    assert!(run.last_receipt.is_none(), "a fresh run is not holding a receipt");
    run.take_choice(fight);
    let got = run.take_receipt().expect("answering something leaves a receipt");
    assert!(!got.is_empty());
    assert!(run.last_receipt.is_none(), "a receipt is read once and dismissed");
}

#[test]
fn buying_a_rung_off_says_what_it_paid_rather_than_how_many_times_over() {
    // "the bounty two times over" is the table. "+18g" is what happened, and
    // the receipt is the second thing.
    let mut run = Run::seeded(0x1234);
    common::build_full_loadout(&mut run);
    run.rung = 2;
    let ev = run.pending_event().expect("the toad's offer");
    let deal = ev.choices.iter().find(|c| c.label == "TAKE THE DEAL").expect("authored");
    if !run.choice_open(deal) {
        // No 2x2 in the tray, which is a fixture accident and not the subject.
        return;
    }
    run.take_choice(deal);
    let got = run.take_receipt().expect("a receipt");
    assert!(got.iter().any(|l| l.contains('g')), "no gold figure in {:?}", got);
    assert!(got.iter().any(|l| l.starts_with("Handed over: ")), "{:?}", got);
}

#[test]
fn a_town_door_leaves_a_receipt_too() {
    let mut run = Run::seeded(0x99);
    common::build_full_loadout(&mut run);
    run.rung = gearmaster_engine::town::TOWNS[0].after;
    run.force_win();
    run.settle();
    assert!(run.town.is_some(), "the fixture did not reach the gate");
    let visit = run.visit_town(Action::Chapel);
    let got = run.take_receipt().expect("a visit leaves a receipt");
    assert_eq!(got, visit.receipt());
    assert!(got.iter().any(|l| l.contains("Piety")), "{:?}", got);
}

#[test]
fn walking_past_a_town_leaves_one_as_well() {
    let mut run = Run::seeded(0x99);
    common::build_full_loadout(&mut run);
    run.rung = gearmaster_engine::town::TOWNS[0].after;
    run.force_win();
    run.settle();
    let paid = run.skip_town();
    let got = run.take_receipt().expect("walking on is a resolution like any other");
    assert!(got.iter().any(|l| l.contains(&format!("+{}g", paid))), "{:?}", got);
}

// ------------------------------------------------------ what a rumour is for

#[test]
fn a_rumour_says_which_door_it_is_a_key_to_and_where_that_door_stands() {
    for r in gearmaster_engine::rumour::RUMOURS {
        let line = gearmaster_engine::rumour::conditions_line(r.name)
            .unwrap_or_else(|| panic!("{} says nothing about itself", r.name));
        assert!(line.starts_with("Conditions: "));
        assert!(line.contains("rung"), "{}: {}", r.name, line);
        // And it is still not the answer. The hint is the puzzle; this is the
        // address.
        assert!(
            !line.contains(&r.needs.describe()),
            "{} gives its own condition away in the reverse index",
            r.name
        );
    }
}

#[test]
fn an_event_knows_whether_it_stands_on_a_rung_or_roams_a_window() {
    let casino = EVENTS.iter().find(|e| e.id == "the-casino").expect("authored");
    assert!(casino.where_it_stands().contains("to"), "an earned event roams: {}", casino.where_it_stands());
    let toad = EVENTS.iter().find(|e| e.id == "the-toads-offer").expect("authored");
    assert_eq!(toad.where_it_stands(), "rung 3");
}

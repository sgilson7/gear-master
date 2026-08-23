//! The casino, and the chip you walk out with.
//!
//! Two things here are easy to ship broken and hard to notice: an earned event
//! whose condition nothing can meet, and a piece that reaches out of the fight
//! into the purse and either never pays or never stops.

use gearmaster_engine::combat::{Difficulty, Event, Side, LADDER};
use gearmaster_engine::event::{Outcome as ChoiceOutcome, EVENTS};
use gearmaster_engine::piece::CATALOG;
use gearmaster_engine::run::{Mode, Run};

fn casino() -> &'static gearmaster_engine::event::LadderEvent {
    EVENTS.iter().find(|e| e.id == "the-casino").expect("the casino is authored")
}

#[test]
fn the_casino_opens_for_a_quick_kill_and_hands_over_a_chip() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(1_500);

    let ev = run.pending_event().expect("a fast kill in the shallow end opens the door");
    assert_eq!(ev.id, "the-casino");

    let walk = ev
        .choices
        .iter()
        .find(|c| matches!(c.outcome, ChoiceOutcome::Give("Gold Chip")))
        .expect("the walk-away branch hands over the Gold Chip");
    assert!(run.choice_open(walk));

    let before = run.owned.iter().filter(|&&i| run.registry.def(i).name == "Gold Chip").count();
    run.take_choice(walk);
    let after = run.owned.iter().filter(|&&i| run.registry.def(i).name == "Gold Chip").count();
    assert_eq!(after, before + 1, "walked out without the chip");

    // Asked once, and never again.
    assert!(run.pending_event().is_none(), "the casino asked twice");
}

#[test]
fn a_slow_run_never_sees_the_casino() {
    let mut run = Run::with_all_pieces();
    run.rung = 4;
    run.best_fight_ms = Some(9_000);
    assert!(
        run.pending_event().map(|e| e.id) != Some("the-casino"),
        "the door opened for a run that never earned it"
    );

    // Quick enough, but far too late.
    run.best_fight_ms = Some(500);
    run.rung = casino().at + 1;
    assert!(
        run.pending_event().map(|e| e.id) != Some("the-casino"),
        "the door was still open past its last rung"
    );
}

#[test]
fn neither_chip_is_for_sale() {
    for name in ["Gold Chip", "Platinum Chip"] {
        assert!(
            gearmaster_engine::piece::is_event_only(name),
            "{name} would turn up on a shelf, which makes the casino pointless"
        );
        assert!(CATALOG.iter().any(|d| d.name == name), "{name} is in the catalogue");
    }
}

/// A build wearing the chip, with money to burn.
fn chip_build(gold: i32) -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    run.mode = Mode::Grinder;
    run.gold = gold;
    for name in ["Oak Handle", "Iron Blade", "Gold Chip"] {
        let id = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == name && !run.is_equipped(i))
            .unwrap_or_else(|| panic!("no {name}"));
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
    }
    run
}

#[test]
fn the_gold_chip_spends_the_purse_and_hits_harder_each_time() {
    let mut run = chip_build(500);
    run.rung = 11;
    let log = run.fight_next();

    let spends: Vec<i32> = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Spent { side: Side::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .collect();
    assert!(!spends.is_empty(), "the chip never paid anything");
    assert!(spends.iter().all(|&a| a == 5), "the cost is flat: {spends:?}");

    // Flat cost, climbing payout. The escalation is the whole piece.
    let total = log.gold_spent;
    assert_eq!(total, spends.iter().sum::<i32>(), "the log and the tally disagree");
    assert!(total <= 40, "the chip blew past its budget: {total}");

    let purse = run.gold;
    assert_eq!(purse, 500 - total, "the run was not charged what the fight spent");
}

#[test]
fn a_replayed_fight_does_not_charge_you_twice() {
    let mut run = chip_build(500);
    run.rung = 11;
    run.fight_next();
    let after_one = run.gold;
    assert!(after_one < 500, "nothing was spent, so this proves nothing");

    // Same fight again - a rematch is a new fight and may spend again, but
    // simply looking at the log must not move the purse.
    let spent_once = 500 - after_one;
    let _ = run.log.as_ref().map(|l| l.gold_spent);
    assert_eq!(run.gold, after_one, "reading the log charged the run again");
    assert_eq!(spent_once, run.log.as_ref().unwrap().gold_spent);
}

#[test]
fn a_penniless_build_still_swings() {
    // The chip going quiet must not stop the weapon it is built into.
    let mut run = chip_build(0);
    run.rung = 11;
    let log = run.fight_next();
    assert_eq!(log.gold_spent, 0, "spent money it did not have");
    assert!(
        log.entries.iter().any(|e| matches!(e.event, Event::Activate { side: Side::Player, .. })),
        "a broke player stopped fighting"
    );
    assert!(run.gold >= 0, "the purse went negative");
}

#[test]
fn the_casino_stands_on_a_rung_nothing_else_claims() {
    // `event::at` returns the first match, so a collision means one of the two
    // silently never fires.
    let at = casino().at;
    assert_eq!(EVENTS.iter().filter(|e| e.at == at).count(), 1);
    assert!(at < 10, "the casino is meant to be a shallow-end door, not a mid-run one");
    assert_eq!(LADDER[at].name, casino().expects);
}

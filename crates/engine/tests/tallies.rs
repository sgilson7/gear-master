//! What one item did in one fight.
//!
//! A `CombatLog` is a flat transcript. "What did that piece do" needs
//! attribution, attribution needs a rule, and the rule is the one
//! `tests/baseline.rs` has attributed damage by since the slot rewrite:
//! `Event::Activate` precedes its own item's effects and carries the item's
//! index, so a hit belongs to whichever item on that side last activated.
//!
//! These tests are about the rule holding, not about any particular board.

mod common;

use gearmaster_engine::combat::{tally_items, Event, Side};
use gearmaster_engine::run::Run;

fn a_fight() -> gearmaster_engine::combat::CombatLog {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.fight_next().clone()
}

#[test]
fn every_item_on_the_board_gets_an_account() {
    let log = a_fight();
    let tally = tally_items(&log, Side::Player, 0);
    assert_eq!(
        tally.len(),
        log.player.items.len(),
        "one account per item, in the order Activate names them"
    );
    for (i, t) in tally.iter().enumerate() {
        assert_eq!(t.index, i);
        assert_eq!(t.name, log.player.items[i].name);
    }
}

#[test]
fn the_accounts_add_up_to_the_fight() {
    let log = a_fight();
    let tally = tally_items(&log, Side::Player, 0);

    // Every activation the log records is in exactly one account.
    let logged = log
        .entries
        .iter()
        .filter(|e| matches!(&e.event, Event::Activate { side: Side::Player, .. }))
        .count() as u32;
    let counted: u32 = tally.iter().map(|t| t.activations).sum();
    assert_eq!(counted, logged, "an activation went missing or was counted twice");

    // And every point of damage the player dealt is attributed to something.
    let dealt: i32 = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Hit { by: Side::Player, damage, .. } => Some(*damage),
            _ => None,
        })
        .sum();
    let attributed: i32 = tally.iter().map(|t| t.of("damage")).sum();
    assert!(dealt > 0, "the preset board did no damage at all");
    assert_eq!(attributed, dealt, "damage was dropped on the floor");
}

#[test]
fn no_entry_is_claimed_by_two_items() {
    let log = a_fight();
    for (side, who) in [(Side::Player, 0u8), (Side::Enemy, 0u8)] {
        let tally = tally_items(&log, side, who);
        let mut seen: Vec<usize> = tally.iter().flat_map(|t| t.entries.clone()).collect();
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n, "{side:?}: one log entry landed in two accounts");
    }
}

#[test]
fn the_clock_and_the_ending_belong_to_nobody() {
    let log = a_fight();
    let tally = tally_items(&log, Side::Player, 0);
    let owned: Vec<usize> = tally.iter().flat_map(|t| t.entries.clone()).collect();
    for (i, e) in log.entries.iter().enumerate() {
        if matches!(e.event, Event::SuddenDeath { .. } | Event::End { .. }) {
            assert!(
                !owned.contains(&i),
                "the clock was attributed to an item, which is how a fight the \
                 clock decided reads as a board that won"
            );
        }
    }
}

#[test]
fn the_other_side_gets_the_same_treatment() {
    let log = a_fight();
    let tally = tally_items(&log, Side::Enemy, 0);
    assert_eq!(tally.len(), log.enemy().items.len());
    let dealt: i32 = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Hit { by: Side::Enemy, damage, .. } => Some(*damage),
            _ => None,
        })
        .sum();
    assert_eq!(tally.iter().map(|t| t.of("damage")).sum::<i32>(), dealt);
}

#[test]
fn a_foe_that_does_not_exist_has_no_account() {
    let log = a_fight();
    assert!(tally_items(&log, Side::Enemy, 9).is_empty(), "a foe nobody fought");
}

#[test]
fn a_tally_is_a_pure_function_of_the_log() {
    let log = a_fight();
    assert_eq!(
        tally_items(&log, Side::Player, 0),
        tally_items(&log, Side::Player, 0),
        "two reads of one log disagree"
    );
}

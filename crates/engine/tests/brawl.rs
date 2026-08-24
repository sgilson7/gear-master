//! Fights with more than one thing in them.
//!
//! The refactor that made this possible is behaviour-preserving for duels -
//! every other test in the suite says so - which means nothing in the suite
//! actually exercises a party. These do.

use gearmaster_engine::combat::{
    simulate_party, CombatLog, Difficulty, Event, Side, LADDER,
};
use gearmaster_engine::run::Run;

fn a_fighter() -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    for name in ["Oak Handle", "Iron Blade", "Adamant Base", "Riveted Layer", "Bone Frame", "Tin Plating"] {
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
    run
}

/// Health each foe was left on, read from the events rather than the setup.
fn foe_health(log: &CombatLog) -> Vec<i32> {
    let mut hp: Vec<i32> = log.enemies.iter().map(|e| e.health).collect();
    for e in &log.entries {
        let who = e.who as usize;
        match &e.event {
            Event::Hit { by: Side::Player, target_health, .. } => {
                if let Some(h) = hp.get_mut(who) {
                    *h = *target_health;
                }
            }
            Event::Fell { side: Side::Enemy } => {
                if let Some(h) = hp.get_mut(who) {
                    *h = 0;
                }
            }
            _ => {}
        }
    }
    hp
}

fn brawl(names: &[&str]) -> CombatLog {
    let run = a_fighter();
    let specs: Vec<_> = names
        .iter()
        .map(|n| *LADDER.iter().find(|m| m.name == *n).expect("on the ladder"))
        .collect();
    simulate_party(
        run.player_stats(),
        &run.combat_items(),
        &specs,
        Difficulty::Medium,
        &[],
        0,
    )
}

#[test]
fn a_duel_still_reads_as_a_duel() {
    let log = brawl(&["Cave Rat"]);
    assert_eq!(log.enemies.len(), 1);
    assert!(!log.is_brawl());
    // Nothing in a one-creature fight is about foe one.
    assert!(log.entries.iter().all(|e| e.who == 0), "a duel logged a second foe");
}

#[test]
fn two_creatures_are_two_creatures() {
    let log = brawl(&["Cave Rat", "Bog Toad"]);
    assert_eq!(log.enemies.len(), 2);
    assert!(log.is_brawl());
    assert_eq!(log.enemies[0].name, "Cave Rat");
    assert_eq!(log.enemies[1].name, "Bog Toad");
    // `enemy()` is the shorthand for the usual case and must not lie about it.
    assert_eq!(log.enemy().name, "Cave Rat");
}

#[test]
fn the_aim_moves_along_so_they_come_down_together() {
    // Two of the same thing, so anything other than an even split is the
    // targeting rule and not the creatures.
    let log = brawl(&["Bog Toad", "Bog Toad"]);
    let hp = foe_health(&log);
    assert_eq!(hp.len(), 2);

    let start = log.enemies[0].health;
    let dealt: Vec<i32> = hp.iter().map(|h| start - h).collect();
    assert!(dealt.iter().all(|&d| d > 0), "one of them was never touched: {dealt:?}");

    // Whittled at one rate, not one at a time. A single swing of slack is
    // fine - the aim moves after each attack, so at any moment one of them is
    // at most one hit ahead.
    let gap = (dealt[0] - dealt[1]).abs();
    let biggest_hit = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Hit { by: Side::Player, damage, .. } => Some(*damage),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    assert!(
        gap <= biggest_hit.max(1),
        "one took {} and the other {} - that is focus fire, not a spread",
        dealt[0],
        dealt[1]
    );
}

#[test]
fn both_of_them_get_to_hit_you() {
    let log = brawl(&["The Iron Warden", "The Iron Warden"]);
    let mut acted: Vec<u8> = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::Activate { side: Side::Enemy, .. }))
        .map(|e| e.who)
        .collect();
    acted.sort_unstable();
    acted.dedup();
    assert_eq!(acted, vec![0, 1], "only {acted:?} of the two ever took a turn");
}

#[test]
fn killing_one_does_not_end_the_fight() {
    // A rat and something that will not fall over: the rat goes down early
    // and the fight has to carry on.
    let log = brawl(&["Cave Rat", "The Hollow King"]);
    let fell: Vec<u8> = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::Fell { side: Side::Enemy }))
        .map(|e| e.who)
        .collect();
    if fell.contains(&0) && !fell.contains(&1) {
        assert_ne!(
            log.outcome,
            gearmaster_engine::combat::Outcome::Victory,
            "the fight was won with one of them still standing"
        );
    }
}

#[test]
fn a_foe_that_is_down_stops_taking_turns() {
    let log = brawl(&["Cave Rat", "The Hollow King"]);
    let Some(fell_at) = log
        .entries
        .iter()
        .find(|e| matches!(e.event, Event::Fell { side: Side::Enemy }) && e.who == 0)
        .map(|e| e.at_ms)
    else {
        return; // the rat survived; nothing to check
    };
    let after: Vec<u32> = log
        .entries
        .iter()
        .filter(|e| {
            e.who == 0
                && e.at_ms > fell_at
                && matches!(e.event, Event::Activate { side: Side::Enemy, .. })
        })
        .map(|e| e.at_ms)
        .collect();
    assert!(after.is_empty(), "a dead thing kept swinging at {after:?}");
}

#[test]
fn a_brawl_is_worse_than_either_of_them_alone() {
    // The point of the whole feature. If two at once is easier than the harder
    // one on its own, something is wrong with the targeting or the turns.
    //
    // Measured as how long the player lasts, not what health they end on:
    // sudden death brings every unfinished fight to nearly zero on both sides,
    // so end-state health stopped telling one fight from another the moment
    // that rule landed.
    let one = brawl(&["The Iron Warden"]);
    let two = brawl(&["The Iron Warden", "The Iron Warden"]);
    let lasted = |log: &CombatLog| -> u32 {
        log.entries
            .iter()
            .find(|e| matches!(e.event, Event::Fell { side: Side::Player }))
            .map(|e| e.at_ms)
            .unwrap_or(log.duration_ms)
    };
    assert!(
        lasted(&two) < lasted(&one),
        "the player lasted {}ms against two of them and {}ms against one - two is not harder",
        lasted(&two),
        lasted(&one)
    );
}

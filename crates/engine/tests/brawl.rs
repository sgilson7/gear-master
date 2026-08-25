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
///
/// Note the clamp: a foe that falls is written down to zero, so the damage this
/// implies for the killing blow includes every point of overkill. That is fine
/// for "did this one come down" and useless for "were they whittled evenly" -
/// see `the_aim_moves_along_so_they_come_down_together`, which counts blows.
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
    assert_eq!(log.enemies.len(), 2);

    // Counted in blows, not in damage.
    //
    // Damage cannot answer this question and it took a repack to notice. A
    // foe that falls is charged with all the health it had left, however far
    // the killing blow overshot, so the sequence hit-A, hit-B, kill-A reads as
    // "A took its whole health bar and B took one hit" - a gap the size of the
    // creature, produced by the aim working exactly as intended. The lighter
    // the creature, the worse the reading, so the test got harder to pass the
    // *weaker* the thing being hit, which is not a property any test should
    // have.
    //
    // What the rule says is that the aim moves along after every attack. That
    // is about blows, and blows are what this counts - up to the moment one of
    // them falls, after which every remaining swing goes to the survivor and
    // ought to.
    let mut hits = [0i32; 2];
    let mut down = [false; 2];
    for e in &log.entries {
        let who = e.who as usize;
        match &e.event {
            Event::Hit { by: Side::Player, .. } if !down[0] && !down[1] => {
                if let Some(h) = hits.get_mut(who) {
                    *h += 1;
                }
            }
            Event::Fell { side: Side::Enemy } => {
                if let Some(d) = down.get_mut(who) {
                    *d = true;
                }
            }
            _ => {}
        }
    }
    let landed: i32 = hits.iter().sum();
    assert!(
        landed >= 2,
        "only {landed} blow(s) landed while both were standing, so there is no alternation \
         to see. The fixture is too thin for the creatures it is fighting - give it more \
         board, do not relax this."
    );
    assert!(hits.iter().all(|&h| h > 0), "one of them was never touched: {hits:?}");
    assert!(
        (hits[0] - hits[1]).abs() <= 1,
        "one took {} blows and the other {} while both were standing - that is focus \
         fire, not a spread",
        hits[0],
        hits[1]
    );

    // And it is not a spread that leaves one of them healthy: both come down.
    let hp = foe_health(&log);
    let start = log.enemies[0].health;
    let dealt: Vec<i32> = hp.iter().map(|h| start - h).collect();
    assert!(dealt.iter().all(|&d| d > 0), "one of them took no damage at all: {dealt:?}");
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

//! Taking a pool off someone, in both directions.
//!
//! A drain is worth nothing against an empty pool, which makes it very easy to
//! ship one that never fires and never looks broken. Both tests here bank the
//! pool first, on purpose.

use gearmaster_engine::combat::{simulate_at, CombatLog, Difficulty, Event, Side, LADDER};
use gearmaster_engine::run::Run;

fn wearing(names: &[&str]) -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    for name in names {
        let id = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == *name && !run.is_equipped(i))
            .unwrap_or_else(|| panic!("no such component: {name}"));
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
        assert!(run.is_equipped(id), "{name} would not sit in {slot:?}");
    }
    run
}

fn drains(log: &CombatLog, on: Side) -> Vec<(&'static str, i32)> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Drained { on: o, what, amount, .. } if *o == on => Some((*what, *amount)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_leech_takes_the_pool_off_the_other_side() {
    // Sump Sole takes the lot rather than a slice, so any enemy that banks
    // mana at all will show it.
    let run = wearing(&["Rootwoven Material", "Sump Sole"]);
    let stats = run.player_stats();
    let items = run.combat_items();
    assert!(!items.is_empty(), "the greave has to assemble to fire");

    let taken: Vec<(&str, i32)> = LADDER
        .iter()
        .flat_map(|spec| {
            let log = simulate_at(stats, &items, spec, Difficulty::Medium);
            drains(&log, Side::Enemy)
        })
        .collect();
    assert!(
        !taken.is_empty(),
        "no creature on the ladder ever lost mana to a piece whose whole job is taking it"
    );
    assert!(taken.iter().all(|(w, n)| *w == "mana" && *n > 0), "{taken:?}");
}

#[test]
fn losing_a_pool_to_a_creature_hurts_for_what_was_taken() {
    // A faith build, walked into the creatures carrying Tithe Collector. It
    // needs a weapon and a chest as well as the faith: their helmet comes
    // round at four seconds, and a build wearing only a hat does not last
    // four seconds.
    let run = wearing(&[
        "Covenant Frame",
        "Warded Plating",
        "Vigil Crest",
        "Zealot's Haft",
        "Iron Blade",
        "Adamant Base",
        "Riveted Layer",
    ]);
    let stats = run.player_stats();
    let items = run.combat_items();
    assert!(!items.is_empty(), "the helmet has to assemble to bank faith");

    let carriers = ["Pale Twin", "Null Sentinel", "The Iron Choir"];
    let mut seen = 0usize;
    for name in carriers {
        let spec = LADDER.iter().find(|m| m.name == name).expect("on the ladder");
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        let lost = drains(&log, Side::Player);
        if lost.is_empty() {
            continue;
        }
        seen += 1;
        assert!(lost.iter().all(|(w, _)| *w == "faith"), "{name}: {lost:?}");

        // The damage lands in the same tick as the drain and is priced off it.
        let at = log
            .entries
            .iter()
            .position(|e| matches!(e.event, Event::Drained { on: Side::Player, .. }))
            .expect("just found one");
        let taken = lost[0].1;
        let hit = log.entries[at + 1..]
            .iter()
            .take(4)
            .find_map(|e| match &e.event {
                Event::Hit { by: Side::Enemy, damage, .. } => Some(*damage),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name}: took {taken} faith and charged nothing for it"));
        assert_eq!(
            hit,
            taken * 3,
            "{name}: took {taken} faith and hit for {hit}, which is not three a point"
        );
    }
    assert!(seen > 0, "none of {carriers:?} ever took a point of faith off a faith build");
}

#[test]
fn a_drain_against_an_empty_pool_does_nothing_at_all() {
    // No faith banked, so the same creatures should take nothing and, more to
    // the point, charge nothing for it.
    let run = wearing(&["Oak Handle", "Iron Blade"]);
    let stats = run.player_stats();
    let items = run.combat_items();

    for name in ["Pale Twin", "Null Sentinel", "The Iron Choir"] {
        let spec = LADDER.iter().find(|m| m.name == name).expect("on the ladder");
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        assert!(
            drains(&log, Side::Player).is_empty(),
            "{name} drained faith from a build that has none"
        );
    }
}

//! End-to-end: does a built loadout actually change the fight?

mod common;

use common::{build_full_loadout, equip};
use gearmaster_engine::combat::{Event, Outcome, Side, ENEMY_HEALTH};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::{Phase, Run};
use gearmaster_engine::stats::Stats;

#[test]
fn a_bare_character_starts_at_the_documented_baseline() {
    let run = Run::new();
    assert_eq!(run.player_stats(), Stats::new(100, 5, 0, 100));
    assert_eq!(run.player_stats().damage_per_attack(), 5);
}

#[test]
fn a_full_loadout_totals_up_base_stats_plus_every_bonus() {
    let mut run = Run::new();
    build_full_loadout(&mut run);

    // 100 base + 30 helmet + 61 chest + 5 gloves + 30 greaves
    // 5 base str + 3 helmet + 5 chest (Hollow Weave) + 9 gloves + 7 weapon
    // 1 helmet + 2 chest + 3 greaves regen
    // 1.00x base + 0.15x gloves + 1.30x weapon
    assert_eq!(run.player_stats(), Stats::new(226, 29, 6, 245));
    assert_eq!(run.player_stats().damage_per_attack(), 71);
}

#[test]
fn an_ungeared_character_is_beaten_by_the_golem() {
    let mut run = Run::new();
    let log = run.begin_fight().clone();

    assert_eq!(log.outcome, Outcome::Defeat);
    assert_eq!(log.turns, 10, "100 health against 10 damage a turn");
}

#[test]
fn a_full_loadout_beats_the_golem() {
    let mut run = Run::new();
    build_full_loadout(&mut run);
    let log = run.begin_fight().clone();

    assert_eq!(log.outcome, Outcome::Victory);
    assert_eq!(log.turns, 6, "400 golem health at 71 damage a turn");
    assert!(
        log.player.health > 0,
        "the player should still be standing at the end"
    );
}

#[test]
fn the_fight_log_replays_the_whole_bout_in_order() {
    let mut run = Run::new();
    build_full_loadout(&mut run);
    let log = run.begin_fight().clone();

    // First beat is always the player's swing.
    assert!(matches!(
        log.entries.first().map(|e| &e.event),
        Some(Event::Attack { by: Side::Player, .. })
    ));
    // Last beat is always the outcome.
    assert!(matches!(
        log.entries.last().map(|e| &e.event),
        Some(Event::End { outcome: Outcome::Victory })
    ));
    // Turn numbers never go backwards.
    let turns: Vec<u32> = log.entries.iter().map(|e| e.turn).collect();
    assert!(turns.windows(2).all(|w| w[0] <= w[1]), "turns must be monotonic");

    // Replaying the attack events reproduces the golem's health exactly, which
    // is what lets the GUI animate straight from the log.
    let mut enemy_hp = ENEMY_HEALTH;
    for entry in &log.entries {
        if let Event::Attack { by: Side::Player, damage, target_health } = entry.event {
            enemy_hp -= damage;
            assert_eq!(enemy_hp, target_health);
        }
    }
    assert!(enemy_hp <= 0);
}

#[test]
fn regeneration_shows_up_in_the_log_and_slows_the_bleed() {
    let mut run = Run::new();
    build_full_loadout(&mut run); // 6 regen
    let log = run.begin_fight().clone();

    let regen_events = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::Regen { side: Side::Player, .. }))
        .count();
    assert!(regen_events > 0, "a 6-regen build should heal between turns");
}

#[test]
fn gear_is_locked_while_a_fight_is_running() {
    let mut run = Run::new();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);
    run.begin_fight();
    assert_eq!(run.phase, Phase::Fighting);

    let blade = common::piece(&run, "Iron Blade");
    assert!(run.equip(blade, SlotKind::Weapon, 1, 0).is_err());
    assert!(run.rotate(blade).is_err());
    assert!(run.clear_slot(SlotKind::Weapon).is_err());

    run.back_to_loadout();
    assert_eq!(run.phase, Phase::Loadout);
    assert!(run.log.is_none());
    assert!(run.equip(blade, SlotKind::Weapon, 1, 0).is_ok(), "unlocked again");
}

#[test]
fn assembling_the_weapon_is_what_turns_the_fight_around() {
    // Same two components, the only difference being whether they touch.
    let mut apart = Run::new();
    equip(&mut apart, "Balanced Grip", SlotKind::Weapon, 0, 0);
    equip(&mut apart, "Iron Blade", SlotKind::Weapon, 3, 0);
    assert_eq!(apart.report(SlotKind::Weapon).assembled_count(), 0);

    let mut together = Run::new();
    equip(&mut together, "Balanced Grip", SlotKind::Weapon, 0, 0);
    equip(&mut together, "Iron Blade", SlotKind::Weapon, 1, 0);
    assert_eq!(together.report(SlotKind::Weapon).assembled_count(), 1);

    // The bonus is worth +0.50x, so the same pieces hit meaningfully harder.
    assert_eq!(apart.player_stats().power, 190);
    assert_eq!(together.player_stats().power, 240);
    assert!(
        together.player_stats().damage_per_attack()
            > apart.player_stats().damage_per_attack(),
        "completing the weapon must beat leaving it in bits"
    );
}

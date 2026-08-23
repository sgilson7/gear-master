//! Classes have to reach the simulation.
//!
//! Every other class test asks whether a build *qualifies* for one. Nothing
//! asked whether holding it changes a fight, and the answer is not obvious
//! from the code: `Standing` powers are folded into the character sheet by the
//! run, and the rest are fields on `Combatant` that the tick loop has to
//! actually read. A power added to `ClassPower` but never read would pass the
//! whole suite.

use gearmaster_engine::class::{ClassPower, CLASSES};
use gearmaster_engine::combat::{simulate_with_class, CombatLog, Difficulty, Event, Side, LADDER};
use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;

/// Health on both sides when the fight ended.
///
/// `CombatLog::player` and `::enemy` are the combatants as they *started* -
/// the interface lays the two boards out from them - so reading health off
/// them gives the pre-fight number. A build that loses at rung 41 still
/// reports full health there.
fn final_health(log: &CombatLog) -> (i32, i32) {
    let mut player = log.player.health;
    let mut enemy = log.enemy.health;
    for e in &log.entries {
        match &e.event {
            Event::Hit { by, target_health, .. } => match by {
                Side::Player => enemy = *target_health,
                Side::Enemy => player = *target_health,
            },
            Event::Burn { side, health, .. } | Event::Regen { side, health, .. } => match side {
                Side::Player => player = *health,
                Side::Enemy => enemy = *health,
            },
            Event::Fell { side } => match side {
                Side::Player => player = 0,
                Side::Enemy => enemy = 0,
            },
            _ => {}
        }
    }
    (player, enemy)
}

/// A board that fights: enough gear to swing, not enough to be safe.
fn a_fighting_run() -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    let ids: Vec<_> = run.owned.iter().copied().take(60).collect();
    for id in ids {
        'placed: for slot in SlotKind::ALL {
            for y in 0..8u8 {
                for x in 0..6u8 {
                    if run.equip(id, slot, x, y).is_ok() {
                        break 'placed;
                    }
                }
            }
        }
    }
    run
}

#[test]
fn bastion_actually_reduces_damage() {
    let run = a_fighting_run();
    let (stats, items) = (run.player_stats(), run.combat_items());
    let bulwark = *CLASSES.iter().find(|c| c.name == "Bulwark").expect("Bulwark exists");
    assert!(matches!(bulwark.power, ClassPower::Bastion(_)), "{:?}", bulwark.power);

    // Across the ladder rather than at one rung. A soak cannot show up in a
    // fight nothing lands in, nor in one the player loses to a single blow
    // either way - so the question is whether it *ever* matters, not whether
    // it matters here.
    let mut moved = Vec::new();
    for (i, spec) in LADDER.iter().enumerate() {
        let bare = simulate_with_class(stats, &items, spec, Difficulty::Medium, &[]);
        let with = simulate_with_class(stats, &items, spec, Difficulty::Medium, &[bulwark]);
        let (bare_hp, _) = final_health(&bare);
        let (with_hp, _) = final_health(&with);
        if with_hp != bare_hp || with.duration_ms != bare.duration_ms {
            moved.push((i + 1, bare_hp, with_hp));
        }
    }
    assert!(
        !moved.is_empty(),
        "Bastion(35) changed nothing at any of the {} rungs - the power is not \
         reaching the simulation",
        LADDER.len()
    );
    println!("Bastion moved {} of {} rungs, e.g. {:?}", moved.len(), LADDER.len(), &moved[..3.min(moved.len())]);
}

#[test]
fn the_log_records_the_fight_not_the_setup() {
    // The guard for the mistake above: if this ever passes trivially again,
    // every margin measured off `log.player` is measuring the character sheet.
    let run = a_fighting_run();
    let spec = LADDER[40];
    let log =
        simulate_with_class(run.player_stats(), &run.combat_items(), &spec, Difficulty::Medium, &[]);
    let (player, _) = final_health(&log);
    assert!(
        player < log.player.health,
        "a fight the player loses left them on {} of {} health - `log.player` is the \
         starting snapshot, so end state has to be read from the events",
        player,
        log.player.health
    );
}

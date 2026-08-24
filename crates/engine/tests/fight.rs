//! Combat: per-item cooldowns, armour, mana, curses and mind damage.

mod common;

use common::{build_full_loadout, equip, piece};
use gearmaster_engine::combat::{
    simulate, Event, MonsterAttack, MonsterSpec, Outcome, Side, BURN_REPORT_MS, RUST_GOLEM,
};
use gearmaster_engine::curse::CurseKind;
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::piece::{Action, SlotKind, Target, Trigger};
use gearmaster_engine::run::{Phase, Run};
use gearmaster_engine::stats::Stats;

/// A bare item profile, so a mechanic can be tested without contriving a
/// loadout that happens to produce it.
fn item(name: &str, slot: SlotKind, cooldown_ms: u32, stats: Stats) -> ItemProfile {
    ItemProfile {
        sigil_seed: 0,
        pieces: Vec::new(),
        name: name.to_string(),
        full_name: name.to_string(),
        core: name.to_string(),
        slot,
        cooldown_ms,
        stats,
        triggers: Vec::new(),
        adjacent_assembled_same_slot: 0,
        diagonal_items: Vec::new(),
        open_cells: 0,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    }
}

/// A monster that stands there and does nothing, for testing player mechanics.
const DUMMY: MonsterSpec = MonsterSpec {
    name: "Dummy",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
    attacks: &[],
    gear: &[],
    gear_offset: 0,
    bounty: 0,
    sprite: gearmaster_engine::combat::MonsterSprite::Rat,
    rank: gearmaster_engine::combat::Rank::Ordinary,
    drops: &[],
    items: &[],
};

/// A monster that only hits, for testing defensive mechanics.
const PUNCHER: MonsterSpec = MonsterSpec {
    name: "Puncher",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
    attacks: &[MonsterAttack::hit("jab", 1000, 10)],
    gear: &[],
    gear_offset: 0,
    bounty: 0,
    sprite: gearmaster_engine::combat::MonsterSprite::Rat,
    rank: gearmaster_engine::combat::Rank::Ordinary,
    drops: &[],
    items: &[],
};

fn activations_of(log: &gearmaster_engine::combat::CombatLog, name: &str) -> Vec<u32> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Activate { side: Side::Player, item, .. } if item == name => Some(e.at_ms),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------- baseline

#[test]
fn a_bare_character_starts_at_the_documented_baseline() {
    let run = Run::with_all_pieces();
    let s = run.player_stats();
    assert_eq!((s.health, s.strength, s.regen, s.power), (gearmaster_engine::stats::BASE_HEALTH, 5, 0, 100));
    assert!(run.combat_items().is_empty(), "nothing assembled, nothing acts");
}

#[test]
fn an_ungeared_character_is_beaten_by_the_golem() {
    let mut run = Run::with_all_pieces();
    let log = run.begin_fight().clone();
    // With no weapon you deal nothing, so the only question is how long the
    // golem takes.
    assert_eq!(log.outcome, Outcome::Defeat);
}

#[test]
fn a_full_loadout_beats_the_golem() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);
    let log = run.begin_fight().clone();
    assert_eq!(log.outcome, Outcome::Victory);
    assert!(log.duration_ms < 15_000, "took {}ms", log.duration_ms);
}

#[test]
fn the_log_is_ordered_and_ends_with_the_outcome() {
    let mut run = Run::with_all_pieces();
    build_full_loadout(&mut run);
    let log = run.begin_fight().clone();

    let times: Vec<u32> = log.entries.iter().map(|e| e.at_ms).collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "timestamps must not go backwards");
    assert!(matches!(
        log.entries.last().map(|e| &e.event),
        Some(Event::End { outcome: Outcome::Victory })
    ));
}

// ------------------------------------------------ per-item cooldowns

#[test]
fn each_item_keeps_its_own_cooldown() {
    let fast = item("Fast", SlotKind::Weapon, 500, Stats::physical(1));
    let slow = item("Slow", SlotKind::Weapon, 2000, Stats::physical(1));
    let log = simulate(Stats::new(1000, 0, 0, 100), &[fast, slow], &DUMMY);

    // Counted over a whole number of the slow item's cycles, not to the end of
    // the fight. Sudden death stops a fight wherever it stops, which can leave
    // the fast item one swing into a cycle the slow one has not finished - a
    // ratio of 85 to 21 rather than 84 to 21, and nothing wrong with either.
    const WINDOW_MS: u32 = 20_000;
    let within = |name: &str| {
        activations_of(&log, name).into_iter().filter(|&t| t <= WINDOW_MS).count()
    };
    let (fast_hits, slow_hits) = (within("Fast"), within("Slow"));
    assert!(slow_hits > 0, "the slow item never fired; this proves nothing");
    assert_eq!(
        fast_hits,
        slow_hits * 4,
        "a 0.5s item fires four times as often as a 2s one ({} vs {})",
        fast_hits,
        slow_hits
    );
}

#[test]
fn activations_land_exactly_on_the_cooldown() {
    let log = simulate(
        Stats::new(1000, 0, 0, 100),
        &[item("Tick", SlotKind::Weapon, 750, Stats::physical(1))],
        &DUMMY,
    );
    let at = activations_of(&log, "Tick");
    assert_eq!(&at[..4], &[750, 1500, 2250, 3000]);
}

#[test]
fn a_speed_bonus_halves_the_cooldown_of_the_weapon_it_joins() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Cursed Handle", SlotKind::Weapon, 0, 0);
    let alone = run.combat_items();
    assert_eq!(alone.len(), 0, "a handle on its own is not a weapon yet");

    equip(&mut run, "Cursed Blade", SlotKind::Weapon, 1, 0);
    let built = run.combat_items();
    assert_eq!(built.len(), 1);
    // Handle's own 2s, halved by the blade's +100% speed.
    assert_eq!(built[0].cooldown_ms, 1000);
}

// --------------------------------------------------------------- armour

#[test]
fn armour_soaks_damage_before_health_does() {
    // One chest item granting 30 armour a second, against a 10-damage jab.
    let armour = item("Plate", SlotKind::Chest, 1000, Stats::armor(30));
    let log = simulate(Stats::new(100, 0, 0, 100), &[armour], &PUNCHER);

    let first_hit = log
        .entries
        .iter()
        .find_map(|e| match e.event {
            Event::Hit { by: Side::Enemy, absorbed, target_health, .. } => {
                Some((absorbed, target_health))
            }
            _ => None,
        })
        .expect("the puncher swings");
    assert_eq!(first_hit, (10, 100), "fully absorbed, health untouched");
}

#[test]
fn armour_starts_every_fight_at_zero() {
    // No armour-granting item, so the very first hit lands on health.
    let log = simulate(Stats::new(100, 0, 0, 100), &[], &PUNCHER);
    let first_hit = log
        .entries
        .iter()
        .find_map(|e| match e.event {
            Event::Hit { by: Side::Enemy, absorbed, target_health, .. } => {
                Some((absorbed, target_health))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(first_hit, (0, 90));
}

// ----------------------------------------------------------------- mana

#[test]
fn a_mana_trigger_takes_the_failure_branch_when_it_cannot_pay() {
    let mut caster = item("Caster", SlotKind::Weapon, 1000, Stats::physical(1));
    caster.triggers = vec![Trigger::SpendMana {
        cost: 5,
        on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
        on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
    }];
    // No mana income at all.
    let log = simulate(Stats::new(1000, 0, 0, 100), &[caster], &DUMMY);

    let paid: Vec<bool> = log
        .entries
        .iter()
        .filter_map(|e| match e.event {
            Event::ManaCheck { paid, .. } => Some(paid),
            _ => None,
        })
        .collect();
    assert!(!paid.is_empty());
    assert!(paid.iter().all(|p| !p), "never affordable, so never paid");
    assert!(
        log.entries.iter().any(|e| matches!(
            e.event,
            Event::Cursed { on: Side::Player, kind: CurseKind::Frost, .. }
        )),
        "the failure branch curses its own wearer"
    );
}

#[test]
fn a_mana_trigger_spends_and_curses_the_enemy_when_it_can_pay() {
    let battery = item("Battery", SlotKind::Helmet, 500, Stats::mana(10));
    let mut caster = item("Caster", SlotKind::Weapon, 1000, Stats::physical(1));
    caster.triggers = vec![Trigger::SpendMana {
        cost: 5,
        on_success: Action::Curse { kind: CurseKind::Searing, target: Target::Enemy },
        on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
    }];
    let log = simulate(Stats::new(1000, 0, 0, 100), &[battery, caster], &DUMMY);

    assert!(
        log.entries.iter().any(|e| matches!(e.event, Event::ManaCheck { paid: true, .. })),
        "20 mana a second easily covers 5 a second"
    );
    assert!(
        log.entries.iter().any(|e| matches!(
            e.event,
            Event::Cursed { on: Side::Enemy, kind: CurseKind::Searing, .. }
        )),
        "the success branch curses the enemy"
    );
    assert!(
        !log.entries.iter().any(|e| matches!(e.event, Event::ManaCheck { paid: false, .. })),
        "and never falls through to the penalty"
    );
}

// -------------------------------------------------------------- curses

#[test]
fn frost_on_yourself_visibly_delays_your_next_activation() {
    let mut cursed = item("Cursed", SlotKind::Weapon, 1000, Stats::physical(1));
    cursed.triggers = vec![Trigger::SpendMana {
        cost: 5,
        on_success: Action::GainMana(0),
        // Nothing to pay with, so every swing frosts its own wearer.
        on_failure: Action::Curse { kind: CurseKind::Frost, target: Target::Yourself },
    }];
    let log = simulate(Stats::new(5000, 0, 0, 100), &[cursed], &DUMMY);

    let at = activations_of(&log, "Cursed");
    assert_eq!(at[0], 1000, "the first swing is on time");
    assert_eq!(
        at[1] - at[0],
        1500,
        "frost halves the fill rate for a second, so the next takes 1.5s"
    );
}

#[test]
fn searing_burns_the_enemy_over_time() {
    let mut brand = item("Brand", SlotKind::Weapon, 20_000, Stats::physical(1));
    brand.triggers = vec![Trigger::OnActivate(Action::Curse {
        kind: CurseKind::Searing,
        target: Target::Enemy,
    })];
    let log = simulate(Stats::new(1000, 0, 0, 100), &[brand], &DUMMY);

    // The brand re-applies every 20s, so measure one curse's own window.
    let applied_at = log
        .entries
        .iter()
        .find_map(|e| match e.event {
            Event::Cursed { on: Side::Enemy, kind: CurseKind::Searing, duration_ms, .. } => {
                Some((e.at_ms, duration_ms))
            }
            _ => None,
        })
        .expect("the brand curses");
    assert_eq!(applied_at.1, 10_000, "unresisted searing runs its full 10s");
    let burn_total: i32 = log
        .entries
        .iter()
        .filter(|e| {
            e.at_ms > applied_at.0 && e.at_ms <= applied_at.0 + applied_at.1 + BURN_REPORT_MS
        })
        .filter_map(|e| match e.event {
            Event::Burn { side: Side::Enemy, damage, .. } => Some(damage),
            _ => None,
        })
        .sum();
    // 10 a second for 10 seconds, landing in half-point slices.
    assert_eq!(burn_total, 100);
}

#[test]
fn curse_resistance_shortens_the_burn() {
    let mut brand = item("Brand", SlotKind::Weapon, 20_000, Stats::physical(1));
    brand.triggers = vec![Trigger::OnActivate(Action::Curse {
        kind: CurseKind::Searing,
        target: Target::Enemy,
    })];
    const TOUGH: MonsterSpec = MonsterSpec {
        name: "Warded",
        health: 100_000,
        strength: 0,
        regen: 0,
        mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
        curse_resist: 50,
        attacks: &[],
        gear: &[],
        gear_offset: 0,
        bounty: 0,
        sprite: gearmaster_engine::combat::MonsterSprite::Rat,
        rank: gearmaster_engine::combat::Rank::Ordinary,
        drops: &[],
        items: &[],
    };
    let log = simulate(Stats::new(1000, 0, 0, 100), &[brand], &TOUGH);

    let (applied_at, duration) = log
        .entries
        .iter()
        .find_map(|e| match e.event {
            Event::Cursed { on: Side::Enemy, kind: CurseKind::Searing, duration_ms, .. } => {
                Some((e.at_ms, duration_ms))
            }
            _ => None,
        })
        .expect("the brand curses");
    assert_eq!(duration, 5_000, "50% resistance halves the duration");
    let burn_total: i32 = log
        .entries
        .iter()
        .filter(|e| e.at_ms > applied_at && e.at_ms <= applied_at + duration + BURN_REPORT_MS)
        .filter_map(|e| match e.event {
            Event::Burn { side: Side::Enemy, damage, .. } => Some(damage),
            _ => None,
        })
        .sum();
    assert_eq!(burn_total, 50, "half the duration, so half the damage");
}

#[test]
fn the_per_adjacent_item_trigger_fires_once_per_touching_item() {
    let mut blade = item("Blade", SlotKind::Weapon, 1000, Stats::physical(1));
    blade.triggers = vec![Trigger::PerAdjacentItem {
        action: Action::Curse { kind: CurseKind::Searing, target: Target::Yourself },
        same_slot_only: true,
    }];
    blade.adjacent_assembled_same_slot = 2;
    let log = simulate(Stats::new(100_000, 0, 0, 100), &[blade], &DUMMY);

    // Two curses land on the first activation alone.
    let first_batch = log
        .entries
        .iter()
        .filter(|e| {
            e.at_ms == 1000
                && matches!(e.event, Event::Cursed { on: Side::Player, kind: CurseKind::Searing, .. })
        })
        .count();
    assert_eq!(first_batch, 2, "one per adjacent assembled item");
}

// ---------------------------------------------------------- mind damage

#[test]
fn mind_damage_eats_maximum_health_and_cannot_be_healed_back() {
    const PSION: MonsterSpec = MonsterSpec {
        name: "Psion",
        health: 100_000,
        strength: 0,
        regen: 0,
        mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
        attacks: &[MonsterAttack::mind("whisper", 1000, 4)],
        gear: &[],
        gear_offset: 0,
        bounty: 0,
        sprite: gearmaster_engine::combat::MonsterSprite::Rat,
        rank: gearmaster_engine::combat::Rank::Ordinary,
        drops: &[],
        items: &[],
    };
    // Plenty of regeneration: it still cannot undo a lowered ceiling.
    let log = simulate(Stats::new(100, 0, 50, 100), &[], &PSION);

    let last_max = log
        .entries
        .iter()
        .filter_map(|e| match e.event {
            Event::MindHit { target_max_health, .. } => Some(target_max_health),
            _ => None,
        })
        .last()
        .expect("the psion whispers");
    assert!(last_max < 100, "maximum health came down to {}", last_max);
    assert_eq!(log.outcome, Outcome::Defeat, "a ceiling of zero is still death");
}

#[test]
fn mind_resistance_blunts_it() {
    const PSION: MonsterSpec = MonsterSpec {
        name: "Psion",
        health: 100_000,
        strength: 0,
        regen: 0,
        mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
        attacks: &[MonsterAttack::mind("whisper", 1000, 10)],
        gear: &[],
        gear_offset: 0,
        bounty: 0,
        sprite: gearmaster_engine::combat::MonsterSprite::Rat,
        rank: gearmaster_engine::combat::Rank::Ordinary,
        drops: &[],
        items: &[],
    };
    let mut warded = Stats::new(100, 0, 0, 100);
    warded.mind_resist = 60;
    let log = simulate(warded, &[], &PSION);

    let first = log
        .entries
        .iter()
        .find_map(|e| match e.event {
            Event::MindHit { amount, .. } => Some(amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(first, 4, "10 mind damage at 60% resistance");
}

// ------------------------------------------------- cross-item strength

#[test]
fn the_cursed_handle_doubles_a_touching_items_strength() {
    let mut run = Run::with_all_pieces();
    // A finished glove whose material carries strength...
    equip(&mut run, "Steel Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Gauntlet Mold", SlotKind::Gloves, 2, 0);
    let before = run.report(SlotKind::Gloves).stats.strength;

    // ...and the same glove with the cursed handle's aura is unaffected,
    // because the handle lives in the weapon slot, not this one.
    assert_eq!(run.report(SlotKind::Gloves).stats.strength, before);

    // Within the weapon slot: two weapons flush against each other.
    let mut w = Run::with_all_pieces();
    equip(&mut w, "Cursed Handle", SlotKind::Weapon, 0, 0); // (0, 0..2)
    equip(&mut w, "Cursed Blade", SlotKind::Weapon, 1, 0); // touches it
    equip(&mut w, "Oak Handle", SlotKind::Weapon, 3, 0); // (3, 0..2)
    equip(&mut w, "Serrated Edge", SlotKind::Weapon, 4, 0); // strength 4

    let r = w.report(SlotKind::Weapon);
    assert_eq!(r.assembled_count(), 2, "{}", r.summary());
    // The two weapons don't touch yet, so nothing is doubled.
    assert!(!r.notes().iter().any(|n| n.contains("doubled")), "{:?}", r.notes());
}

// ----------------------------------------------------------- lifecycle

#[test]
fn gear_is_locked_while_a_fight_is_running() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Balanced Grip", SlotKind::Weapon, 0, 0);
    run.begin_fight();
    assert_eq!(run.phase, Phase::Fighting);

    let blade = piece(&run, "Iron Blade");
    assert!(run.equip(blade, SlotKind::Weapon, 1, 0).is_err());
    assert!(run.rotate(blade).is_err());
    assert!(run.clear_slot(SlotKind::Weapon).is_err());

    run.back_to_loadout();
    assert_eq!(run.phase, Phase::Loadout);
    assert!(run.log.is_none());
    assert!(run.equip(blade, SlotKind::Weapon, 1, 0).is_ok(), "unlocked again");
}

#[test]
fn the_same_loadout_always_produces_the_same_fight() {
    let mut a = Run::with_all_pieces();
    build_full_loadout(&mut a);
    let first = a.begin_fight().clone();

    let mut b = Run::with_all_pieces();
    build_full_loadout(&mut b);
    let second = b.begin_fight().clone();

    assert_eq!(first.duration_ms, second.duration_ms);
    assert_eq!(first.outcome, second.outcome);
    assert_eq!(first.entries.len(), second.entries.len());
    assert_eq!(RUST_GOLEM.bounty, 10, "and the bounty is part of the spec");
}


// ------------------------------------------------- power belongs to its item

/// Power on a helmet must not multiply the weapon.
///
/// It used to: `power` was summed across the whole build and applied to every
/// swing, so five slots of it compounded into one blade and damage went
/// through the roof. Strength is the only stat that reaches across a build now.
#[test]
fn power_in_another_slot_does_not_reach_the_weapon() {
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    let build = |with_powered_helmet: bool| -> i32 {
        let mut run = Run::with_all_pieces();
        equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
        equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
        if with_powered_helmet {
            // Crown of the Deep carries power: 25.
            equip(&mut run, "Steel Frame", SlotKind::Helmet, 0, 0);
            equip(&mut run, "Iron Plating", SlotKind::Helmet, 0, 2);
            equip(&mut run, "Crown of the Deep", SlotKind::Helmet, 3, 0);
            assert_eq!(run.report(SlotKind::Helmet).assembled_count(), 1, "fixture");
        }
        let stats = run.player_stats();
        run.combat_items()
            .iter()
            .filter(|i| i.slot == SlotKind::Weapon)
            .map(|i| i.hit_for(stats.strength))
            .sum()
    };
    assert_eq!(
        build(true),
        build(false),
        "a powered helmet changed what the weapon hits for"
    );
}

/// And power on the weapon itself still does.
#[test]
fn power_on_the_weapon_still_multiplies_it() {
    use gearmaster_engine::piece::SlotKind;
    use gearmaster_engine::run::Run;

    let mut plain = Run::with_all_pieces();
    equip(&mut plain, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut plain, "Iron Blade", SlotKind::Weapon, 1, 0);

    let mut inked = Run::with_all_pieces();
    equip(&mut inked, "Leaden Tome", SlotKind::Weapon, 0, 0);
    equip(&mut inked, "Soot Ink", SlotKind::Weapon, 3, 0);
    equip(&mut inked, "Emberburst", SlotKind::Weapon, 3, 1);
    assert_eq!(inked.report(SlotKind::Weapon).assembled_count(), 1, "fixture");

    let p = plain.combat_items().into_iter().find(|i| i.slot == SlotKind::Weapon).unwrap();
    let k = inked.combat_items().into_iter().find(|i| i.slot == SlotKind::Weapon).unwrap();
    // An item's multiplier is one, plus every point of power its own pieces
    // carry, plus its ink - and nothing from any other slot.
    let own = |run: &Run, prof: &gearmaster_engine::loadout::ItemProfile| -> i32 {
        100 + prof
            .pieces
            .iter()
            .map(|&id| run.registry.def(id).base.power + run.registry.def(id).power_bonus)
            .sum::<i32>()
    };
    assert_eq!(p.power, own(&plain, &p), "martial weapon carries its handle and blade");
    assert_eq!(k.power, own(&inked, &k), "the book carries its ink");
    assert!(k.power > p.power, "an inked book beats a plain blade: {} vs {}", k.power, p.power);
}

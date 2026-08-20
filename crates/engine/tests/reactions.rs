//! Gear that reacts to other gear: touching neighbours, cross-slot alignment,
//! and the rule that unassembled gear never acts.

mod common;

use common::equip;
use gearmaster_engine::combat::{simulate, Event, MonsterSpec, Side};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::piece::{Action, SlotKind, Trigger};
use gearmaster_engine::run::Run;
use gearmaster_engine::stats::Stats;

const DUMMY: MonsterSpec = MonsterSpec {
    name: "Dummy",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,
    curse_resist: 0,
    attacks: &[],
    gear: &[],
    bounty: 0,
};

fn item(name: &str, slot: SlotKind, cooldown_ms: u32, stats: Stats) -> ItemProfile {
    ItemProfile {
        name: name.to_string(),
        full_name: name.to_string(),
        core: name.to_string(),
        slot,
        cooldown_ms,
        stats,
        triggers: Vec::new(),
        adjacent_assembled_same_slot: 0,
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    }
}

fn activations(log: &gearmaster_engine::combat::CombatLog, name: &str) -> Vec<u32> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Activate { side: Side::Player, item, .. } if item == name => Some(e.at_ms),
            _ => None,
        })
        .collect()
}

// -------------------------------------------------- reacting to a neighbour

#[test]
fn a_reactive_item_answers_the_neighbour_it_touches() {
    let driver = item("Driver", SlotKind::Weapon, 1000, Stats::damage(1));
    let mut reactor = item("Reactor", SlotKind::Helmet, 60_000, Stats::ZERO);
    reactor.triggers = vec![Trigger::OnAdjacentActivate(Action::GainMana(3))];
    reactor.adjacent_items = vec![0]; // touching the driver

    let log = simulate(Stats::new(1000, 0, 0, 100), &[driver, reactor], &DUMMY);

    let gains = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::GainMana { side: Side::Player, amount: 3, .. }))
        .count();
    let driver_fired = activations(&log, "Driver").len();
    assert!(driver_fired > 5);
    assert_eq!(gains, driver_fired, "one reaction per neighbour activation");
}

#[test]
fn a_reactive_item_ignores_gear_it_does_not_touch() {
    let stranger = item("Stranger", SlotKind::Weapon, 1000, Stats::damage(1));
    let mut reactor = item("Reactor", SlotKind::Helmet, 60_000, Stats::ZERO);
    reactor.triggers = vec![Trigger::OnAdjacentActivate(Action::GainMana(3))];
    // adjacent_items left empty: it touches nothing.

    let log = simulate(Stats::new(1000, 0, 0, 100), &[stranger, reactor], &DUMMY);

    assert!(
        !log.entries.iter().any(|e| matches!(e.event, Event::GainMana { .. })),
        "nothing is adjacent, so nothing reacts"
    );
}

#[test]
fn reducing_a_cooldown_makes_the_item_fire_sooner() {
    let driver = item("Driver", SlotKind::Weapon, 1000, Stats::damage(1));
    let mut charmed = item("Charmed", SlotKind::Helmet, 4000, Stats::armor(1));
    charmed.triggers = vec![Trigger::OnAdjacentActivate(Action::ReduceCooldown(1000))];
    charmed.adjacent_items = vec![0];

    let with_charm = simulate(Stats::new(1000, 0, 0, 100), &[driver.clone(), charmed], &DUMMY);
    let plain = item("Charmed", SlotKind::Helmet, 4000, Stats::armor(1));
    let without = simulate(Stats::new(1000, 0, 0, 100), &[driver, plain], &DUMMY);

    let fast = activations(&with_charm, "Charmed").len();
    let slow = activations(&without, "Charmed").len();
    assert!(
        fast > slow,
        "the charm should get more activations in ({} vs {})",
        fast,
        slow
    );
}

#[test]
fn two_items_reacting_to_each_other_do_not_loop() {
    // Both react to the other. A reaction must not itself count as an
    // activation, or this would recurse until the stack gives out.
    let mut a = item("A", SlotKind::Weapon, 1000, Stats::damage(1));
    a.triggers = vec![Trigger::OnAdjacentActivate(Action::GainMana(1))];
    a.adjacent_items = vec![1];
    let mut b = item("B", SlotKind::Helmet, 1000, Stats::armor(1));
    b.triggers = vec![Trigger::OnAdjacentActivate(Action::GainMana(1))];
    b.adjacent_items = vec![0];

    let log = simulate(Stats::new(1000, 0, 0, 100), &[a, b], &DUMMY);

    // Terminating at all is most of the point; the counts should also be sane.
    let gains = log
        .entries
        .iter()
        .filter(|e| matches!(e.event, Event::GainMana { side: Side::Player, .. }))
        .count();
    let fired = activations(&log, "A").len() + activations(&log, "B").len();
    assert_eq!(gains, fired, "exactly one reaction per activation, no cascade");
}

// ------------------------------------------------------- cross-slot alignment

#[test]
fn alignment_is_computed_from_the_rows_two_items_occupy() {
    let mut run = Run::with_all_pieces();
    // A weapon across rows 0-3...
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    // ...and gloves on the same rows in a different grid.
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Channeling Mold", SlotKind::Gloves, 2, 0);

    let items = run.combat_items();
    assert_eq!(items.len(), 2);
    let gloves = items.iter().find(|i| i.slot == SlotKind::Gloves).unwrap();
    assert_eq!(gloves.aligned_items.len(), 1, "the weapon shares its rows");
    assert!(gloves.adjacent_items.is_empty(), "different grids never touch");
}

#[test]
fn moving_gear_out_of_line_breaks_the_alignment() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0); // rows 0-2
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0); // rows 0-3
    // Gloves pushed down to rows 5-6, clear of the weapon.
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 5);
    equip(&mut run, "Channeling Mold", SlotKind::Gloves, 2, 5);

    let items = run.combat_items();
    let gloves = items.iter().find(|i| i.slot == SlotKind::Gloves).unwrap();
    assert!(gloves.aligned_items.is_empty(), "rows 5-6 do not meet rows 0-3");
}

#[test]
fn aligned_gloves_bank_mana_whenever_the_weapon_swings() {
    let mut run = Run::with_all_pieces();
    equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);
    equip(&mut run, "Leather Material", SlotKind::Gloves, 0, 0);
    equip(&mut run, "Channeling Mold", SlotKind::Gloves, 2, 0);

    let log = run.fight_next().clone();
    assert!(
        log.entries
            .iter()
            .any(|e| matches!(e.event, Event::GainMana { side: Side::Player, amount: 1, .. })),
        "the channelling mold should be earning mana off the weapon"
    );
}

// ------------------------------------------- unassembled gear stays inert

#[test]
fn an_unassembled_item_gives_passives_but_never_acts() {
    let mut run = Run::with_all_pieces();
    // A handle with no damaging piece: never a weapon, so never a swing.
    equip(&mut run, "Cursed Handle", SlotKind::Weapon, 0, 0);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 0);
    assert_eq!(report.stats.power, 30, "its passive power still counts");
    assert!(run.combat_items().is_empty(), "but it takes no part in the fight");

    let log = run.fight_next().clone();
    assert!(
        !log.entries
            .iter()
            .any(|e| matches!(e.event, Event::Activate { side: Side::Player, .. })),
        "nothing of the player's should activate"
    );
    assert!(
        !log.entries.iter().any(|e| matches!(e.event, Event::ManaCheck { .. })),
        "and its mana trigger must stay silent"
    );
}

#[test]
fn an_oversized_piece_pays_off_precisely_because_it_cannot_be_built() {
    let mut run = Run::with_all_pieces();
    // The Vast Tapestry is a 5x4 slab: a base cannot fit beside it.
    equip(&mut run, "Vast Tapestry", SlotKind::Chest, 0, 0);

    let loose = run.report(SlotKind::Chest);
    assert_eq!(loose.assembled_count(), 0);
    assert_eq!(loose.stats.health, 76, "6 base + 70 while unbound");
    assert_eq!(loose.stats.armor, 12);

    // Finish the chestpiece around it and the bonus switches off.
    equip(&mut run, "Hide Base", SlotKind::Chest, 0, 4);
    let built = run.report(SlotKind::Chest);
    assert_eq!(built.assembled_count(), 1);
    assert_eq!(built.stats.health, 6 + 14, "the unbound bonus is gone");
}

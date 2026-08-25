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
    let driver = item("Driver", SlotKind::Weapon, 1000, Stats::physical(1));
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
    let stranger = item("Stranger", SlotKind::Weapon, 1000, Stats::physical(1));
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
    let driver = item("Driver", SlotKind::Weapon, 1000, Stats::physical(1));
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
    let mut a = item("A", SlotKind::Weapon, 1000, Stats::physical(1));
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
    assert_eq!(loose.stats.health, 580, "30 base + 550 while unbound");
    // Its unbound bonus is deliberately *not* armour: loose gear never
    // activates, and armour only accrues on activation, so armour on a piece
    // that can never be built would be worth nothing at all.
    assert_eq!(loose.stats.armor, 0);

    // Finish the chestpiece around it and the bonus switches off.
    equip(&mut run, "Hide Base", SlotKind::Chest, 0, 4);
    let built = run.report(SlotKind::Chest);
    assert_eq!(built.assembled_count(), 1);
    assert_eq!(built.stats.health, 30 + 70, "the unbound bonus is gone");
}

// -------------------------------------------------- the two mana buffs

#[test]
fn mana_empowerment_scales_power_with_the_mana_you_still_hold() {
    use gearmaster_engine::combat::Combatant;
    let mut c = Combatant::player(Stats::new(100, 10, 0, 100), &[]);
    c.mana = 20;
    assert_eq!(c.effective_power(), 100, "no stacks, no bonus");

    c.empowerment = 1;
    assert_eq!(c.effective_power(), 200, "0.05x per point of 20 mana = +1.00x");
    c.empowerment = 2;
    assert_eq!(c.effective_power(), 300);

    // Spending the mana that powers it cuts the bonus straight back down.
    c.mana = 5;
    assert_eq!(c.effective_power(), 150, "2 stacks against 5 mana");
}

#[test]
fn a_ward_can_turn_activations_into_deflection_and_the_iron_stops_landing() {
    let mut hitter = item("Hitter", SlotKind::Weapon, 1000, Stats::physical(30));
    hitter.triggers = vec![];
    const PUNCHER: MonsterSpec = MonsterSpec {
        name: "Puncher",
        health: 100_000,
        strength: 0,
        regen: 0,
        mind_resist: 0,    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
        attacks: &[gearmaster_engine::combat::MonsterAttack::hit("jab", 1000, 20)],
        gear: &[],
        gear_offset: 0,
        bounty: 0,
        sprite: gearmaster_engine::combat::MonsterSprite::Rat,
        rank: gearmaster_engine::combat::Rank::Ordinary,
        drops: &[],
        items: &[],
    };

    // A battery to bank mana, and a ward that turns it into Deflection.
    //
    // It used to bank a mana shield, and the puncher's jab is physical - a
    // monster's innate attack has no slot and counts as a weapon swing. Since
    // the lanes were separated the shield does not answer iron at all, so the
    // fixture reads the twin instead. What the test is about is unchanged: a
    // ward that spends its way into mitigation, and mitigation arriving on the
    // blow rather than in the log.
    let battery = item("Battery", SlotKind::Chest, 500, Stats::mana(4));
    let mut ward = item("Ward", SlotKind::Helmet, 600, Stats::ZERO);
    ward.triggers = vec![Trigger::SpendMana {
        cost: 3,
        on_success: Action::GainDeflection(1),
        on_failure: Action::GainArmor(0),
    }];

    let log = simulate(Stats::new(2000, 0, 0, 100), &[battery, ward], &PUNCHER);
    assert!(
        log.entries.iter().any(|e| matches!(e.event, Event::Deflecting { .. })),
        "the ward should be converting mana into deflection"
    );

    // Once shielded, the puncher's 20s stop getting through in full.
    let late: Vec<i32> = log
        .entries
        .iter()
        .filter(|e| e.at_ms > 20_000)
        .filter_map(|e| match e.event {
            Event::Hit { by: Side::Enemy, damage, .. } => Some(damage),
            _ => None,
        })
        .collect();
    assert!(!late.is_empty());
    assert!(
        late.iter().all(|&d| d == 20),
        "the log reports the swing, mitigation happens on arrival"
    );
    // Health should be barely touched compared with 20 a second unmitigated.
    //
    // Read before sudden death, which takes a growing share of maximum health
    // off both sides from thirty seconds and does not care what you are
    // wearing. That is the point of that rule, and it is not what this test is
    // about - measuring after it reads the overtime rather than the shield.
    let hp_before_overtime = log
        .entries
        .iter()
        .filter(|e| e.at_ms < gearmaster_engine::combat::SUDDEN_DEATH_MS)
        .rev()
        .find_map(|e| match e.event {
            Event::Hit { by: Side::Enemy, target_health, .. } => Some(target_health),
            _ => None,
        })
        .expect("the puncher landed something in the first thirty seconds");
    assert!(
        hp_before_overtime > 1500,
        "deflection should have turned most of it, hp {}",
        hp_before_overtime
    );
}

#[test]
fn a_ward_that_cannot_pay_falls_back_instead_of_stacking() {
    let mut ward = item("Ward", SlotKind::Helmet, 600, Stats::ZERO);
    ward.triggers = vec![Trigger::SpendMana {
        cost: 3,
        on_success: Action::GainShield(1),
        on_failure: Action::GainArmor(5),
    }];
    // No mana income at all.
    let log = simulate(Stats::new(2000, 0, 0, 100), &[ward], &DUMMY);
    assert!(
        !log.entries.iter().any(|e| matches!(e.event, Event::Shielded { .. })),
        "nothing to spend, so nothing to stack"
    );
    assert!(log.entries.iter().any(|e| matches!(e.event, Event::GainArmor { .. })));
}

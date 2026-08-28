//! Three lanes, and each one keeps to itself.
//!
//! Empowerment and the mana shield used to reach everything: empowerment
//! multiplied every swing whatever it was made of, and the shield came off any
//! incoming number at all - magic, iron and mind alike. That made mana the
//! answer to three questions and left the other two lanes with none of their
//! own, which is the thing this file pins closed.
//!
//! The rule now, and the whole of it:
//!
//! | lane | what sharpens it | what answers it |
//! |---|---|---|
//! | magic | mana empowerment | the mana shield |
//! | physical | Spellblade | Deflection |
//! | mind | Dread, later | `mind_resist`, and nothing else |
//!
//! The two pairs are deliberately the same shape and deliberately not the same
//! bargain. The mana pair scales off held mana, so it has a ceiling worth
//! building towards and a pool worth keeping full. The twins are flat, so they
//! ask for nothing and never get better. `SPELLBLADE_POWER` and
//! `DEFLECTION_FLAT` are both set where a stack of the twin equals a stack of
//! its cousin at ten mana, which is where the two bargains cross.

mod common;

use gearmaster_engine::combat::{
    simulate, Combatant, DamageType, Event, MonsterAttack, MonsterSpec, MonsterSprite, Rank, Side,
    DEFLECTION_FLAT, SPELLBLADE_POWER,
};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::piece::{Action, SlotKind, Trigger};
use gearmaster_engine::stats::Stats;

/// Something that hits back, so a defensive stack has work to do. Its jab is
/// innate, and an innate attack has no slot, so it swings as a weapon - which
/// makes it physical.
const PUNCHER: MonsterSpec = MonsterSpec {
    name: "Puncher",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,
    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
    attacks: &[MonsterAttack::hit("jab", 1000, 40)],
    gear: &[],
    gear_offset: 0,
    bounty: 0,
    sprite: MonsterSprite::Rat,
    rank: Rank::Ordinary,
    drops: &[],
    items: &[],
};

/// A sandbag, so a swing can be measured without anything answering it.
const DUMMY: MonsterSpec = MonsterSpec { attacks: &[], ..PUNCHER };

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
        attracts_curses: false,
        steady: false,
        overtakes: false,
        wrong_sense: false,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    }
}

/// Every blow the player landed, in order.
fn swings(log: &gearmaster_engine::combat::CombatLog) -> Vec<i32> {
    log.entries
        .iter()
        .filter_map(|e| match e.event {
            Event::Hit { by: Side::Player, damage, .. } => Some(damage),
            _ => None,
        })
        .collect()
}

/// The player's health the last time the puncher landed before overtime.
///
/// Read before sudden death, which takes a growing share of maximum health off
/// both sides from thirty seconds and does not care what anybody is wearing.
fn health_before_overtime(log: &gearmaster_engine::combat::CombatLog) -> i32 {
    log.entries
        .iter()
        .filter(|e| e.at_ms < gearmaster_engine::combat::SUDDEN_DEATH_MS)
        .rev()
        .find_map(|e| match e.event {
            Event::Hit { by: Side::Enemy, target_health, .. } => Some(target_health),
            _ => None,
        })
        .expect("the puncher landed something in the first thirty seconds")
}

// ------------------------------------------------------ the two amplifiers

#[test]
fn empowerment_is_the_magic_lane_and_spellblade_is_the_iron() {
    let mut c = Combatant::player(Stats::new(100, 10, 0, 100), &[]);
    c.mana = 20;

    // Nothing banked: both lanes swing at the item's own power.
    assert_eq!(c.effective_power(), 100);
    assert_eq!(c.effective_physical_power(), 100);

    c.empowerment = 2;
    assert_eq!(c.effective_power(), 300, "0.05x per point of 20 mana, twice over");
    assert_eq!(c.effective_physical_power(), 100, "iron does not feel it");

    c.spellblade = 2;
    assert_eq!(c.effective_physical_power(), 100 + 2 * SPELLBLADE_POWER);
    assert_eq!(c.effective_power(), 300, "and magic does not feel that");

    // Spending the mana cuts empowerment down and leaves Spellblade where it
    // was, which is the whole difference between the two.
    c.mana = 0;
    assert_eq!(c.effective_power(), 100);
    assert_eq!(c.effective_physical_power(), 100 + 2 * SPELLBLADE_POWER);
}

#[test]
fn a_board_stacking_empowerment_swings_iron_no_harder_than_one_that_is_not() {
    // A blade, a battery, and a crown that turns mana into empowerment. The
    // blade deals physical and nothing else, so the empowerment has nothing of
    // its own to multiply.
    let blade = item("Blade", SlotKind::Weapon, 1000, Stats::physical(50));
    let battery = item("Battery", SlotKind::Chest, 500, Stats::mana(4));
    let mut crown = item("Crown", SlotKind::Helmet, 600, Stats::ZERO);
    crown.triggers = vec![Trigger::OnActivate(Action::GainEmpowerment(1))];

    let with = simulate(Stats::new(2000, 0, 0, 100), &[blade.clone(), battery.clone(), crown], &DUMMY);
    let without = simulate(Stats::new(2000, 0, 0, 100), &[blade, battery], &DUMMY);

    assert!(
        with.entries.iter().any(|e| matches!(e.event, Event::Empowered { .. })),
        "the crown should be banking empowerment"
    );
    let (a, b) = (swings(&with), swings(&without));
    assert!(!a.is_empty() && !b.is_empty());
    assert_eq!(
        a.iter().max(),
        b.iter().max(),
        "empowerment reached a physical swing, which is the thing A1 removes"
    );
}

#[test]
fn spellblade_sharpens_that_same_swing() {
    let blade = item("Blade", SlotKind::Weapon, 1000, Stats::physical(50));
    let mut glove = item("Glove", SlotKind::Gloves, 600, Stats::ZERO);
    glove.triggers = vec![Trigger::OnActivate(Action::GainSpellblade(1))];

    let with = simulate(Stats::new(2000, 0, 0, 100), &[blade.clone(), glove], &DUMMY);
    let without = simulate(Stats::new(2000, 0, 0, 100), &[blade], &DUMMY);

    assert!(
        with.entries.iter().any(|e| matches!(e.event, Event::Whetted { .. })),
        "the glove should be banking spellblade"
    );
    let (a, b) = (swings(&with), swings(&without));
    assert!(
        a.iter().max() > b.iter().max(),
        "spellblade did not reach the swing: {:?} against {:?}",
        a.iter().max(),
        b.iter().max()
    );
}

// ------------------------------------------------------- the two mitigations

#[test]
fn the_shield_answers_magic_and_deflection_answers_iron() {
    let stacked = |shield: u32, deflection: u32| {
        let mut c = Combatant::player(Stats::new(10_000, 0, 0, 100), &[]);
        c.mana = 20;
        c.shield = shield;
        c.deflection = deflection;
        c
    };

    // A shield of one stack against twenty mana is twenty points, and it takes
    // them off magic only.
    let mut c = stacked(1, 0);
    assert_eq!(c.damage_reduction(), 20);
    assert_eq!(c.physical_reduction(), 0);
    assert_eq!(c.take_typed(100, DamageType::Magic, 0).1, 80);
    assert_eq!(c.take_typed(100, DamageType::Physical, 0).1, 100, "iron walks past it");

    // Deflection is the mirror: flat, no mana, physical only.
    let mut c = stacked(0, 2);
    assert_eq!(c.physical_reduction(), 2 * DEFLECTION_FLAT);
    assert_eq!(c.damage_reduction(), 0);
    assert_eq!(c.take_typed(100, DamageType::Physical, 0).1, 100 - 2 * DEFLECTION_FLAT);
    assert_eq!(c.take_typed(100, DamageType::Magic, 0).1, 100, "the spell walks past it");
}

#[test]
fn deflection_is_taken_before_armour_the_way_the_shield_is() {
    let mut c = Combatant::player(Stats::new(10_000, 0, 0, 100), &[]);
    c.armor = 1_000;
    c.deflection = 3;
    // Thirty turned, seventy eaten by the armour, nothing through to health.
    let (absorbed, through) = c.take_typed(100, DamageType::Physical, 0);
    assert_eq!(absorbed, 100 - 3 * DEFLECTION_FLAT);
    assert_eq!(through, 0);
    assert_eq!(c.armor, 1_000 - (100 - 3 * DEFLECTION_FLAT), "the turned share cost no armour");
}

#[test]
fn a_shield_no_longer_saves_a_board_from_a_fist() {
    // The same fixture that used to prove "the shield blunts every kind of
    // damage". It is here to prove the opposite, because that is the change.
    let battery = item("Battery", SlotKind::Chest, 500, Stats::mana(4));
    let mut ward = item("Ward", SlotKind::Helmet, 600, Stats::ZERO);
    ward.triggers = vec![Trigger::SpendMana {
        cost: 3,
        on_success: Action::GainShield(1),
        on_failure: Action::GainArmor(0),
    }];
    let shielded = simulate(Stats::new(4000, 0, 0, 100), &[battery.clone(), ward], &PUNCHER);
    let bare = simulate(Stats::new(4000, 0, 0, 100), &[battery], &PUNCHER);

    assert!(shielded.entries.iter().any(|e| matches!(e.event, Event::Shielded { .. })));
    assert_eq!(
        health_before_overtime(&shielded),
        health_before_overtime(&bare),
        "the mana shield reached a physical jab"
    );
}

// ------------------------------------------------------------- the mind lane

#[test]
fn mind_damage_is_answered_by_mind_resist_and_by_nothing_else() {
    // Mana shield up, and a great deal of it. It does not touch this.
    let mut c = Combatant::player(Stats::new(1_000, 0, 0, 100), &[]);
    c.mana = 40;
    c.shield = 4;
    c.deflection = 9;
    assert_eq!(c.damage_reduction(), 160, "there is plenty of shield to ignore");
    assert_eq!(c.take_mind(100), 100, "all of it landed");
    assert_eq!(c.max_health, 900);

    // Resistance is the lane's own answer, and it is the helmet's.
    let mut c = Combatant::player(Stats::new(1_000, 0, 0, 100), &[]);
    c.mind_resist = 50;
    let dealt = c.take_mind(100);
    assert!(dealt < 100, "mind resistance did nothing: {}", dealt);
    assert_eq!(c.max_health, 1_000 - dealt);
}

// ------------------------------------------------------- and the twins reset

#[test]
fn neither_twin_survives_the_fight_that_banked_it() {
    // Every counter in this game is per-fight, and a stack bought in one is
    // not a stack owned in the next. Read straight off a fresh combatant,
    // because that is the only place a fight starts from.
    let c = Combatant::player(Stats::new(100, 0, 0, 100), &[]);
    assert_eq!(c.spellblade, 0);
    assert_eq!(c.deflection, 0);
    assert_eq!(c.empowerment, 0);
    assert_eq!(c.shield, 0);
}

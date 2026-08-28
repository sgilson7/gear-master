//! The third lane's pool, and the fact that nothing can reach it yet.
//!
//! Insight is the eighth resource and it is deliberately the strangest one.
//! Three of the eight are fusions, which nothing spends; four are the pools a
//! trigger may ask for. Insight is neither. It is **fuel**, on exactly mana's
//! terms - holding it pays nothing at all, and what it is worth depends
//! entirely on the stacks standing on it - and it is the only resource in the
//! game a run has to be *given* before it exists.
//!
//! This file is mostly about that second half. The mechanic ships whole and
//! dark: `Run::insight_unlocked` is false, `Shop::insight_open` is false, and
//! no component in the catalogue banks a point of it. The Insight gear family
//! arrives with the rest of the mission's catalogue (M9) and the tests below
//! that read `CATALOG` are written to become real on the day it does rather
//! than to be rewritten then.

mod common;

use gearmaster_engine::combat::{
    simulate, Combatant, Event, MonsterSpec, MonsterSprite, Rank, Side,
    DREAD_DIVISOR,
};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::piece::{
    touches_insight, Action, PieceDef, PieceKind, Resource, SlotKind, Target, Trigger, CATALOG,
};
use gearmaster_engine::run::Run;
use gearmaster_engine::stats::Stats;

const DUMMY: MonsterSpec = MonsterSpec {
    name: "Dummy",
    health: 100_000,
    strength: 0,
    regen: 0,
    mind_resist: 0,
    physical_resist: 0,
    magic_resist: 0,
    curse_resist: 0,
    attacks: &[],
    gear: &[],
    gear_offset: 0,
    bounty: 0,
    sprite: MonsterSprite::Rat,
    rank: Rank::Ordinary,
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

/// Maximum health removed over a whole fight.
fn mind_dealt(log: &gearmaster_engine::combat::CombatLog) -> i32 {
    log.entries
        .iter()
        .filter_map(|e| match e.event {
            Event::MindHit { by: Side::Player, amount, .. } => Some(amount),
            _ => None,
        })
        .sum()
}

// --------------------------------------------------------- the eighth pool

#[test]
fn every_table_that_knows_about_resources_knows_about_this_one() {
    assert_eq!(Resource::ALL.len(), 8);
    assert!(Resource::ALL.contains(&Resource::Insight));
    assert_eq!(Resource::Insight.index(), 7);
    assert_eq!(Resource::Insight.name(), "insight");
    assert_eq!(Resource::by_name("insight"), Some(Resource::Insight));

    // Every index is distinct and inside the array the run banks into.
    let mut seen: Vec<usize> = Resource::ALL.into_iter().map(|r| r.index()).collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..8).collect::<Vec<_>>());

    // Not a fusion, and not made of anything.
    assert!(!Resource::Insight.is_fused());
    assert_eq!(Resource::Insight.parents(), None);
    // And not spendable in v1: nothing asks for it, it only feeds Dread.
    assert!(!Resource::SPENDABLE.contains(&Resource::Insight));
}

#[test]
fn the_run_can_bank_the_eighth_without_running_off_the_end_of_the_array() {
    // `banked_all_run` was `[i32; 4]` against an index that already ran to
    // six. Nothing wrote past the end - a fusion has an event of its own - but
    // that is a fact about today's actions rather than about the array.
    let mut run = Run::new();
    for r in Resource::ALL {
        run.banked_all_run[r.index()] += 1;
    }
    assert_eq!(run.banked_all_run, [1; 8]);
}

#[test]
fn holding_insight_pays_absolutely_nothing() {
    // The point of the pool, stated as a test so that giving it a passive
    // rate later has to come through here and argue for it.
    let mut c = Combatant::player(Stats::new(100, 0, 0, 100), &[]);
    let bare = c.held_bonus();
    c.insight = 40;
    assert_eq!(c.held_bonus(), bare, "insight is fuel, like mana, and pays nothing held");
    assert_eq!(c.pool(Resource::Insight), 40);
    c.set_pool(Resource::Insight, 7);
    assert_eq!(c.insight, 7);
}

// ------------------------------------------------------------ dread and it

#[test]
fn a_stack_is_worth_nothing_without_the_pool_and_the_pool_nothing_without_a_stack() {
    let mut c = Combatant::player(Stats::new(100, 0, 0, 100), &[]);
    assert_eq!(c.mind_bonus(), 0);
    c.dread = 4;
    assert_eq!(c.mind_bonus(), 0, "four stacks on an empty pool");
    c.dread = 0;
    c.insight = 40;
    assert_eq!(c.mind_bonus(), 0, "a full pool nobody is reading");
    c.dread = 4;
    assert_eq!(c.mind_bonus(), 4 * 40 / DREAD_DIVISOR);
}

#[test]
fn dread_reaches_the_mind_damage_an_item_deals() {
    let whisper = item("Whisper", SlotKind::Helmet, 1000, Stats { mind: 5, ..Stats::ZERO });
    let mut crown = item("Crown", SlotKind::Helmet, 900, Stats::ZERO);
    crown.triggers = vec![
        Trigger::OnActivate(Action::Gain { what: Resource::Insight, amount: 4 }),
        Trigger::OnActivate(Action::GainDread(1)),
    ];

    let with = simulate(Stats::new(2000, 0, 0, 100), &[whisper.clone(), crown], &DUMMY);
    let without = simulate(Stats::new(2000, 0, 0, 100), &[whisper], &DUMMY);

    assert!(with.entries.iter().any(|e| matches!(e.event, Event::Dreading { .. })));
    assert!(
        mind_dealt(&with) > mind_dealt(&without),
        "dread did not reach the whisper: {} against {}",
        mind_dealt(&with),
        mind_dealt(&without)
    );
}

#[test]
fn dread_reaches_a_mind_damage_action_as_well() {
    // Two routes to mind damage - a piece's `mind` stat and the action - and a
    // bonus that only reached one of them would be a lane with a hole in it.
    let mut sting = item("Sting", SlotKind::Helmet, 1000, Stats::ZERO);
    sting.triggers =
        vec![Trigger::OnActivate(Action::MindDamage { amount: 5, target: Target::Enemy })];
    let mut crown = item("Crown", SlotKind::Helmet, 900, Stats::ZERO);
    crown.triggers = vec![
        Trigger::OnActivate(Action::Gain { what: Resource::Insight, amount: 4 }),
        Trigger::OnActivate(Action::GainDread(1)),
    ];

    let with = simulate(Stats::new(2000, 0, 0, 100), &[sting.clone(), crown], &DUMMY);
    let without = simulate(Stats::new(2000, 0, 0, 100), &[sting], &DUMMY);
    assert!(mind_dealt(&with) > mind_dealt(&without));
}

#[test]
fn insight_is_a_pool_a_drain_can_take() {
    // The counterplay doctrine the fused pools already live under: anything
    // worth banking is worth somebody taking off you.
    let mut c = Combatant::player(Stats::new(100, 0, 0, 100), &[]);
    c.insight = 12;
    c.dread = 2;
    assert_eq!(c.mind_bonus(), 12);
    c.set_pool(Resource::Insight, 0);
    assert_eq!(c.mind_bonus(), 0, "drained, and the stacks are left holding nothing");
}

#[test]
fn neither_survives_the_fight_that_banked_it() {
    let c = Combatant::player(Stats::new(100, 0, 0, 100), &[]);
    assert_eq!(c.insight, 0);
    assert_eq!(c.dread, 0);
}

// ------------------------------------------------------------- and the gate

#[test]
fn a_fresh_run_has_not_earned_it() {
    let run = Run::new();
    assert!(!run.insight_unlocked);
    assert!(!run.shop.insight_open);
}

#[test]
fn clearing_the_threshold_opens_the_shelf_as_well_as_the_flag() {
    let mut run = Run::new();
    run.unlock_insight();
    assert!(run.insight_unlocked);
    assert!(run.shop.insight_open, "the run learned it and the shop did not");
}

#[test]
fn the_predicate_knows_both_halves_of_the_lane() {
    const BANKS: &[Trigger] =
        &[Trigger::OnActivate(Action::Gain { what: Resource::Insight, amount: 1 })];
    const STACKS: &[Trigger] = &[Trigger::OnActivate(Action::GainDread(1))];
    const NEITHER: &[Trigger] =
        &[Trigger::OnActivate(Action::Gain { what: Resource::Nature, amount: 1 })];
    const NESTED: &[Trigger] = &[Trigger::SpendMana {
        cost: 3,
        on_success: Action::GainDread(1),
        on_failure: Action::GainMana(1),
    }];

    let def = |triggers: &'static [Trigger]| PieceDef {
        name: "probe",
        slot: SlotKind::Helmet,
        kind: PieceKind::Crest,
        cells: &[(0, 0)],
        base: Stats::ZERO,
        assembly_bonus: None,
        effect: None,
        cooldown_ms: 0,
        quest: None,
        power_bonus: 0,
        speed_bonus: 0,
        triggers,
        price: 1,
    };
    assert!(touches_insight(&def(BANKS)));
    assert!(touches_insight(&def(STACKS)));
    assert!(touches_insight(&def(NESTED)), "a gated grant is still a grant");
    assert!(!touches_insight(&def(NEITHER)));
}

#[test]
fn nothing_that_deals_in_the_pool_reaches_a_locked_shelf() {
    // Vacuous today and deliberately written to stop being vacuous: the
    // Insight family lands with the rest of the mission's catalogue, and this
    // is the assertion that will catch it if the gate is forgotten.
    use gearmaster_engine::rng::Rng;
    use gearmaster_engine::shop::Shop;
    for seed in 0..200u64 {
        let mut rng = Rng::new(0x5EED_0000_0000_0000 ^ seed);
        let mut shop = Shop::new(&mut rng);
        assert!(!shop.insight_open, "a shop opens shut");
        for _ in 0..6 {
            for &i in &shop.stock {
                assert!(
                    !touches_insight(&CATALOG[i]),
                    "{} was on a shelf before the pool was earned",
                    CATALOG[i].name
                );
            }
            shop.restock(&mut rng, false);
        }
    }
}

#[test]
fn the_family_has_landed_and_lives_where_the_lane_does() {
    // This replaces a lint that asserted the catalogue carried none of it and
    // asked to be deleted on the day it did. That day was M9.
    let carriers: Vec<&'static gearmaster_engine::piece::PieceDef> =
        CATALOG.iter().filter(|d| touches_insight(d)).collect();
    assert!(carriers.len() >= 8, "the family is {} pieces", carriers.len());
    let elsewhere: Vec<&str> = carriers
        .iter()
        .filter(|d| d.slot != SlotKind::Helmet)
        .map(|d| d.name)
        .collect();
    assert!(
        elsewhere.len() * 5 <= carriers.len(),
        "the lane has spread off the head: {:?}",
        elsewhere
    );
    // And none of it is a floating kind, which could sit in a grid the lane
    // does not belong to.
    for d in &carriers {
        assert!(
            !matches!(d.kind, PieceKind::Material | PieceKind::Plating),
            "{} deals in the mind lane and can float out of the head",
            d.name
        );
    }
}

/// Accruing Insight is income, and income on the mind lane is gated like it.
///
/// Nothing in this mission's content accrues Insight. The gate is written
/// anyway, because a pool locked behind a dungeon has to be locked in every
/// direction it can be reached from, and `touches_insight` is the direction
/// the shelves read.
#[test]
fn accrue_on_insight_is_gated_like_income() {
    use gearmaster_engine::piece::{is_town_stock, Action, PieceDef, PieceKind, Trigger};

    // A definition that exists only here: `touches_insight` reads a `PieceDef`,
    // and M5 is where the catalogue grows. Nothing in `CATALOG` should answer
    // yes to this yet, and the assertion below says so.
    let accruer = PieceDef {
        triggers: &[Trigger::OnActivate(Action::Accrue {
            what: gearmaster_engine::piece::Resource::Insight,
            pct: 10,
        })],
        ..*CATALOG.iter().find(|d| d.kind == PieceKind::Frame).expect("a helmet frame")
    };
    assert!(
        gearmaster_engine::piece::touches_insight(&accruer),
        "an income on Insight has to read as touching it, or the shelf gate opens early"
    );

    // And the mirror: a flat gain of a pool that is not Insight does not.
    let plain = PieceDef {
        triggers: &[Trigger::OnActivate(Action::Accrue {
            what: gearmaster_engine::piece::Resource::Mana,
            pct: 10,
        })],
        ..*CATALOG.iter().find(|d| d.kind == PieceKind::Frame).expect("a helmet frame")
    };
    assert!(!gearmaster_engine::piece::touches_insight(&plain));
    assert!(!is_town_stock(&plain), "and it is not ground, whatever else it is");

    let accruers: Vec<&str> = CATALOG
        .iter()
        .filter(|d| {
            d.triggers.iter().any(|t| {
                let mut found = false;
                gearmaster_engine::piece::walk_actions(t, &mut |a| {
                    found |= matches!(
                        a,
                        Action::Accrue { what: gearmaster_engine::piece::Resource::Insight, .. }
                    );
                });
                found
            })
        })
        .map(|d| d.name)
        .collect();
    assert!(
        accruers.is_empty(),
        "nothing in the catalogue accrues Insight, and these do: {accruers:?}"
    );
}

// ----------------------------------------------- A6: the wrong sense
//
// THE THRESHOLD's shelf sells one crest that is a trade rather than a bonus:
// every point of physical and magic the board would deal is not dealt, and the
// mind lane is multiplied by what was given up.
//
// The trade is the whole design. A version that let the damage land and added
// mind on top would be a free multiplier, and every board in the game would
// wear it.

/// A board wearing it deals no physical and no magic at all.
///
/// Asserted off the **log** rather than off the stat block: the stat block
/// still says what the pieces are worth, and the claim is about what crosses.
#[test]
fn the_wrong_sense_stops_the_damage_it_trades_away() {
    use gearmaster_engine::combat::{simulate_at, Difficulty, Event, Side};
    use gearmaster_engine::piece::CATALOG;

    let crest = CATALOG.iter().find(|d| d.name == "The Wrong Sense").expect("the crest");
    // An effect and not a trigger: the trade is standing, true from the bell,
    // and `OnBattleStart` is the greaves' identity mechanic which a helmet may
    // not borrow. Read off the board the way Overtake is.
    assert!(
        matches!(
            crest.effect.map(|e| e.kind),
            Some(gearmaster_engine::piece::EffectKind::WrongSense)
        ),
        "the crest stopped carrying the trade"
    );

    // Two runs off one board: the same gear, and the crest seated on one.
    let hits = |with_crest: bool| -> (usize, usize) {
        let mut run = Run::with_all_pieces();
        run.apply_preset();
        #[allow(unused_assignments)]
        if with_crest {
            // A whole helmet, because a loose piece is inert - the trade is
            // the item's trigger and an unassembled crest never fires. Frame,
            // plating, crest, which is the recipe.
            let mut run2 = Run::with_all_pieces();
            let id = |r: &Run, n: &str| {
                r.owned.iter().copied().find(|&p| r.registry.def(p).name == n).expect(n)
            };
            let (f, pl, cr) = (
                id(&run2, "Listener's Frame"),
                id(&run2, "Countingstair Plating"),
                id(&run2, "The Wrong Sense"),
            );
            let helmet = gearmaster_engine::piece::SlotKind::Helmet;
            run2.equip(f, helmet, 0, 0).expect("frame seats");
            'seat: for piece in [pl, cr] {
                for y in 0..8u8 {
                    for x in 0..6u8 {
                        if run2.equip(piece, helmet, x, y).is_ok() {
                            continue 'seat;
                        }
                    }
                }
                panic!("could not seat a piece of the probe helmet");
            }
            assert!(
                run2.report(helmet).assembled_count() > 0,
                "the probe helmet never assembled, so the crest never fired"
            );
            run = run2;
        }
        let spec = gearmaster_engine::combat::creature("Cog Priest").expect("exists");
        let log = simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium);
        let swings = log
            .entries
            .iter()
            .filter(|e| matches!(&e.event, Event::Hit { by: Side::Player, .. }))
            .count();
        let minds = log
            .entries
            .iter()
            .filter(|e| matches!(&e.event, Event::MindHit { by: Side::Player, .. }))
            .count();
        (swings, minds)
    };

    let (swings_without, _) = hits(false);
    assert!(swings_without > 0, "the control board never swung, so this proves nothing");
    let (swings_with, _) = hits(true);
    assert_eq!(
        swings_with, 0,
        "the crest is worn and the board still landed {swings_with} blows. It is a \
         free multiplier until the damage actually stops."
    );
}

/// And the multiplier is capped, so a long fight is not a free win.
///
/// An uncapped conversion is a board that gets stronger for every second it
/// fails to kill anything, and `SUDDEN_DEATH_MS` already owns everything past
/// thirty seconds - so an uncapped one would make the clock the only opponent.
#[test]
fn the_wrong_sense_is_capped() {
    use gearmaster_engine::combat::{Combatant, WRONG_SENSE_CAP, WRONG_SENSE_PER, WRONG_SENSE_STEP};
    use gearmaster_engine::stats::Stats;

    let mut c = Combatant::player(Stats { health: 500, ..Stats::ZERO }, &[]);
    assert_eq!(c.wrong_sense_multiplied(10), 10, "it multiplies without the crest");

    c.wrong_sense = true;
    assert_eq!(c.wrong_sense_multiplied(10), 10, "nothing given up, nothing gained");

    c.surrendered = WRONG_SENSE_PER as i64;
    assert_eq!(
        c.wrong_sense_multiplied(100),
        100 + WRONG_SENSE_STEP,
        "one step is not one step"
    );

    // Far past the ceiling, and it stops rather than running away.
    c.surrendered = WRONG_SENSE_PER as i64 * 10_000;
    let at_cap = 100 + WRONG_SENSE_CAP * WRONG_SENSE_STEP;
    assert_eq!(c.wrong_sense_multiplied(100), at_cap, "the cap did not hold");
    c.surrendered = i64::MAX / 2;
    assert_eq!(c.wrong_sense_multiplied(100), at_cap, "it ran away at the top");
}

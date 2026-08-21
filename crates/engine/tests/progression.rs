//! The run loop: gold, the shop, and climbing the monster ladder.

mod common;

use gearmaster_engine::combat::{Outcome, LADDER};
use gearmaster_engine::run::{Run, RuleError, STARTER_KIT};
use gearmaster_engine::shop::{SHOP_SIZE, STARTING_GOLD};

#[test]
fn a_run_opens_with_the_basic_weapon_and_nothing_else() {
    let run = Run::new();
    assert_eq!(run.gold, STARTING_GOLD);
    assert_eq!(run.owned.len(), STARTER_KIT.len());
    for name in STARTER_KIT {
        assert!(
            run.owned.iter().any(|&id| &run.registry.def(id).name == name),
            "missing {} from the starter kit",
            name
        );
    }
    assert_eq!(run.inventory().len(), STARTER_KIT.len(), "and none of it is equipped");
    assert!(
        run.combat_items().is_empty(),
        "nothing acts until you actually place it in the weapon slot"
    );
    assert_eq!(run.shop.stock.len(), SHOP_SIZE);
    assert_eq!(run.rung, 0);
    assert_eq!(run.monster().name, "Cave Rat", "the ladder starts easy");
}

#[test]
fn the_starter_kit_assembles_into_a_working_weapon() {
    use gearmaster_engine::piece::SlotKind;
    let mut run = Run::new();
    common::equip(&mut run, "Oak Handle", SlotKind::Weapon, 0, 0);
    common::equip(&mut run, "Iron Blade", SlotKind::Weapon, 1, 0);

    let report = run.report(SlotKind::Weapon);
    assert_eq!(report.assembled_count(), 1, "{}", report.summary());
    assert_eq!(run.combat_items().len(), 1);

    // And it is enough to take the first rung.
    assert_eq!(run.fight_next().outcome, gearmaster_engine::combat::Outcome::Victory);
}

#[test]
fn leaving_the_starter_weapon_in_bits_loses_to_the_rat() {
    // The pieces are handed over unequipped on purpose: placing them is the
    // first thing the game asks you to do.
    let mut run = Run::new();
    assert_eq!(
        run.fight_next().outcome,
        gearmaster_engine::combat::Outcome::Defeat,
        "an unplaced weapon deals nothing"
    );
}

#[test]
fn the_opening_gold_buys_a_working_weapon() {
    // The shelves guarantee a handle and a damaging piece; the starting purse
    // has to cover the cheapest pair of them, or a run is dead on arrival.
    use gearmaster_engine::piece::{PieceKind, CATALOG, SlotKind};
    let cheapest = |kind: PieceKind| {
        CATALOG
            .iter()
            .filter(|d| d.slot == SlotKind::Weapon && d.kind == kind)
            .map(|d| d.price)
            .min()
            .unwrap()
    };
    let floor = cheapest(PieceKind::Handle) + cheapest(PieceKind::Damaging);
    assert!(
        STARTING_GOLD >= floor,
        "{} gold cannot buy the cheapest weapon ({})",
        STARTING_GOLD,
        floor
    );
}

#[test]
fn rerolling_costs_gold_and_changes_the_shelves() {
    let mut run = Run::new();
    let before = run.shop.stock.clone();
    let gold = run.gold;

    run.reroll().expect("affordable");

    assert_eq!(run.gold, gold - gearmaster_engine::shop::REROLL_COST);
    assert_ne!(run.shop.stock, before);

    run.gold = 0;
    assert!(run.reroll().is_err(), "and it is not free");
}

// ------------------------------------------------------------------ shop

#[test]
fn buying_costs_gold_and_hands_over_the_component() {
    let mut run = Run::new();
    run.gold = 400; // strong shelves cost real money now
    let price = run.shop.price(0).unwrap();
    let name = run.shop.def(0).unwrap().name;
    let before_gold = run.gold;
    let before_owned = run.owned.len();

    let id = run.buy(0).expect("affordable");

    assert_eq!(run.gold, before_gold - price);
    assert_eq!(run.owned.len(), before_owned + 1);
    assert_eq!(run.registry.def(id).name, name);
    assert!(run.inventory().contains(&id), "it lands in the inventory unequipped");
    assert_eq!(run.shop.stock.len(), SHOP_SIZE - 1, "and off the shelf");
}

#[test]
fn you_cannot_buy_what_you_cannot_afford() {
    let mut run = Run::new();
    run.gold = 0;
    let price = run.shop.price(0).unwrap();

    let err = run.buy(0).unwrap_err();

    assert_eq!(err, RuleError::NotEnoughGold { need: price, have: 0 });
    assert_eq!(run.shop.stock.len(), SHOP_SIZE, "a refused sale leaves the shelf alone");
    assert_eq!(run.gold, 0);
}

#[test]
fn buying_from_an_empty_shelf_is_refused() {
    let mut run = Run::new();
    run.gold = 1000;
    while !run.shop.is_empty() {
        run.buy(0).expect("plenty of gold");
    }
    assert_eq!(run.buy(0).unwrap_err(), RuleError::NothingThere);
}

#[test]
fn selling_refunds_half_and_strips_the_piece_off() {
    use gearmaster_engine::piece::SlotKind;
    let mut run = Run::with_all_pieces();
    let id = common::piece(&run, "Oak Handle");
    run.equip(id, SlotKind::Weapon, 0, 0).unwrap();
    let price = gearmaster_engine::rating::shop_price(run.registry.def(id));
    let before = run.gold;

    let refund = run.sell(id).unwrap();

    assert_eq!(refund, price / 2);
    assert_eq!(run.gold, before + refund);
    assert!(!run.is_equipped(id), "sold gear comes off");
    assert!(!run.owned.contains(&id));
}

// ------------------------------------------------------------- the ladder

#[test]
fn the_ladder_climbs_all_the_way_up() {
    assert_eq!(LADDER.len(), 33, "eleven monsters, two bosses, and twenty beyond");
    let bounties: Vec<i32> = LADDER.iter().map(|m| m.bounty).collect();
    assert!(
        bounties.windows(2).all(|w| w[0] <= w[1]),
        "bounties should not go down as the ladder gets harder: {:?}",
        bounties
    );
    // Every one of them must be able to act, whether by tooth or by gear.
    for m in LADDER {
        assert!(
            !m.attacks.is_empty() || !m.gear.is_empty(),
            "{} has neither attacks nor gear",
            m.name
        );
        assert!(m.health > 0, "{} has no health", m.name);
        for a in m.attacks {
            assert!(a.cooldown_ms > 0, "{}'s {} never fires", m.name, a.name);
        }
    }
}

#[test]
fn winning_pays_the_bounty_and_moves_you_up() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let bounty = run.monster().bounty;
    let gold_before = run.gold;

    let outcome = run.fight_next().outcome;
    assert_eq!(outcome, Outcome::Victory, "a full preset beats a cave rat");
    let reward = run.settle();

    assert_eq!(reward, Some(bounty));
    assert_eq!(run.gold, gold_before + bounty);
    assert_eq!(run.wins, 1);
    assert_eq!(run.rung, 1);
    assert_eq!(run.monster().name, "Bog Toad", "next rung up");
}

#[test]
fn losing_still_pays_the_bounty_but_never_advances_you() {
    // A run with no income cannot buy its way past whatever just beat it, so
    // a loss pays out. It does not move you up: the thing is still standing.
    let mut run = Run::new(); // starter pieces, none of them placed
    run.rung = 5;
    let gold_before = run.gold;
    let bounty = run.monster().bounty;

    assert_eq!(run.fight_next().outcome, Outcome::Defeat);
    let reward = run.settle();

    assert_eq!(reward, Some(bounty));
    assert_eq!(run.gold, gold_before + bounty);
    assert_eq!(run.losses, 1);
    assert_eq!(run.wins, 0, "a loss is not a win");
}

#[test]
fn a_grinder_loss_drops_you_to_the_rung_you_last_cleared() {
    use gearmaster_engine::run::Mode;
    let mut run = Run::with_mode(Mode::Grinder);
    run.rung = 4;

    run.fight_next();
    run.settle();

    assert_eq!(run.rung, 3, "knocked back so there is something easier to farm");
    assert!(run.last_settlement.as_ref().unwrap().knocked_back);

    // And it cannot push you below the bottom of the ladder.
    run.rung = 0;
    run.back_to_loadout();
    run.fight_next();
    run.settle();
    assert_eq!(run.rung, 0);
}

#[test]
fn a_rogue_run_dies_after_three_losses() {
    use gearmaster_engine::run::{Mode, ROGUE_LIVES};
    let mut run = Run::with_mode(Mode::Rogue);
    run.rung = 4;
    run.gold = 500;

    for expected in (0..ROGUE_LIVES).rev() {
        run.back_to_loadout();
        run.fight_next();
        run.settle();
        let s = run.last_settlement.clone().unwrap();
        assert_eq!(s.lives_left, Some(expected));
        if expected > 0 {
            assert_eq!(run.rung, 4, "a rogue loss stays put");
            assert!(!s.run_ended);
        } else {
            assert!(s.run_ended, "the third loss ends it");
        }
    }

    // Everything is gone: gear, gold and ladder are back to a fresh run.
    assert_eq!(run.rung, 0);
    assert_eq!(run.gold, gearmaster_engine::shop::STARTING_GOLD);
    assert_eq!(run.lives, ROGUE_LIVES);
    assert_eq!(run.mode, Mode::Rogue, "the mode survives the wipe");
    assert_eq!(run.owned.len(), STARTER_KIT.len());
}

#[test]
fn a_reward_cannot_be_banked_twice() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.fight_next();

    assert!(run.settle().is_some());
    assert_eq!(run.settle(), None, "settling again pays nothing");
    assert_eq!(run.wins, 1, "and does not double-count the win");
}

#[test]
fn the_shop_turns_over_after_every_battle() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();

    for _ in 0..5 {
        let before = run.shop.stock.clone();
        run.fight_next();
        run.settle();
        run.back_to_loadout();
        for item in &run.shop.stock {
            assert!(!before.contains(item), "the shop re-offered something it just had");
        }
        assert_eq!(run.shop.stock.len(), SHOP_SIZE);
    }
}

#[test]
fn the_shop_restocks_after_a_loss_too() {
    let mut run = Run::new();
    let before = run.shop.stock.clone();
    run.fight_next();
    run.settle();
    assert_ne!(run.shop.stock, before);
}

#[test]
fn a_seeded_run_stocks_the_same_shop_every_time() {
    let a = Run::seeded(12345);
    let b = Run::seeded(12345);
    assert_eq!(a.shop.stock, b.shop.stock);

    let c = Run::seeded(999);
    assert_ne!(a.shop.stock, c.shop.stock, "a different seed stocks differently");
}

#[test]
fn the_whole_ladder_can_be_walked() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let mut beaten = Vec::new();
    for _ in 0..LADDER.len() {
        let name = run.monster().name;
        let outcome = run.fight_next().outcome;
        run.settle();
        run.back_to_loadout();
        if outcome == Outcome::Victory {
            beaten.push(name);
        } else {
            break;
        }
    }
    // The preset is a mid-game build, so it should clear the early rungs and
    // eventually meet something it can't handle. Either way the loop must
    // terminate and the run must stay coherent.
    assert!(!beaten.is_empty(), "the preset should beat at least the cave rat");
    assert_eq!(run.wins as usize, beaten.len());
    assert!(run.rung <= LADDER.len());
}

#[test]
fn every_monster_actually_assembles_its_gear() {
    // A typo in a monster's loadout would leave it silently harmless, which is
    // exactly the kind of bug that hides as "the game got easier".
    for m in LADDER {
        let problems = m.unassembled();
        assert!(problems.is_empty(), "{}'s loadout is broken: {:?}", m.name, problems);
    }
}

#[test]
fn every_monster_can_actually_hurt_you() {
    use gearmaster_engine::combat::{simulate, Event, Side};
    use gearmaster_engine::stats::Stats;
    for m in LADDER {
        // A punching bag with plenty of health and no offence of its own.
        let log = simulate(Stats::new(100_000, 0, 0, 100), &[], m);
        let hurt = log.entries.iter().any(|e| {
            matches!(e.event, Event::Hit { by: Side::Enemy, .. })
                || matches!(e.event, Event::MindHit { by: Side::Enemy, .. })
                || matches!(e.event, Event::Burn { side: Side::Player, .. })
        });
        assert!(hurt, "{} never lands anything", m.name);
    }
}

// ----------------------------------------------------------- difficulty

#[test]
fn the_difficulty_multiple_is_what_it_says_on_the_tin() {
    use gearmaster_engine::combat::{Combatant, Difficulty};
    // Half the factor goes into staying alive and half into hitting back, so
    // the two multiply back out to the number the player picked.
    let base = Combatant::monster_at(&LADDER[6], Difficulty::Easy);
    for &d in Difficulty::ALL {
        let scaled = Combatant::monster_at(&LADDER[6], d);
        let tough = scaled.max_health as f32 / base.max_health as f32;
        let deadly = scaled.strength as f32 / base.strength as f32;
        let product = tough * deadly;
        assert!(
            (product - d.factor()).abs() < d.factor() * 0.06,
            "{:?}: {:.2}x tougher and {:.2}x deadlier is {:.1}x, not {}x",
            d,
            tough,
            deadly,
            product,
            d.factor()
        );
    }
}

#[test]
fn a_harder_setting_is_actually_harder() {
    use gearmaster_engine::combat::Difficulty;
    let mut easy = Run::with_all_pieces();
    easy.apply_preset();
    assert_eq!(easy.fight_next().outcome, Outcome::Victory, "the preset clears rung 1 on easy");

    let mut insane = Run::with_all_pieces();
    insane.difficulty = Difficulty::Insane;
    insane.apply_preset();
    insane.rung = 6;
    assert_ne!(
        insane.fight_next().outcome,
        Outcome::Victory,
        "the same build should not walk through a mid-ladder monster at 27x"
    );
}

#[test]
fn higher_difficulties_hand_the_monster_passives() {
    use gearmaster_engine::combat::Difficulty;
    assert!(Difficulty::Easy.passives().is_empty());
    for &d in &[Difficulty::Medium, Difficulty::Hard, Difficulty::Insane] {
        assert!(!d.passives().is_empty(), "{:?} should carry passives", d);
    }
    assert!(
        Difficulty::Insane.passives().len() > Difficulty::Medium.passives().len(),
        "they should stack up with the setting"
    );
}

// --------------------------------------------------------------- prices

#[test]
fn price_climbs_with_effectiveness_and_the_best_gear_is_dear() {
    use gearmaster_engine::piece::CATALOG;
    use gearmaster_engine::rating::{piece_rating, shop_price, Rarity, RARE_AT};

    let mut priced: Vec<(i32, i32, &str)> =
        CATALOG.iter().map(|d| (piece_rating(d), shop_price(d), d.name)).collect();
    priced.sort_unstable();

    // Monotonic: nothing better is ever cheaper.
    for w in priced.windows(2) {
        assert!(w[1].1 >= w[0].1, "{} out-rates {} but costs less", w[1].2, w[0].2);
    }

    // A component strong enough to carry an item to legendary on its own has
    // to cost a fortune, or the tiers mean nothing in the shop.
    let carriers: Vec<&(i32, i32, &str)> =
        priced.iter().filter(|(r, _, _)| Rarity::of(*r) >= Rarity::Rare).collect();
    assert!(!carriers.is_empty(), "some component should reach a tier on its own");
    for (r, price, name) in carriers {
        assert!(
            *price >= 60,
            "{} rates {} on its own but costs only {}",
            name,
            r,
            price
        );
    }

    // And the floor stays reachable, or a run is dead on arrival.
    let cheapest = priced.first().unwrap().1;
    assert!(cheapest <= 5, "the cheapest component costs {}", cheapest);
    let _ = RARE_AT;
}

// ------------------------------------------------------------- the fountain

#[test]
fn the_fountain_sits_at_the_fifth_rung_and_always_gives_something() {
    let mut run = Run::with_all_pieces();
    run.rung = Run::FOUNTAIN_RUNG;
    assert!(run.at_fountain());

    // Even a bare board gets an imbuement - a fountain is never wasted.
    let class = run.drink();
    assert_eq!(class.name, "Wanderer");
    assert!(run.class.is_some());
    assert_eq!(run.rung, Run::FOUNTAIN_RUNG + 1, "and it moves you past it");
    assert!(!run.at_fountain(), "it only happens once");
}

#[test]
fn the_class_you_would_get_is_visible_before_you_drink() {
    // The whole point of the outlook: no surprises. What the panel shows and
    // what the fountain hands over have to be the same thing.
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let predicted = run
        .class_outlook()
        .into_iter()
        .find(|m| m.eligible)
        .expect("something is always eligible")
        .class
        .name;
    run.rung = Run::FOUNTAIN_RUNG;
    assert_eq!(run.drink().name, predicted);
}

#[test]
fn a_class_that_is_out_of_reach_says_how_far() {
    let run = Run::with_all_pieces(); // nothing equipped at all
    let outlook = run.class_outlook();
    let miss = outlook.iter().find(|m| !m.eligible).expect("most are out of reach");
    assert!(!miss.detail.is_empty(), "it should name what is short");
    for (_, need, have) in &miss.detail {
        assert!(have <= need || miss.detail.iter().any(|(_, n, h)| h < n));
    }
}

#[test]
fn a_standing_class_power_reaches_the_players_stats() {
    // No shipped class is a plain stat bundle any more - they all carry a
    // rule - so this tests the mechanism with a class of its own rather than
    // pinning whichever class happens to use it.
    use gearmaster_engine::class::{ClassDef, ClassPower};
    use gearmaster_engine::stats::Stats;

    static STONE: ClassDef = ClassDef {
        name: "Test Stone",
        blurb: "",
        requires: &[],
        power: ClassPower::Standing(Stats { health: 90, physical_harden: 30, ..Stats::ZERO }),
    };

    let mut run = Run::with_all_pieces();
    let before = run.player_stats();
    run.class = Some(&STONE);
    let after = run.player_stats();

    assert_eq!(after.health, before.health + 90);
    assert_eq!(after.physical_harden, before.physical_harden + 30);
}

#[test]
fn every_class_carries_a_rule_and_not_just_numbers() {
    use gearmaster_engine::class::{ClassPower, CLASSES};
    let bundles = CLASSES
        .iter()
        .filter(|c| matches!(c.power, ClassPower::Standing(_)))
        .map(|c| c.name)
        .collect::<Vec<_>>();
    assert!(
        bundles.is_empty(),
        "these classes are only a stat bundle: {:?}",
        bundles
    );
    // And no two classes share a power, or they would play the same.
    let mut seen: Vec<String> = Vec::new();
    for c in CLASSES {
        let key = format!("{:?}", c.power);
        assert!(!seen.contains(&key), "{} duplicates another class's power", c.name);
        seen.push(key);
    }
}

#[test]
fn slow_time_spreads_a_hit_instead_of_stopping_it() {
    use gearmaster_engine::class::{ClassPower, ClassDef};
    use gearmaster_engine::combat::{simulate_with_class, Difficulty, Event, Side};
    use gearmaster_engine::stats::Stats;

    static CHRONO: ClassDef = ClassDef {
        name: "Test Chronomancer",
        blurb: "",
        requires: &[],
        power: ClassPower::SlowTime,
    };

    let stats = Stats::new(400, 0, 0, 100);
    let plain = simulate_with_class(stats, &[], &LADDER[6], Difficulty::Easy, None);
    let slowed = simulate_with_class(stats, &[], &LADDER[6], Difficulty::Easy, Some(&CHRONO));

    // The swing is still logged either way - slow time changes when it lands,
    // not whether it happened.
    let swings = |log: &gearmaster_engine::combat::CombatLog| {
        log.entries
            .iter()
            .filter(|e| matches!(e.event, Event::Hit { by: Side::Enemy, .. }))
            .count()
    };
    assert!(swings(&plain) > 0 && swings(&slowed) > 0);

    // But it should take measurably longer to kill you.
    assert!(
        slowed.duration_ms >= plain.duration_ms,
        "slow time should buy time: {} vs {}",
        slowed.duration_ms,
        plain.duration_ms
    );
}

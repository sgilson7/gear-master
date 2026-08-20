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
    let price = run.registry.def(id).price;
    let before = run.gold;

    let refund = run.sell(id).unwrap();

    assert_eq!(refund, price / 2);
    assert_eq!(run.gold, before + refund);
    assert!(!run.is_equipped(id), "sold gear comes off");
    assert!(!run.owned.contains(&id));
}

// ------------------------------------------------------------- the ladder

#[test]
fn there_are_eleven_monsters_and_the_bounties_climb() {
    assert_eq!(LADDER.len(), 11, "the golem plus ten more");
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
fn losing_pays_nothing_and_leaves_you_where_you_were() {
    let mut run = Run::new(); // starter pieces, none of them placed
    run.rung = 10; // the boss
    let gold_before = run.gold;

    assert_eq!(run.fight_next().outcome, Outcome::Defeat);
    let reward = run.settle();

    assert_eq!(reward, None);
    assert_eq!(run.gold, gold_before);
    assert_eq!(run.losses, 1);
    assert_eq!(run.rung, 10, "you stay on the same rung");
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

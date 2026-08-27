//! Can you actually meet the catalogue?

use gearmaster_engine::piece::{is_boss_only, is_quest_reward, CATALOG};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::shop::Shop;
use std::collections::HashMap;

fn shelf_counts(ensure_weapon: bool, runs: u64, restocks: usize) -> HashMap<&'static str, usize> {
    let mut counts = HashMap::new();
    for r in 0..runs {
        let mut rng = Rng::new(0xC0FFEE + r);
        let mut shop = Shop::new(&mut rng);
        for _ in 0..restocks {
            for i in 0..8 {
                if let Some(d) = shop.def(i) {
                    *counts.entry(d.name).or_insert(0usize) += 1;
                }
            }
            shop.restock(&mut rng, ensure_weapon);
        }
    }
    counts
}

fn sellable() -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|d| {
            !is_boss_only(d.name)
                && !is_quest_reward(d.name)
                // Event gear is owned, not bought: what it is worth is the
                // story of how you got it.
                && !gearmaster_engine::piece::is_event_only(d.name)
                // Town gear is reachable, but not here. What a town sells is
                // covered by `town_gear_is_reachable_and_only_in_a_town`.
                && !gearmaster_engine::piece::is_town_stock(d)
                // And the mind lane's gear is reachable once the pool is. A
                // piece that banks something you have not been given yet is a
                // piece that does nothing, and the promise moves rather than
                // lapses - see the test below.
                && !gearmaster_engine::piece::touches_insight(d)
        })
        .map(|d| d.name)
        .collect()
}

/// Every component that is not a trophy or a quest reward has to be reachable.
/// A piece nobody can ever buy is a piece that may as well not have been
/// written.
#[test]
fn every_sellable_component_reaches_a_shelf() {
    let counts = shelf_counts(false, 60, 40);
    let missing: Vec<&str> = sellable().into_iter().filter(|n| !counts.contains_key(n)).collect();
    assert!(missing.is_empty(), "{} never appear: {:?}", missing.len(), &missing[..missing.len().min(10)]);
}

/// And roughly evenly. This is the test that would have caught what players
/// noticed before anyone had to say it: the shelves used to reserve two of six
/// slots for a handle and a damaging piece on every restock, so those turned
/// up seven times more often than anything else and a run felt like the same
/// few items over and over.
///
/// The ratio is a percentile spread, so it says the same thing whatever
/// `SHOP_SIZE` is - which is why a seventh shelf did not move it.
#[test]
fn the_shelves_are_not_the_same_few_things_every_time() {
    let counts = shelf_counts(false, 60, 40);
    let mut v: Vec<(&&str, &usize)> = counts.iter().collect();
    v.sort_by_key(|(_, c)| **c);
    let low = *v[v.len() / 20].1 as f32; // 5th percentile
    let high = *v[v.len() - 1 - v.len() / 20].1 as f32; // 95th
    assert!(
        high / low < 2.0,
        "the shelves favour some components {:.1}x over others: {:?} vs {:?}",
        high / low,
        &v[v.len() - 3..],
        &v[..3]
    );
}

/// Town gear is still reachable - just not from the road.
///
/// The doctrine is that every component a player can own has somewhere it can
/// be met. Taking the five town shelves and the underlays out of the ordinary
/// pool would quietly break that, so the promise moves rather than lapses:
/// what a town sells is exactly what a town sells, and the road never offers
/// it.
/// The mind lane's gear is reachable, and only after it is worth anything.
///
/// Same shape as the town rule and the same doctrine: every component a player
/// can own has somewhere it can be met. `Shop::insight_open` is that
/// somewhere, and it is shut until THE THRESHOLD is cleared.
#[test]
fn the_mind_lane_is_reachable_once_the_pool_is_open() {
    use gearmaster_engine::piece::touches_insight;
    let gated: Vec<&str> =
        CATALOG.iter().filter(|d| touches_insight(d)).map(|d| d.name).collect();
    assert!(!gated.is_empty(), "nothing in the catalogue deals in it");

    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for r in 0..80u64 {
        let mut rng = Rng::new(0x1153_1637u64.wrapping_add(r));
        let mut shop = Shop::new(&mut rng);
        shop.insight_open = true;
        shop.restock(&mut rng, false);
        for _ in 0..60 {
            for i in 0..8 {
                if let Some(d) = shop.def(i) {
                    *counts.entry(d.name).or_insert(0) += 1;
                }
            }
            shop.restock(&mut rng, false);
        }
    }
    let missing: Vec<&&str> = gated.iter().filter(|n| !counts.contains_key(*n)).collect();
    assert!(missing.is_empty(), "unreachable even once earned: {:?}", missing);
}

#[test]
fn town_gear_is_reachable_and_only_in_a_town() {
    use gearmaster_engine::piece::{is_event_only, is_town_stock, town_shelf};
    let shelf = town_shelf();
    // Re-pinned, not loosened. This read "every piece that is town stock is on
    // the cart", and the Switchyard's four enchantments are town stock - they
    // are ground, so `shop.rs` refuses them on the road for that reason - and
    // are deliberately not for sale anywhere: they are what a four-fight line
    // pays, and a shelf is a purchase (`the-switchyard.md` A3).
    //
    // The half that is the law is untouched and is asserted below: nothing on
    // the cart is ever offered by the road. What narrowed is the half that was
    // a consequence of collecting by kind.
    for d in CATALOG.iter().filter(|d| is_town_stock(d) && !is_event_only(d.name)) {
        assert!(shelf.contains(&d.name), "{} is town gear nobody stocks", d.name);
    }
    // And the cart holds nothing else: a filter that took one piece too many
    // would leave a shipped underlay unbuyable, and the loop above cannot see
    // that because it walks the catalogue rather than the cart.
    for name in shelf {
        let d = CATALOG.iter().find(|d| &d.name == name).expect("a real component");
        assert!(is_town_stock(d), "{name} is on the cart and is not town gear");
        assert!(!is_event_only(name), "{name} is dug up and is on the cart");
    }
    let counts = shelf_counts(false, 60, 40);
    let leaked: Vec<&str> =
        shelf.iter().copied().filter(|n| counts.contains_key(n)).collect();
    assert!(leaked.is_empty(), "the road offered town gear: {:?}", leaked);
}

/// The guarantee still holds where it is meant to: a player with no weapon
/// gets shelves they can build one from.
#[test]
fn an_unarmed_player_is_always_offered_a_weapon() {
    use gearmaster_engine::piece::{PieceKind, SlotKind};
    let mut rng = Rng::new(31);
    let mut shop = Shop::new(&mut rng);
    for round in 0..60 {
        let has = |k: PieceKind| {
            shop.stock_defs().iter().any(|d| d.slot == SlotKind::Weapon && d.kind == k)
        };
        let martial = has(PieceKind::Handle) && has(PieceKind::Damaging);
        let bound = has(PieceKind::Book) && has(PieceKind::Ink) && has(PieceKind::Spell);
        let ball = has(PieceKind::Orb) && has(PieceKind::Spell);
        assert!(martial || bound || ball, "round {} offers no weapon at all", round);
        shop.restock(&mut rng, true);
    }
}


/// Two people who start a run see two different shops.
///
/// The shop is the one surface where a player meets the catalogue, and a game
/// that dealt every run the same six things would be a game with one opening.
/// `Run::seeded` is the only way in and the shop's rolls come off the run's own
/// xorshift, so this is a property of the seeding rather than of the shelves -
/// which is exactly why it is worth an assertion: nothing else in the suite
/// would notice if a refactor started every shop from a constant.
#[test]
fn two_seeds_are_two_shops() {
    let stock = |seed: u64| -> Vec<&'static str> {
        gearmaster_engine::run::Run::seeded(seed)
            .shop
            .stock_defs()
            .iter()
            .map(|d| d.name)
            .collect()
    };

    // Different seeds, different openings. Sampled over sixteen rather than
    // two, because two could collide by luck and say nothing.
    let seeds: Vec<Vec<&str>> = (0..16).map(|i| stock(0x51D0_0000 + i * 0x9E37)).collect();
    let mut distinct = seeds.clone();
    distinct.sort();
    distinct.dedup();
    assert!(
        distinct.len() >= 15,
        "sixteen seeds produced {} distinct shops: {seeds:?}",
        distinct.len()
    );

    // And no two of them are even close: a shop that shares five of six with
    // its neighbour is a shop nobody would call different.
    let shared = |a: &Vec<&str>, b: &Vec<&str>| a.iter().filter(|n| b.contains(n)).count();
    let worst = (0..seeds.len())
        .flat_map(|i| ((i + 1)..seeds.len()).map(move |j| (i, j)))
        .map(|(i, j)| shared(&seeds[i], &seeds[j]))
        .max()
        .expect("pairs");
    assert!(
        worst < seeds[0].len(),
        "two seeds dealt the same shop: {worst} of {} shared",
        seeds[0].len()
    );

    // The same seed twice is the same shop, which is the other half of the
    // contract and the half share codes and replays depend on.
    assert_eq!(stock(0x51D0), stock(0x51D0), "one seed, two shops");
}

/// Which slot the shelves actually deal, over 400 opening shops.
///
/// The number to watch is the weapon's: it is two fifths of the catalogue and
/// without a tilt it takes 54.8% of every shelf, which is a game that is a
/// weapon and some accessories. Measured at `SHOP_SIZE` 6 and 7 on the day the
/// seventh shelf landed: **53.2% at six, 48.7% at seven**. The extra draw is
/// one more pass of the round-robin, and the armour slots are what pick it up.
#[test]
#[ignore]
fn report_shelf_mix() {
    use gearmaster_engine::piece::SlotKind;
    let mut per_slot = std::collections::BTreeMap::new();
    let mut n = 0;
    for seed in 0..400u64 {
        let run = gearmaster_engine::run::Run::seeded(0xA11CE + seed * 0x9E37);
        for d in run.shop.stock_defs() {
            *per_slot.entry(format!("{:?}", d.slot)).or_insert(0usize) += 1;
            n += 1;
        }
    }
    println!("\n## 400 opening shelves, {n} cards\n");
    for (k, v) in &per_slot {
        println!("  {k:<10}{v:>6}  {:.1}%", *v as f32 / n as f32 * 100.0);
    }
    let _ = SlotKind::ALL;
}

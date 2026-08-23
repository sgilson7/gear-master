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
/// six items over and over.
#[test]
fn the_shelves_are_not_the_same_six_things_every_time() {
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

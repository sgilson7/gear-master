//! The three primitives the interaction fabric is built from: watchers that
//! count, the diagonal relation, and fused pools.
//!
//! These tests build their item profiles by hand rather than by packing a
//! board: what is under test is the mechanic, not the gear. That was because
//! nothing in the catalogue carried any of these when they were written, and it
//! stays that way now that pieces do, because a test that reaches for whichever
//! watcher happens to exist is a test that changes meaning under a sweep.
//!
//! The catalogue has since caught up unevenly - fifteen pieces watch, seven see
//! diagonally, one is terrain, and **nothing fuses a pool**. Fusion is the one
//! primitive that still exists only here.

mod common;

use gearmaster_engine::combat::{simulate, CombatLog, Event, MonsterSpec, Side};
use gearmaster_engine::loadout::{ItemProfile, Loadout};
use gearmaster_engine::piece::{
    Action, PieceRegistry, Resource, SlotKind, Trigger, Watched, CATALOG,
};
use gearmaster_engine::slot::SLOT_W;
use gearmaster_engine::stats::Stats;

/// Something to hit that will not die and will not hit back, so a fight runs
/// the full clock and every tick is the gear's.
const DUMMY: MonsterSpec = MonsterSpec {
    name: "Dummy",
    health: 1_000_000,
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
    sprite: gearmaster_engine::combat::MonsterSprite::Rat,
    rank: gearmaster_engine::combat::Rank::Ordinary,
    drops: &[],
    items: &[],
};

fn item(name: &str, slot: SlotKind, cooldown_ms: u32) -> ItemProfile {
    ItemProfile {
        sigil_seed: 0,
        pieces: Vec::new(),
        name: name.to_string(),
        full_name: name.to_string(),
        core: name.to_string(),
        slot,
        cooldown_ms,
        stats: Stats::ZERO,
        triggers: Vec::new(),
        adjacent_assembled_same_slot: 0,
        diagonal_items: Vec::new(),
        open_cells: 0,
        attracts_curses: false,
        power: 100,
        rating: 0,
        power_bonus: 0,
        casts: Vec::new(),
        adjacent_items: Vec::new(),
        aligned_items: Vec::new(),
    }
}

fn me() -> Stats {
    Stats::new(10_000, 0, 0, 100)
}

/// Every time a watcher on the player's side paid out.
fn payouts(log: &CombatLog, name: &str) -> Vec<u32> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Watched { side: Side::Player, item, .. } if item == name => Some(e.at_ms),
            _ => None,
        })
        .collect()
}

fn mana_totals(log: &CombatLog) -> Vec<i32> {
    log.entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::GainMana { side: Side::Player, total, .. } => Some(*total),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------------- watch

#[test]
fn a_watcher_pays_out_every_nth_thing_it_sees() {
    // One item ticking every second, and a watcher waiting for four of them.
    // Over ten seconds that is two payouts, at four and at eight.
    let driver = item("Driver", SlotKind::Weapon, 1000);
    let mut watcher = item("Watcher", SlotKind::Helmet, 600_000);
    watcher.triggers = vec![Trigger::Watch {
        what: Watched::AnyActivation,
        count: 4,
        then: Action::GainMana(1),
        repeats: true,
    }];

    let log = simulate(me(), &[driver, watcher], &DUMMY);
    let paid = payouts(&log, "Watcher");
    assert!(paid.len() >= 2, "a watcher counting to four never came round: {:?}", paid);
    // The gaps are what matters, not the absolute times: four activations
    // apart at one a second is four seconds apart.
    assert_eq!(paid[1] - paid[0], 4000, "payouts were {:?}", paid);
}

#[test]
fn a_watcher_that_does_not_repeat_pays_once_and_stops() {
    let driver = item("Driver", SlotKind::Weapon, 1000);
    let mut watcher = item("Watcher", SlotKind::Helmet, 600_000);
    watcher.triggers = vec![Trigger::Watch {
        what: Watched::AnyActivation,
        count: 3,
        then: Action::GainMana(1),
        repeats: false,
    }];

    let log = simulate(me(), &[driver, watcher], &DUMMY);
    assert_eq!(payouts(&log, "Watcher").len(), 1, "a once-only watcher paid more than once");
}

#[test]
fn a_watcher_does_not_count_its_own_item() {
    // The only thing acting is the watcher itself. If it counted its own
    // activations it would pay out repeatedly; it should never pay at all.
    let mut watcher = item("Watcher", SlotKind::Helmet, 1000);
    watcher.triggers = vec![
        Trigger::OnActivate(Action::GainMana(1)),
        Trigger::Watch {
            what: Watched::AnyActivation,
            count: 2,
            then: Action::GainMana(50),
            repeats: true,
        },
    ];

    let log = simulate(me(), &[watcher], &DUMMY);
    assert!(payouts(&log, "Watcher").is_empty(), "a watcher counted itself coming round");
}

#[test]
fn each_kind_of_watcher_counts_only_its_own_relation() {
    // One driver, and four watchers set to the four activation relations. The
    // driver is recorded as touching only the first of them, so only that one
    // and the `AnyActivation` watcher should ever pay.
    let driver = item("Driver", SlotKind::Weapon, 1000);
    let mut items = vec![driver];
    for (i, what) in [
        Watched::AnyActivation,
        Watched::AdjacentActivation,
        Watched::AlignedActivation,
        Watched::DiagonalActivation,
    ]
    .into_iter()
    .enumerate()
    {
        let mut w = item(&format!("W{i}"), SlotKind::Helmet, 600_000);
        w.triggers = vec![Trigger::Watch {
            what,
            count: 2,
            then: Action::GainMana(1),
            repeats: true,
        }];
        // Only the adjacency watcher is told it touches the driver.
        if what == Watched::AdjacentActivation {
            w.adjacent_items = vec![0];
        }
        items.push(w);
    }

    let log = simulate(me(), &items, &DUMMY);
    assert!(!payouts(&log, "W0").is_empty(), "the any-activation watcher saw nothing");
    assert!(!payouts(&log, "W1").is_empty(), "the adjacent watcher saw nothing");
    assert!(payouts(&log, "W2").is_empty(), "an aligned watcher counted an unaligned item");
    assert!(payouts(&log, "W3").is_empty(), "a diagonal watcher counted a non-diagonal item");
}

#[test]
fn a_watched_board_replays_identically() {
    // The whole engine rests on combat being a pure function of the two
    // boards, and a counter is state - which is exactly the sort of thing that
    // leaks between runs if it is stored in the wrong place.
    let build = || {
        let driver = item("Driver", SlotKind::Weapon, 700);
        let mut w = item("Watcher", SlotKind::Gloves, 600_000);
        w.triggers = vec![Trigger::Watch {
            what: Watched::AnyActivation,
            count: 3,
            then: Action::GainMana(2),
            repeats: true,
        }];
        vec![driver, w]
    };
    let a = simulate(me(), &build(), &DUMMY);
    let b = simulate(me(), &build(), &DUMMY);
    assert_eq!(a.duration_ms, b.duration_ms);
    assert_eq!(payouts(&a, "Watcher"), payouts(&b, "Watcher"));
    assert_eq!(mana_totals(&a), mana_totals(&b));
}

#[test]
fn a_curse_watcher_counts_curses_and_not_activations() {
    let mut curser = item("Curser", SlotKind::Weapon, 1000);
    curser.triggers = vec![Trigger::OnActivate(Action::Curse {
        kind: gearmaster_engine::curse::CurseKind::Searing,
        target: gearmaster_engine::piece::Target::Enemy,
    })];
    let mut watcher = item("Watcher", SlotKind::Helmet, 600_000);
    watcher.triggers = vec![Trigger::Watch {
        what: Watched::CurseApplied,
        count: 2,
        then: Action::GainMana(1),
        repeats: true,
    }];

    let cursed = simulate(me(), &[curser, watcher.clone()], &DUMMY);
    assert!(!payouts(&cursed, "Watcher").is_empty(), "a curse watcher saw no curses");

    // The same watcher beside an item that only ever swings sees nothing.
    let plain = item("Plain", SlotKind::Weapon, 1000);
    let quiet = simulate(me(), &[plain, watcher], &DUMMY);
    assert!(payouts(&quiet, "Watcher").is_empty(), "a curse watcher counted an activation");
}


// ---------------------------------------------------------------- diagonal

/// The smallest chest components there are, so a test board can hold two
/// finished items with room between them. Looked up rather than named, because
/// a name is a key and this test has no business pinning one.
fn smallest(kind: gearmaster_engine::piece::PieceKind) -> usize {
    CATALOG
        .iter()
        .enumerate()
        .filter(|(_, d)| d.slot == SlotKind::Chest && d.kind == kind)
        .min_by_key(|(_, d)| d.cells.len())
        .map(|(i, _)| i)
        .unwrap_or_else(|| panic!("no chest {:?} in the catalogue", kind))
}

/// Seat two single pieces in one grid and ask the slot directly what relation
/// their footprints have. Pure geometry - no recipes, no assembly.
fn geometry_at(a: (u8, u8), b: (u8, u8)) -> Option<(bool, bool)> {
    use gearmaster_engine::piece::PieceKind;
    let layer = smallest(PieceKind::Layer);
    let mut reg = PieceRegistry::new();
    let (one, two) = (reg.alloc(layer), reg.alloc(layer));
    let mut lo = Loadout::new();
    let slot = lo.slot_mut(SlotKind::Chest);
    if slot.can_place(&reg, one, a.0, a.1).is_err() {
        return None;
    }
    slot.place(&reg, one, a.0, a.1);
    if slot.can_place(&reg, two, b.0, b.1).is_err() {
        return None;
    }
    slot.place(&reg, two, b.0, b.1);
    Some((slot.sets_touch(&[one], &[two]), slot.sets_touch_diagonally(&[one], &[two])))
}

#[test]
fn an_edge_is_adjacent_and_a_corner_is_diagonal() {
    let layer = &CATALOG[smallest(gearmaster_engine::piece::PieceKind::Layer)];
    assert_eq!(layer.cells.len(), 1, "this test wants a one-cell layer to reason about");

    assert_eq!(geometry_at((1, 1), (2, 1)), Some((true, false)), "side by side");
    assert_eq!(geometry_at((1, 1), (1, 2)), Some((true, false)), "one above the other");
    assert_eq!(geometry_at((1, 1), (2, 2)), Some((false, true)), "corner to corner");
    assert_eq!(geometry_at((1, 1), (0, 0)), Some((false, true)), "the other corner");
    assert_eq!(geometry_at((1, 1), (3, 3)), Some((false, false)), "two cells apart");
}

#[test]
fn the_two_relations_are_never_both_true() {
    // Diagonal is the leftover relation: it names the pair that reaches *past*
    // its neighbours. A pair sharing an edge somewhere is adjacent whatever
    // its corners do, or a packed board would answer every diagonal trigger
    // as well as every adjacent one.
    let mut saw_adjacent = false;
    let mut saw_diagonal = false;
    for ax in 0..SLOT_W {
        for ay in 0..4u8 {
            for bx in 0..SLOT_W {
                for by in 0..4u8 {
                    let Some((adj, diag)) = geometry_at((ax, ay), (bx, by)) else { continue };
                    assert!(
                        !(adj && diag),
                        "({ax},{ay}) and ({bx},{by}) came back adjacent *and* diagonal"
                    );
                    saw_adjacent |= adj;
                    saw_diagonal |= diag;
                }
            }
        }
    }
    assert!(saw_adjacent && saw_diagonal, "the sweep never produced both relations");
}

#[test]
fn a_corner_off_the_board_is_simply_not_there() {
    // A piece in the corner of the grid has one diagonal, not four, and asking
    // for the others does not panic or wrap around to the far side.
    assert_eq!(geometry_at((0, 0), (1, 1)), Some((false, true)), "the corner it does have");
    let far = SLOT_W - 1;
    assert_eq!(geometry_at((0, 0), (far, 0)), Some((false, false)), "across the grid");
    assert_eq!(geometry_at((0, 0), (far, 1)), Some((false, false)), "no wrapping round");
}

#[test]
fn two_finished_items_can_be_diagonal_to_one_another() {
    // The relation is no use unless `combat_items` actually reports it, which
    // is a different question from whether the geometry is right. Rather than
    // pin an arrangement - the shapes of the smallest chest pieces are not
    // this test's business - sweep for one and require that it exists.
    use gearmaster_engine::piece::PieceKind;
    let (base, layer) = (smallest(PieceKind::Base), smallest(PieceKind::Layer));

    let mut found = false;
    for bx in 0..SLOT_W {
        for by in 0..6u8 {
            let mut reg = PieceRegistry::new();
            let mut lo = Loadout::new();
            let mut ok = true;
            // Two items, each a base with its layer tucked against it.
            for (x, y) in [(0u8, 0u8), (bx, by)] {
                let (core, skin) = (reg.alloc(base), reg.alloc(layer));
                for (id, at) in [(core, (x, y)), (skin, (x, y))] {
                    let seated = (0..8u8).any(|dy| {
                        let yy = at.1.saturating_add(dy);
                        if lo.can_place(&reg, id, SlotKind::Chest, at.0, yy).is_ok() {
                            lo.slot_mut(SlotKind::Chest).place(&reg, id, at.0, yy);
                            true
                        } else {
                            false
                        }
                    });
                    ok &= seated;
                }
            }
            if !ok {
                continue;
            }
            gearmaster_engine::loadout::lock_assembled_in(&mut lo, &reg, SlotKind::Chest);
            let items = lo.combat_items(&reg);
            if items.len() != 2 {
                continue;
            }
            for it in &items {
                assert!(
                    it.diagonal_items.iter().all(|j| !it.adjacent_items.contains(j)),
                    "an item was reported both adjacent and diagonal to the same neighbour"
                );
                found |= !it.diagonal_items.is_empty();
            }
        }
    }
    assert!(found, "no arrangement of two chest items ever came back diagonal");
}

// ------------------------------------------------------------------ fusion

/// What a pool stood at when the fight ended.
///
/// `CombatLog.player` is the combatant the fight *started* with - pools all
/// zero - so the end state has to be read off the events, which carry a
/// running total for exactly this reason.
fn final_pool(log: &CombatLog, what: Resource) -> i32 {
    log.entries
        .iter()
        .rev()
        .find_map(|e| match &e.event {
            Event::GainResource { side: Side::Player, what: w, total, .. }
            | Event::Fused { side: Side::Player, what: w, total, .. } if *w == what.name() => {
                Some(*total)
            }
            // A fusion is also the last word on both of its parents.
            Event::Fused { side: Side::Player, from, and, .. } if from.0 == what.name() => {
                Some(from.1)
            }
            Event::Fused { side: Side::Player, and, .. } if and.0 == what.name() => Some(and.1),
            Event::Drained { on: Side::Player, what: w, total, .. } if *w == what.name() => {
                Some(*total)
            }
            _ => None,
        })
        .unwrap_or(0)
}

/// A piece that banks `n` of a pool once, before the first tick.
fn banker(name: &str, what: Resource, n: i32) -> ItemProfile {
    let mut it = item(name, SlotKind::Helmet, 600_000);
    it.triggers = vec![Trigger::OnBattleStart(Action::Gain { what, amount: n })];
    it
}

fn fuser(name: &str, a: Resource, b: Resource, into: Resource, cooldown_ms: u32) -> ItemProfile {
    let mut it = item(name, SlotKind::Helmet, cooldown_ms);
    it.triggers = vec![Trigger::OnActivate(Action::Fuse { a, b, into })];
    it
}

#[test]
fn fusing_spends_one_of_each_parent_for_one_of_the_child() {
    let log = simulate(
        me(),
        &[
            banker("Nature", Resource::Nature, 3),
            banker("Rage", Resource::Rage, 3),
            fuser("Fuser", Resource::Nature, Resource::Rage, Resource::DruidicMight, 1000),
        ],
        &DUMMY,
    );
    // Three of each parent buys three conversions and then nothing.
    assert_eq!(final_pool(&log, Resource::DruidicMight), 3);
    assert_eq!(final_pool(&log, Resource::Nature), 0);
    assert_eq!(final_pool(&log, Resource::Rage), 0);
}

#[test]
fn fusing_does_nothing_at_all_when_a_parent_is_missing() {
    // Rage but no nature. The trade is refused rather than half-made, so the
    // rage is still there at the end of the fight.
    let log = simulate(
        me(),
        &[
            banker("Rage", Resource::Rage, 5),
            fuser("Fuser", Resource::Nature, Resource::Rage, Resource::DruidicMight, 500),
        ],
        &DUMMY,
    );
    assert_eq!(final_pool(&log, Resource::DruidicMight), 0, "fused out of nothing");
    assert_eq!(final_pool(&log, Resource::Rage), 5, "the surviving parent was spent anyway");
}

#[test]
fn a_fusion_may_not_be_used_as_a_parent() {
    let log = simulate(
        me(),
        &[
            banker("Nature", Resource::Nature, 4),
            banker("Rage", Resource::Rage, 4),
            fuser("Make", Resource::Nature, Resource::Rage, Resource::DruidicMight, 500),
            fuser("Abuse", Resource::DruidicMight, Resource::Nature, Resource::Zealotry, 500),
        ],
        &DUMMY,
    );
    assert!(final_pool(&log, Resource::DruidicMight) > 0, "nothing was fused at all");
    assert_eq!(final_pool(&log, Resource::Zealotry), 0, "a fusion was used as fuel");
}

#[test]
fn a_fusion_is_not_spendable() {
    assert!(Resource::DruidicMight.is_fused());
    assert!(Resource::Communion.is_fused());
    assert!(Resource::Zealotry.is_fused());
    assert!(!Resource::SPENDABLE.iter().any(|r| r.is_fused()));
}

#[test]
fn every_fusion_names_the_two_parents_it_is_made_of() {
    for r in Resource::ALL.iter().copied().filter(|r| r.is_fused()) {
        let (a, b) = r.parents().unwrap_or_else(|| panic!("{:?} has no parents", r));
        assert!(!a.is_fused() && !b.is_fused(), "{:?} is made of another fusion", r);
        assert_ne!(a, b, "{:?} is made of one pool twice", r);
    }
    for r in Resource::SPENDABLE {
        assert!(r.parents().is_none(), "{:?} is spendable and should have no parents", r);
    }
}

#[test]
fn a_fused_pool_can_still_be_drained() {
    // The counterplay. A fusion cannot be spent by the one holding it, but it
    // can be taken - which is what stops banking one from being free.
    let built = [
        banker("Nature", Resource::Nature, 2),
        banker("Rage", Resource::Rage, 2),
        fuser("Fuser", Resource::Nature, Resource::Rage, Resource::DruidicMight, 500),
    ];
    let kept = simulate(me(), &built, &DUMMY);
    assert_eq!(final_pool(&kept, Resource::DruidicMight), 2);

    let mut thief = item("Thief", SlotKind::Gloves, 4000);
    thief.triggers = vec![Trigger::OnActivate(Action::Drain {
        what: Resource::DruidicMight,
        amount: 0,
        hurt: 0,
        target: gearmaster_engine::piece::Target::Yourself,
    })];
    let mut robbed = built.to_vec();
    robbed.push(thief);
    let taken = simulate(me(), &robbed, &DUMMY);
    assert_eq!(final_pool(&taken, Resource::DruidicMight), 0, "a fused pool resisted a drain");
}

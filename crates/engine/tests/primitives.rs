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
        steady: false,
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
            // `paid` only: a `Watched` is logged on every sighting now, and a
            // sighting is not a payout.
            Event::Watched { side: Side::Player, item, paid: true, .. } if item == name => {
                Some(e.at_ms)
            }
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

/// The log records where a watcher's counter stands, on every sighting.
///
/// It used not to, and that was a bug the interface wore for months: the
/// segments under a cooldown bar never filled and the number beside it never
/// moved, while the payout animation and the log lines worked perfectly. Those
/// two read *events*; the counter was read off `CombatLog::player`, which is
/// the fighter as it was **before** the first tick - so it was zeros, and it
/// stayed zeros for the length of the replay.
///
/// The fix is that a sighting is a logged fact rather than private state, and
/// this is the test that keeps it one. What it asserts is the shape the
/// interface actually needs: the counter climbs 1, 2, 3, 4 and only the fourth
/// says it paid.
#[test]
fn the_log_says_where_a_watcher_has_counted_to() {
    let driver = item("Driver", SlotKind::Weapon, 1000);
    let mut watcher = item("Watcher", SlotKind::Helmet, 600_000);
    watcher.triggers = vec![Trigger::Watch {
        what: Watched::AnyActivation,
        count: 4,
        then: Action::GainMana(1),
        repeats: true,
    }];
    let log = simulate(me(), &[driver, watcher], &DUMMY);

    let counts: Vec<(u32, bool)> = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::Watched { item, seen, count, paid, .. } if item == "Watcher" => {
                assert_eq!(*count, 4, "the readout forgot what it counts to");
                Some((*seen, *paid))
            }
            _ => None,
        })
        .collect();

    assert!(counts.len() > 4, "only {} sighting(s) logged - the ones between are missing", counts.len());
    // It climbs, one at a time, and never repeats a number.
    for (i, (seen, _)) in counts.iter().enumerate() {
        assert_eq!(*seen, i as u32 + 1, "the counter jumped: {:?}", counts);
    }
    // And exactly every fourth one is a payout.
    for (seen, paid) in &counts {
        assert_eq!(*paid, seen % 4 == 0, "sighting {} disagrees about paying", seen);
    }
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
        // Only the adjacent watcher is told it touches the driver.
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

// ------------------------------------------------------- quest items
//
// A word somebody told you, a trophy, a chit. They were `Frame`s with one cell
// and `Stats::ZERO`, and the rumour module's own doc offered "seating it would
// cost you a cell and gain you nothing" as the reason nobody would - which is
// a rule enforced by not being worth breaking, which is not a rule. The shop
// drew them as helmet frames because that is what they said they were.

fn quest_items() -> Vec<&'static gearmaster_engine::piece::PieceDef> {
    gearmaster_engine::piece::CATALOG
        .iter()
        .filter(|d| d.kind == gearmaster_engine::piece::PieceKind::Quest)
        .collect()
}

#[test]
fn every_rumour_and_the_trophy_trade_is_a_quest_item() {
    for r in gearmaster_engine::rumour::RUMOURS {
        let def = gearmaster_engine::piece::CATALOG
            .iter()
            .find(|d| d.name == r.name)
            .unwrap_or_else(|| panic!("{} is a rumour with nothing to hold", r.name));
        assert_eq!(
            def.kind,
            gearmaster_engine::piece::PieceKind::Quest,
            "{} is still typed as gear",
            r.name
        );
    }
    let trophy = gearmaster_engine::piece::CATALOG
        .iter()
        .find(|d| d.name == gearmaster_engine::rumour::TROPHY_SHELF)
        .expect("the trophy trade is a component");
    assert_eq!(trophy.kind, gearmaster_engine::piece::PieceKind::Quest);
    // Ten words and the trophy trade. Was nine: the Switchyard's chain is
    // seeded by two, and both are `Carried` conditions bought from a door
    // rather than sold at the bar, because `SHELVES` is exactly six names and
    // all six are spoken for - the pub is full (Part E, E-2). The bar's six is
    // `SHELVES`, not `SHOP_SIZE`: a pub stocks itself with `stock_exactly` and
    // never saw the seventh shelf the road shop grew.
    // And three tokens, retyped from `Accessory`: The Stranger's Parcel, An
    // Unwound Mainspring and the Platinum Chip. None of them is gear - two are
    // keys a door reads out of your hands and the third is cargo - and two of
    // the three carried stats that contradicted their own doc comments. A chip
    // whose note says it "costs you a cell to keep" was paying two magic
    // damage and two mana for the privilege.
    assert_eq!(quest_items().len(), 14, "ten words, the trophy trade, and three tokens");
}

#[test]
fn a_quest_item_carries_nothing_and_does_nothing() {
    for d in quest_items() {
        assert_eq!(d.base, gearmaster_engine::stats::Stats::ZERO, "{} has stats", d.name);
        assert!(d.triggers.is_empty(), "{} has a trigger", d.name);
        assert!(d.effect.is_none(), "{} has an effect", d.name);
        assert!(d.assembly_bonus.is_none(), "{} has an assembly_bonus", d.name);
        assert_eq!(d.cells.len(), 1, "{} is bigger than a token", d.name);
    }
}

#[test]
fn no_recipe_can_build_anything_out_of_a_quest_item() {
    // The same guarantee `Enchantment` has, and by the same means: no recipe
    // names the kind, so there is no rule to write and none to forget.
    for slot in gearmaster_engine::piece::SlotKind::ALL {
        for recipe in gearmaster_engine::piece::recipes(slot) {
            for (kind, _, _) in recipe.iter() {
                assert_ne!(
                    *kind,
                    gearmaster_engine::piece::PieceKind::Quest,
                    "a {:?} recipe asks for a quest item",
                    slot
                );
            }
        }
    }
}

#[test]
fn a_quest_item_is_carried_and_never_worn() {
    use gearmaster_engine::piece::SlotKind;
    let mut run = gearmaster_engine::run::Run::seeded(0x9E57_0001);
    let word = quest_items()[0].name;
    let id = run.give(word).expect("the road can hand one over");

    // Every slot, every anchor the grid has. None of them takes it.
    for slot in SlotKind::ALL {
        for y in 0..4u8 {
            for x in 0..4u8 {
                assert!(
                    run.can_equip(id, slot, x, y).is_err(),
                    "{} was seated in the {:?} at {},{}",
                    word,
                    slot,
                    x,
                    y
                );
            }
        }
    }
    assert!(run.inventory().contains(&id), "and it is still in the tray");
}

// ------------------------------------------------- assembly bonuses
//
// It was called an `Adjacency` after the Backpack Battles bonus it was
// modelled on, where the bonus really is adjacency-based. Here it is gated on
// `assembled[gi]` and nothing else - there is no neighbour test on that path -
// and the game uses five other names for genuine adjacency beside it.

fn assembly_bonuses() -> Vec<(&'static str, gearmaster_engine::piece::AssemblyBonus)> {
    gearmaster_engine::piece::CATALOG
        .iter()
        .filter_map(|d| d.assembly_bonus.map(|b| (d.name, b)))
        .collect()
}

/// A label is a name. The figures come from the stat block.
///
/// Twenty-nine of the thirty-seven used to state their own numbers in prose -
/// "Stonewall: +25% physical resistance" - and the other eight stated nothing
/// at all, so a Deeprooted Sole card read "when assembled: planted" and its
/// +10 curse resist appeared on no screen in the game. Both halves are the
/// same fault: the number was wherever the author put it rather than where the
/// renderer could find it.
///
/// This is the ratchet on that. A label may not carry a figure, because a
/// figure in a label is a figure that can disagree with the bonus.
#[test]
fn no_assembly_bonus_states_its_own_numbers() {
    for (piece, b) in assembly_bonuses() {
        assert!(
            !b.label.chars().any(|c| c.is_ascii_digit()) && !b.label.contains('%'),
            "{piece}: the label {:?} states a figure. The stat block is {:?} and \
             that is what the screen prints - a number in the label is a second \
             copy nobody keeps in step.",
            b.label,
            b.stats.summary()
        );
    }
}

/// And every one of them is worth something, so the line it prints is never
/// empty.
#[test]
fn every_assembly_bonus_carries_a_stat() {
    for (piece, b) in assembly_bonuses() {
        assert!(
            !b.stats.summary().is_empty(),
            "{piece}: {:?} is a label over an empty stat block, so the card \
             would print a heading and nothing under it",
            b.label
        );
    }
}

/// The count, so the next person who reads "exactly one per slot" is not
/// misled the way this milestone was.
#[test]
fn the_catalogue_carries_the_assembly_bonuses_it_says_it_does() {
    let n = assembly_bonuses().len();
    assert_eq!(
        n, 36,
        "thirty-six. It was thirty-seven until this test found that Leaden \
         Tome's was a label over Stats::ZERO - a heading for its power_bonus, \
         which is unconditional and printed elsewhere"
    );
}

/// Which assembly bonuses do something beyond their stat block.
///
/// This was `no_assembly_bonus_is_armed_yet` and it asserted the list was
/// empty, because M2 landed the `triggers` field inert and could not prove the
/// wiring - `CATALOG` is static, so no test could invent a piece carrying one.
/// Its own message said the commit that armed the first one would own the
/// re-measurement and that the assertion would become the list. It is the
/// list.
///
/// The proof it could not give lives in `tests/assembly_bonuses.rs`, which
/// fights a board wearing each of these and fails if the trigger never reaches
/// the log.
#[test]
fn the_armed_assembly_bonuses_are_the_ones_that_were_authored() {
    let mut armed: Vec<&str> = gearmaster_engine::piece::CATALOG
        .iter()
        .filter(|d| d.assembly_bonus.is_some_and(|b| !b.triggers.is_empty()))
        .map(|d| d.name)
        .collect();
    armed.sort_unstable();
    assert_eq!(
        armed,
        vec![
            "Ambush Mold",
            "Coldstep Mold",
            "Deadfall Mold",
            "Deeprooted Sole",
            "Pilgrim Sole",
            "Ridge Runner",
            "Rimebound Mold",
            "Worldstrider Sole",
        ],
        "an assembly bonus was armed or disarmed. That moves the ladder, so the \
         commit that did it owns the re-measurement and this line."
    );
}

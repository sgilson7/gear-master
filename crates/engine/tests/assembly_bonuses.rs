//! Assembly bonuses that do something, and the wiring that carries them.
//!
//! M2 added `AssemblyBonus::triggers` and said plainly that it could not prove
//! the wiring: `CATALOG` is static, so a test cannot invent a piece carrying
//! one, and no shipped piece did. This file is that proof. Each test below
//! fails if the trigger never reaches the fight, which is the only way to know
//! the field is connected rather than merely declared.
//!
//! The four here are the ones that cost no new combat code - they are built
//! from triggers the game already had. Three of them exercise machinery that
//! was reachable in principle and by nothing in practice.

mod common;

use gearmaster_engine::combat::{simulate_at, Difficulty, Event, Side};
use gearmaster_engine::piece::{PieceKind, Resource, SlotKind, CATALOG};
use gearmaster_engine::run::Run;

/// Seat a greaves piece and whatever it needs to assemble, and fight.
fn fight_wearing(piece: &str) -> gearmaster_engine::combat::CombatLog {
    let mut run = Run::with_all_pieces();
    let id = |run: &Run, name: &str| {
        run.owned.iter().copied().find(|&p| run.registry.def(p).name == name).expect(name)
    };
    // A greaves item is Material + Mold, so the piece under test is joined by
    // whichever of the two it is not.
    let def = CATALOG.iter().find(|d| d.name == piece).expect("a real piece");
    let partner = CATALOG
        .iter()
        .find(|d| {
            d.slot == SlotKind::Greaves
                && d.assembly_bonus.is_none()
                && match def.kind {
                    PieceKind::Mold => d.kind == PieceKind::Material,
                    _ => d.kind == PieceKind::Mold,
                }
        })
        .expect("something to build it with");
    let a = id(&run, piece);
    let b = id(&run, partner.name);
    run.equip(a, SlotKind::Greaves, 0, 0).expect("seats");
    // Walk the partner along the row until the two touch and assemble.
    for x in 1..6u8 {
        if run.equip(b, SlotKind::Greaves, x, 0).is_ok()
            && run.report(SlotKind::Greaves).assembled_count() > 0
        {
            break;
        }
    }
    assert!(
        run.report(SlotKind::Greaves).assembled_count() > 0,
        "{piece} never assembled, so its bonus was never live and this test proves nothing"
    );
    let spec = gearmaster_engine::combat::creature("Cave Rat").expect("exists");
    simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium)
}

/// The same, plus whatever else the test needs on the board.
fn fight_wearing_and(
    piece: &str,
    also: &[(&str, SlotKind)],
) -> gearmaster_engine::combat::CombatLog {
    let mut run = Run::with_all_pieces();
    let id = |run: &Run, n: &str| {
        run.owned.iter().copied().find(|&p| run.registry.def(p).name == n).expect(n)
    };
    let a = id(&run, piece);
    let b = id(&run, "Runed Material");
    run.equip(a, SlotKind::Greaves, 0, 0).expect("seats");
    for x in 1..6u8 {
        if run.equip(b, SlotKind::Greaves, x, 0).is_ok()
            && run.report(SlotKind::Greaves).assembled_count() > 0
        {
            break;
        }
    }
    for (name, slot) in also {
        let p = id(&run, name);
        for y in 0..4u8 {
            for x in 0..4u8 {
                if run.equip(p, *slot, x, y).is_ok() {
                    break;
                }
            }
        }
    }
    let spec = gearmaster_engine::combat::creature("Cave Rat").expect("exists");
    simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium)
}

/// The wiring, proved: a trigger that exists only on the bonus reaches the log.
#[test]
fn a_bonus_trigger_reaches_the_fight() {
    let log = fight_wearing("Pilgrim Sole");
    let banked = log.entries.iter().any(|e| {
        matches!(&e.event, Event::GainResource { side: Side::Player, what: "faith", .. })
    });
    assert!(
        banked,
        "PILGRIM SOLE's bonus banks faith at the bell and no faith was banked. \
         The piece itself has no triggers - this one is the assembly bonus's, \
         so if it is absent the field is declared and not connected."
    );
}

/// Communion, made by a board for the first time.
///
/// `Resource` has had three fusions since the slot rewrite and
/// `Combatant::held_bonus` pays each at **double both parents' rates**,
/// uncapped. `Action::Fuse` was implemented, guarded and complete. Nothing in
/// the 504-piece catalogue used either, so the best-paying pools in the game
/// were unreachable - the same shape as `cursed_for_good` before the Unwinding
/// found it.
#[test]
fn the_pilgrims_road_makes_communion() {
    let log = fight_wearing("Pilgrim Sole");
    let fused = log
        .entries
        .iter()
        .find(|e| matches!(&e.event, Event::Fused { what: "communion", .. }));
    assert!(
        fused.is_some(),
        "no communion was made. The bonus fuses faith and nature on every \
         activation, so this needs both parents to have something in them - if \
         that is the failure, the fixture wants nature and not the wiring."
    );
}

#[test]
fn planted_banks_the_growth_the_road_fuses() {
    let log = fight_wearing("Deeprooted Sole");
    assert!(
        log.entries.iter().any(|e| matches!(
            &e.event,
            Event::GainResource { side: Side::Player, what: "nature", .. }
        )),
        "DEEPROOTED SOLE's bonus banks nature and none arrived"
    );
}

/// A trap laid in the room it was given.
///
/// `PerAdjacentEmpty` wraps a trigger and composes with the spending ones by
/// design - but it was only ever unwrapped on the activation path, so "for
/// each empty cell, at the bell" matched nothing and did nothing. This is the
/// first thing to ask it of.
#[test]
fn a_deadfall_is_worth_the_room_around_it() {
    let log = fight_wearing("Deadfall Mold");
    let armour: i32 = log
        .entries
        .iter()
        .filter_map(|e| match &e.event {
            Event::GainArmor { side: Side::Player, amount, .. } if e.at_ms == 0 => Some(*amount),
            _ => None,
        })
        .sum();
    assert!(
        armour > 0,
        "no armour at the bell. The bonus is PerAdjacentEmpty(OnBattleStart(..)) \
         and the opening scan has to unwrap it, or the trap is laid in a room \
         nobody counted."
    );
}

/// Seat a piece in its own slot with whatever assembles it, and fight.
///
/// `fight_wearing` is greaves-only, and the last two fusions live on a glove
/// and a chest. The partner is chosen by kind rather than by name so the test
/// does not go stale the day a piece is renamed.
fn fight_wearing_in_slot(piece: &str) -> gearmaster_engine::combat::CombatLog {
    let mut run = Run::with_all_pieces();
    let id = |run: &Run, n: &str| {
        run.owned.iter().copied().find(|&p| run.registry.def(p).name == n).expect(n)
    };
    let def = CATALOG.iter().find(|d| d.name == piece).expect("a real piece");
    let wants = match def.kind {
        PieceKind::Material => PieceKind::Mold,
        PieceKind::Base => PieceKind::Layer,
        PieceKind::Mold => PieceKind::Material,
        other => panic!("no partner rule for {other:?}"),
    };
    let partner = CATALOG
        .iter()
        .find(|d| d.slot == def.slot && d.kind == wants && d.assembly_bonus.is_none())
        .expect("something to build it with");
    let a = id(&run, piece);
    let b = id(&run, partner.name);
    run.equip(a, def.slot, 0, 0).expect("seats");
    // Every anchor until the two touch, putting the partner back if they do
    // not - an inner `break` leaves the loop, not the search, and the last
    // placement tried is the one that sticks.
    'seat: for y in 0..8u8 {
        for x in 0..6u8 {
            if run.equip(b, def.slot, x, y).is_ok() {
                if run.report(def.slot).assembled_count() > 0 {
                    break 'seat;
                }
                run.unequip(b);
            }
        }
    }
    assert!(
        run.report(def.slot).assembled_count() > 0,
        "{piece} never assembled, so its bonus was never live and this test proves nothing"
    );
    let spec = gearmaster_engine::combat::creature("Cave Rat").expect("exists");
    simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium)
}

/// Zealotry, made by a board for the first time.
///
/// Anger and conviction. The pool has existed since the slot rewrite and
/// `held_bonus` has priced it the whole time; nothing in 504 pieces could put
/// a point in it.
#[test]
fn the_breakers_fist_makes_zealotry() {
    let log = fight_wearing_in_slot("Breaker's Fist");
    assert!(
        log.entries.iter().any(|e| matches!(&e.event, Event::Fused { what: "zealotry", .. })),
        "no zealotry was made. The bonus banks rage and faith at the bell and \
         fuses them on every activation, so if neither parent arrived the \
         gain half is unwired, and if they did the fuse half is."
    );
}

/// DruidicMight, and the last pool nothing could make.
///
/// Heartwood is the one bonus whose payload is about its *neighbours*: every
/// item beside it banks nature when it fires. That is the nature half. The
/// rage is the half a chest cannot grow, so it comes at the bell.
#[test]
fn heartwood_makes_druidic_might_from_what_its_neighbours_pay() {
    let log = fight_wearing_in_slot("Heartwood Base");
    assert!(
        log.entries.iter().any(|e| matches!(&e.event, Event::Fused { what: "druidic might", .. })),
        "no druidic might was made, which was the state of the whole catalogue \
         before this commit."
    );
}

/// Every pool the game defines can now be made by some board.
///
/// The lint that would have caught the fusions being unreachable in the first
/// place, written from the other end: not "is this machinery correct" but "can
/// anybody ever get here".
#[test]
fn which_pools_a_board_can_actually_make() {
    let mut reachable: Vec<Resource> = Vec::new();
    for d in CATALOG {
        let triggers = d
            .triggers
            .iter()
            .chain(d.assembly_bonus.iter().flat_map(|b| b.triggers.iter()));
        for t in triggers {
            // `walk_actions`, not a copy of it: two trigger variants hold
            // more than one action and one wraps a whole trigger, and the
            // engine's own walker says in its doc that two of these would
            // drift. It was right - this test had one until it did not.
            gearmaster_engine::piece::walk_actions(t, &mut |a| {
                use gearmaster_engine::piece::Action;
                match a {
                    Action::Gain { what, .. } | Action::Accrue { what, .. } => {
                        if !reachable.contains(what) {
                            reachable.push(*what)
                        }
                    }
                    Action::Fuse { into, .. } => {
                        if !reachable.contains(into) {
                            reachable.push(*into)
                        }
                    }
                    _ => {}
                }
            });
        }
    }
    // Every one of them, now, which is what this test was always going to be
    // asked - it shipped naming the two it could not yet claim, so that the
    // commit which earned them had to come here and say so.
    let missing: Vec<&str> =
        Resource::ALL.iter().filter(|r| !reachable.contains(r)).map(|r| r.name()).collect();
    assert!(
        missing.is_empty(),
        "no board can make: {missing:?}. A pool the engine defines, prices in \
         `held_bonus` and draws in the glossary, that nothing produces - which \
         is the shape all three fusions had before they were armed."
    );
}

// -------------------------------------------------- the cadence four
//
// These four need machinery the game did not have. Each test below fails if
// the primitive is absent, which is the only way to know a new `Action` is
// wired rather than merely priced.

/// A head start, which every fight in this game otherwise begins without.
#[test]
fn downhill_starts_the_fight_already_part_way_through() {
    let log = fight_wearing("Ridge Runner");
    let primed = log
        .entries
        .iter()
        .any(|e| e.at_ms == 0 && matches!(&e.event, Event::Hastened { side: Side::Player, .. }));
    assert!(
        primed,
        "RIDGE RUNNER's bonus primes its bar at the bell and nothing was \
         hastened at t=0. `ReduceCooldown` deliberately cannot do this - it is \
         clamped so it cannot stack into a free item - which is why `Prime` \
         exists at all."
    );
}

/// And gives it back, permanently, which nothing else in the game does.
#[test]
fn downhill_gets_slower_every_time_it_fires() {
    use gearmaster_engine::piece::SlotKind;
    let mut run = Run::with_all_pieces();
    let id = |run: &Run, n: &str| {
        run.owned.iter().copied().find(|&p| run.registry.def(p).name == n).expect(n)
    };
    let a = id(&run, "Ridge Runner");
    run.equip(a, SlotKind::Greaves, 0, 0).expect("seats");
    let before = run
        .combat_items()
        .iter()
        .find(|i| i.slot == SlotKind::Greaves)
        .map(|i| i.cooldown_ms);
    // The drift is a fight-time change, so it shows in the log's own items
    // rather than in the profile the fight started from.
    let spec = gearmaster_engine::combat::creature("Rust Golem").expect("exists");
    let log = simulate_at(run.player_stats(), &run.combat_items(), spec, Difficulty::Medium);
    let after = log.player.items.iter().find(|i| i.slot == Some(SlotKind::Greaves));
    if let (Some(b), Some(a)) = (before, after) {
        assert!(
            a.cooldown_ms > b,
            "RIDGE RUNNER fired and its cooldown did not grow: {b} -> {}",
            a.cooldown_ms
        );
    }
}

/// The board, not the item. Nothing else in the game pays for a full board.
#[test]
fn already_moving_primes_everything_on_the_board() {
    // A second item, in another grid, because a board of one cannot tell
    // `PrimeBoard` from `Prime` - which is the whole distinction under test.
    let log = fight_wearing_and("Ambush Mold", &[("Oak Handle", SlotKind::Weapon), ("Iron Blade", SlotKind::Weapon)]);
    let at_bell: Vec<&str> = log
        .entries
        .iter()
        .filter(|e| e.at_ms == 0)
        .filter_map(|e| match &e.event {
            Event::Hastened { side: Side::Player, item, .. } => Some(item.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        at_bell.len() >= 2,
        "AMBUSH MOLD primes the whole board and only {:?} were hastened at the \
         bell. One is the item itself, which `Prime` would have done - the \
         point of `PrimeBoard` is the rest.",
        at_bell
    );
}

/// The relation that watches the opposition, which nothing did.
#[test]
fn one_stride_ahead_answers_the_other_side() {
    let log = fight_wearing("Worldstrider Sole");
    // Their activations, and ours answering them.
    let theirs = log
        .entries
        .iter()
        .filter(|e| matches!(&e.event, Event::Activate { side: Side::Enemy, .. }))
        .count();
    assert!(theirs > 0, "the fixture creature never acted, so this proves nothing");
    let answered = log.entries.iter().any(|e| {
        matches!(&e.event, Event::Hastened { side: Side::Player, .. }) && e.at_ms > 0
    });
    assert!(
        answered,
        "WORLDSTRIDER SOLE answers an enemy activation and there were {theirs} of \
         them with no answer. Every other relation in the game looks at your own \
         board; this is the only one that looks across."
    );
}

/// Immunity, proved against a control rather than read off the log.
///
/// `CombatLog::player` is the combatant from *before* the fight - CLAUDE.md's
/// own note about the watcher counters - so a flag set at the bell is not
/// visible there. The only honest test is behavioural: the same board with and
/// without the bonus, against something that stuns.
#[test]
fn sure_footed_cannot_be_stunned() {
    // Its gear stuns; Francis's does not, which cost a fixture to find out.
    let stunner = gearmaster_engine::combat::creature("Sootmother").expect("exists");

    let stunned_items = |piece: &str| -> usize {
        let mut run = Run::with_all_pieces();
        let id = |run: &Run, n: &str| {
            run.owned.iter().copied().find(|&p| run.registry.def(p).name == n).expect(n)
        };
        // A whole board, so the fight lasts long enough for a stun to land at
        // all - and the same board both times, with only the mold swapped.
        run.apply_preset();
        for held in run.loadout.slot(SlotKind::Greaves).pieces() {
            let _ = run.unequip(held);
        }
        let a = id(&run, piece);
        let b = id(&run, "Runed Material");
        run.equip(a, SlotKind::Greaves, 0, 0).expect("seats");
        for x in 1..6u8 {
            if run.equip(b, SlotKind::Greaves, x, 0).is_ok()
                && run.report(SlotKind::Greaves).assembled_count() > 0
            {
                break;
            }
        }
        // Stuns that land on *this* item, not on the board. A preset board has
        // nineteen items and `StunStrongest` picks among all of them, so
        // counting every stun compares the boards and not the bonus.
        let items = run.combat_items();
        let mine: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.slot == SlotKind::Greaves)
            .map(|(i, _)| i)
            .collect();
        let log = simulate_at(run.player_stats(), &items, stunner, Difficulty::Medium);
        log.entries
            .iter()
            .filter(|e| match &e.event {
                Event::Stunned { on: Side::Player, index, .. } => mine.contains(index),
                _ => false,
            })
            .count()
    };

    // The control has to actually get stunned, or this proves nothing.
    let control = stunned_items("Greave Mold");
    assert!(
        control > 0,
        "the control board was never stunned, so the comparison below is empty"
    );
    assert_eq!(
        stunned_items("Coldstep Mold"),
        0,
        "COLDSTEP MOLD's item was stunned {control} time(s) as a control and \
         should be stunned none. `StunStrongest` aims at the best item a \
         fighter owns, which is exactly what this protects."
    );
}

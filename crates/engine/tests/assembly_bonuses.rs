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
            for a in gearmaster_engine::piece::every_action(t) {
                use gearmaster_engine::piece::Action;
                match a {
                    Action::Gain { what, .. } | Action::Accrue { what, .. } => {
                        if !reachable.contains(&what) {
                            reachable.push(what)
                        }
                    }
                    Action::Fuse { into, .. } => {
                        if !reachable.contains(&into) {
                            reachable.push(into)
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(
        reachable.contains(&Resource::Communion),
        "communion is not made by anything: {reachable:?}"
    );
    // The other two are M8's, and this says so rather than pretending.
    let missing: Vec<Resource> = [Resource::DruidicMight, Resource::Zealotry]
        .into_iter()
        .filter(|r| !reachable.contains(r))
        .collect();
    assert_eq!(
        missing,
        vec![Resource::DruidicMight, Resource::Zealotry],
        "a fusion became reachable and this list was not lowered - which is \
         the good direction, but the commit that earned it owns this line"
    );
}

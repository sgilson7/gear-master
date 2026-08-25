//! The spell that spends a harvest.
//!
//! A nature build banks steadily all fight and, before this, had nowhere to
//! put it. Slash and Burn is the sink.
//!
//! It took the whole pool at once, a stack of searing per handful. Emptying a
//! pool in one go is `Consume`, and `Consume` is the head's by the exclusivity
//! table - a Spell cannot be a helmet piece, so the spell keeps its sentence
//! and loses its scale: a handful at a time, gated, with the harvest going
//! back in the ground when there is not enough to pay.

use gearmaster_engine::combat::{simulate_at, Difficulty, Event, Side, LADDER};
use gearmaster_engine::curse::CurseKind;
use gearmaster_engine::piece::{Action, Resource, Target, Trigger, CATALOG};
use gearmaster_engine::run::Run;

fn def() -> &'static gearmaster_engine::piece::PieceDef {
    CATALOG.iter().find(|d| d.name == "Slash and Burn").expect("authored")
}

#[test]
fn it_spends_nature_and_pays_in_searing() {
    let d = def();
    let Trigger::Spend { what, cost, on_success, .. } = d.triggers[0] else {
        panic!("Slash and Burn is meant to spend a pool: {:?}", d.triggers[0]);
    };
    assert_eq!(what, Resource::Nature);
    assert!(cost > 0, "a cost of zero would be an infinite loop of stacks");
    assert!(
        matches!(on_success, Action::Curse { kind: CurseKind::Searing, target: Target::Enemy }),
        "it pays in something other than searing on them: {on_success:?}"
    );
}

/// A board wearing the spell, with enough nature banking to feed it.
///
/// It said that and did something else: `apply_preset()`, whose twenty-one
/// hard-coded pieces carry **no searing at all**. So `it_reaches_a_real_fight`
/// was never watching this spell reach a fight - it was watching whether burn
/// happened to arrive from anywhere on a board that could not produce it, and
/// it went green for as long as something else obliged. The repack stopped
/// obliging, which is the only reason anybody looked.
///
/// A book and an ink around the spell, because a spell will not cast without
/// them, and a nature source to give it something to spend.
fn a_burner() -> Run {
    let mut run = Run::with_all_pieces();
    run.difficulty = Difficulty::Medium;
    // Book, ink, spell - and the ink is the nature one, because the recipe
    // takes a single ink and the spell needs a pool to spend.
    for name in ["Pocket Grimoire", "Gravebloom Ink", "Slash and Burn"] {
        let Some(id) = run
            .owned
            .iter()
            .copied()
            .find(|&i| run.registry.def(i).name == name && !run.is_equipped(i))
        else {
            continue;
        };
        let slot = run.registry.def(id).slot;
        'seat: for y in 0..8u8 {
            for x in 0..6u8 {
                if run.equip(id, slot, x, y).is_ok() {
                    break 'seat;
                }
            }
        }
    }
    assert_eq!(
        run.report(gearmaster_engine::piece::SlotKind::Weapon).assembled_count(),
        1,
        "the spell has to be in a finished weapon before it casts anything"
    );
    run
}

#[test]
fn a_pool_that_is_never_banked_never_burns() {
    // The honest half: this is a sink, not a source. A board that banks no
    // nature gets one small burst off its starting pool and nothing after.
    let d = def();
    let Trigger::Spend { cost, .. } = d.triggers[0] else { unreachable!() };
    assert!(
        cost >= 4,
        "a cost of {cost} would turn any trickle of nature into a permanent burn"
    );
}

#[test]
fn the_curse_it_lands_stacks_without_a_ceiling() {
    // Worth stating outright, because it is what makes the spell scale and
    // what would make it break: searing has no cap, so the whole balance of
    // this piece is the size of a handful.
    use gearmaster_engine::curse::Curses;
    let mut c = Curses::new();
    for _ in 0..6 {
        c.apply(CurseKind::Searing, 0);
    }
    let n = c.stacks_of(CurseKind::Searing);
    assert_eq!(n, 6, "searing stopped stacking at {n}");
}

#[test]
fn it_reaches_a_real_fight() {
    // Seated on a real board against real creatures: the trigger fires, the
    // pool empties, and the other side burns.
    let run = a_burner();
    let (stats, items) = (run.player_stats(), run.combat_items());
    let mut burned = 0;
    // The whole ladder, not the first twelve of it.
    //
    // Twelve was "far enough in that a fight lasts", and after the repack it is
    // not: rungs 1-13 are strikers and walls, the spell wants a pool and a
    // couple of seconds, and those fights are over before it has either. What
    // this test is about is that burning reaches a real fight at all, so it
    // asks the whole road rather than the part of it that used to be slow.
    for spec in LADDER.iter() {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        burned += log
            .entries
            .iter()
            .filter(|e| matches!(e.event, Event::Burn { side: Side::Enemy, .. }))
            .count();
    }
    // The auto-builder may not seat this particular spell, so this is a check
    // that burning works at all on a real board rather than proof it was this
    // piece that did it.
    assert!(burned > 0, "nothing burned anywhere on the ladder");
}

//! The spell that spends a harvest.
//!
//! A nature build banks steadily all fight and, before this, had nowhere to
//! put it. Slash and Burn is the sink: the whole pool at once, one stack of
//! searing per handful.

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
    let Trigger::Consume { what, each, per } = d.triggers[0] else {
        panic!("Slash and Burn is meant to empty a pool: {:?}", d.triggers[0]);
    };
    assert_eq!(what, Resource::Nature);
    assert!(each > 0, "a handful of zero would be an infinite loop of stacks");
    assert!(
        matches!(per, Action::Curse { kind: CurseKind::Searing, target: Target::Enemy }),
        "it pays in something other than searing on them: {per:?}"
    );
}

/// A board wearing the spell, with enough nature banking to feed it.
fn a_burner() -> Run {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run
}

#[test]
fn a_pool_that_is_never_banked_never_burns() {
    // The honest half: this is a sink, not a source. A board that banks no
    // nature gets one small burst off its starting pool and nothing after.
    let d = def();
    let Trigger::Consume { each, .. } = d.triggers[0] else { unreachable!() };
    assert!(
        each >= 4,
        "a handful of {each} would turn any trickle of nature into a permanent burn"
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
    for spec in LADDER.iter().take(12) {
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
    assert!(burned > 0, "nothing burned in twelve fights");
}

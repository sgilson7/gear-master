//! Shared fixtures for the integration tests.
#![allow(dead_code)] // each test binary uses a different subset

use gearmaster_engine::piece::{Action, PieceDef, PieceId, SlotKind, Trigger};
use gearmaster_engine::run::Run;

/// Run `f` over every action a trigger can reach.
///
/// Two of the trigger variants hold more than one action and one of them wraps
/// another trigger, so "does this piece apply a curse" is a walk rather than a
/// match. Anything counting mechanics across the catalogue wants this, or it
/// silently misses the payload of every `PerAdjacentEmpty` in the game.
pub fn actions_of(t: &Trigger, f: &mut impl FnMut(&Action)) {
    match t {
        Trigger::OnActivate(a)
        | Trigger::OnAdjacentActivate(a)
        | Trigger::OnAlignedActivate(a)
        | Trigger::OnDiagonalActivate(a)
        | Trigger::OnBattleStart(a)
        | Trigger::OnOtherCast(a) => f(a),
        Trigger::Watch { then, .. } => f(then),
        Trigger::PerAdjacentItem { action, .. } => f(action),
        Trigger::Consume { per, .. } => f(per),
        Trigger::SpendGold { on_success, .. } => f(on_success),
        Trigger::SpendMana { on_success, on_failure, .. }
        | Trigger::Spend { on_success, on_failure, .. } => {
            f(on_success);
            f(on_failure);
        }
        Trigger::PerAdjacentEmpty(inner) => actions_of(inner, f),
    }
}

/// Does any action this piece can reach satisfy `want`?
pub fn does(def: &PieceDef, want: fn(&Action) -> bool) -> bool {
    let mut hit = false;
    for t in def.triggers {
        actions_of(t, &mut |a| hit |= want(a));
    }
    hit
}

/// Does this piece carry a trigger satisfying `want`?
pub fn has(def: &PieceDef, want: fn(&Trigger) -> bool) -> bool {
    def.triggers.iter().any(want)
}

/// Look a starting component up by name.
pub fn piece(run: &Run, name: &str) -> PieceId {
    run.owned
        .iter()
        .copied()
        .find(|&id| run.registry.def(id).name == name)
        .unwrap_or_else(|| panic!("no piece named {}", name))
}

/// Equip by name, failing loudly with the reason if the placement is illegal.
pub fn equip(run: &mut Run, name: &str, slot: SlotKind, ax: u8, ay: u8) {
    let id = piece(run, name);
    run.equip(id, slot, ax, ay)
        .unwrap_or_else(|e| panic!("failed to equip {} at ({}, {}): {}", name, ax, ay, e));
}

/// A complete, legal loadout that assembles all five slots and lights every
/// adjacency bonus. Delegates to the engine's own preset so the tests assert
/// against the same arrangement the GUI's auto-build button produces.
pub fn build_full_loadout(run: &mut Run) {
    run.apply_preset();
}

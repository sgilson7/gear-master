//! Shared fixtures for the integration tests.
#![allow(dead_code)] // each test binary uses a different subset

use gearmaster_engine::piece::{PieceId, SlotKind};
use gearmaster_engine::run::Run;

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

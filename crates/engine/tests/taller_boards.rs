//! Grids that can be given another row.
//!
//! The guarantee worth testing is not that the row appears - that is one
//! `resize` - but that nothing already on the board moves when it does. A
//! reward that rearranged a board you had spent a run packing would be worse
//! than no reward.

use gearmaster_engine::piece::SlotKind;
use gearmaster_engine::run::Run;
use gearmaster_engine::share::{export, import};
use gearmaster_engine::slot::{SLOT_H, SLOT_W};

/// Every piece on the board, by name and where it sits.
fn snapshot(run: &Run) -> Vec<(String, usize, u8, u8, u8)> {
    let mut out = Vec::new();
    for kind in SlotKind::ALL {
        let slot = run.loadout.slot(kind);
        for id in slot.pieces() {
            let Some((x, y)) = slot.anchor_of(id) else { continue };
            out.push((
                run.registry.def(id).name.to_string(),
                kind.index(),
                x,
                y,
                run.registry.rotation(id),
            ));
        }
    }
    out.sort();
    out
}

#[test]
fn a_board_starts_eight_rows_tall() {
    let run = Run::new();
    assert_eq!(run.loadout.rows(), SLOT_H);
    assert_eq!(run.extra_rows, 0);
    for kind in SlotKind::ALL {
        assert_eq!(run.loadout.slot(kind).rows(), SLOT_H);
    }
}

#[test]
fn growing_moves_nothing_that_was_already_down() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    let before = snapshot(&run);
    assert!(!before.is_empty(), "the preset built nothing, so this proves nothing");
    let items_before = run.combat_items().len();

    run.grow_boards(1);

    assert_eq!(run.loadout.rows(), SLOT_H + 1);
    assert_eq!(run.extra_rows, 1);
    assert_eq!(snapshot(&run), before, "growing the board shuffled the pieces on it");
    assert_eq!(
        run.combat_items().len(),
        items_before,
        "growing the board broke an item that was assembled"
    );
}

#[test]
fn the_new_row_is_empty_and_can_be_used() {
    let mut run = Run::with_all_pieces();
    run.grow_boards(1);
    let y = SLOT_H; // the row that did not exist a moment ago

    for kind in SlotKind::ALL {
        for x in 0..SLOT_W {
            assert_eq!(run.loadout.slot(kind).get(x, y), None, "{kind:?} ({x},{y}) came up full");
        }
    }

    // And something can actually be put there.
    let id = run
        .owned
        .iter()
        .copied()
        .find(|&i| run.registry.def(i).name == "Leech Bead")
        .expect("a one-cell accessory");
    assert!(
        run.equip(id, SlotKind::Weapon, 0, y).is_ok(),
        "the new row would not take a piece"
    );
    assert_eq!(run.loadout.slot(SlotKind::Weapon).get(0, y), Some(id));
}

#[test]
fn nothing_can_be_placed_below_the_last_row() {
    let mut run = Run::with_all_pieces();
    let id = run
        .owned
        .iter()
        .copied()
        .find(|&i| run.registry.def(i).name == "Leech Bead")
        .expect("a one-cell accessory");
    assert!(run.equip(id, SlotKind::Weapon, 0, SLOT_H).is_err(), "placed off the bottom");

    run.grow_boards(1);
    assert!(run.equip(id, SlotKind::Weapon, 0, SLOT_H).is_ok(), "the new row is not usable");
    assert!(
        run.equip(id, SlotKind::Weapon, 0, SLOT_H + 1).is_err(),
        "placed off the bottom of the taller board"
    );
}

#[test]
fn a_monster_keeps_the_ordinary_eight() {
    // The player's boards grow; nothing else's does.
    use gearmaster_engine::combat::LADDER;
    let (_, loadout) = LADDER[20].loadout();
    assert_eq!(loadout.rows(), SLOT_H);
}

#[test]
fn a_shared_code_carries_the_extra_rows() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.grow_boards(1);

    // Put something in the new row, which is the part a version 1 reader
    // would have dropped without saying so.
    let id = run
        .owned
        .iter()
        .copied()
        .find(|&i| run.registry.def(i).name == "Leech Bead" && !run.is_equipped(i))
        .expect("a spare one-cell accessory");
    run.equip(id, SlotKind::Weapon, 0, SLOT_H).expect("the new row takes it");

    let code = export(&run);
    let back = import(&code).expect("the code reads back");
    assert_eq!(back.extra_rows, 1);

    let (_, lo) = back.loadout();
    assert_eq!(lo.rows(), SLOT_H + 1, "the shared board came back short");
    assert!(
        lo.slot(SlotKind::Weapon).get(0, SLOT_H).is_some(),
        "the piece in the extra row did not survive the round trip"
    );
}

#[test]
fn growth_survives_the_undo_snapshot() {
    let mut run = Run::with_all_pieces();
    run.apply_preset();
    run.grow_boards(1);
    let id = run
        .owned
        .iter()
        .copied()
        .find(|&i| run.registry.def(i).name == "Leech Bead" && !run.is_equipped(i))
        .expect("a spare one-cell accessory");
    run.equip(id, SlotKind::Weapon, 0, SLOT_H).expect("placed in the new row");

    run.undo();
    assert_eq!(run.loadout.rows(), SLOT_H + 1, "undo shrank the board back");
}

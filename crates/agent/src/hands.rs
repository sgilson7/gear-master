//! Building a board with a mouse and two eyes.
//!
//! The pilot cannot simulate a fight, so it cannot pack the way a search
//! packs. It does what a person does: put a piece down, look at what the slot
//! says, and take it back if it did not help. `Undo` is a button in the game
//! (`run.rs:4666`, depth 40) and that is what makes this legal rather than
//! clever.
//!
//! ## Why not just seat everything
//!
//! The A1 control seated the first thing that fitted and reached rung 2. A
//! loose piece pays its flat stats and **never acts** - and past the shallow
//! end the ladder is not cleared by flat stats. So the whole of this module is
//! about finishing items: pick the seat that assembles something, and where
//! nothing does, pick the seat that leaves the most room for the piece that
//! will.

use crate::sense::Sense;
use gearmaster_console::{Console, PieceId, SlotKind, Verb};

/// What one pass of the hands did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Packed {
    pub presses: usize,
    pub seated: usize,
    pub left_in_tray: usize,
    pub items: usize,
}

/// Seat as much of the tray as helps, one piece at a time.
///
/// Greedy over pieces and exhaustive over seats: for each piece in the tray,
/// every legal anchor is tried, the board is read, and the best one is kept.
/// A piece that improves nothing anywhere is left in the tray - because a
/// board is not a container, and a cell spent on something inert is a cell the
/// next piece cannot have.
pub fn pack(c: &mut Console, budget: usize) -> Packed {
    let mut out = Packed::default();
    // Rotations are pressed rather than assumed: a player turns the piece in
    // their hand and then puts it down, so four turns is four presses and the
    // fourth returns it to where it started.
    const TURNS: usize = 4;

    loop {
        if out.presses >= budget {
            break;
        }
        let before = Sense::of(&c.view()).worth();
        let tray = c.tray_ids();
        if tray.is_empty() {
            break;
        }

        let mut best: Option<(i64, PieceId, SlotKind, u8, u8, usize)> = None;
        for &piece in &tray {
            for turn in 0..TURNS {
                if turn > 0 {
                    if !c.apply(Verb::Rotate { piece }).changed {
                        // It will not turn where it is. Nothing to explore.
                        continue;
                    }
                    out.presses += 1;
                }
                for slot in SlotKind::ALL {
                    for (x, y) in c.anchors_for(piece, slot) {
                        if out.presses >= budget {
                            break;
                        }
                        let v = Verb::Place { piece, slot, x, y };
                        if !c.apply(v).ok {
                            continue;
                        }
                        out.presses += 1;
                        let worth = Sense::of(&c.view()).worth();
                        c.apply(Verb::Undo);
                        out.presses += 1;
                        if best.is_none_or(|(b, ..)| worth > b) {
                            best = Some((worth, piece, slot, x, y, turn));
                        }
                    }
                }
            }
            // Back to where it started, so the next piece is judged against
            // the same board this one was.
            let turns_made = TURNS - 1;
            for _ in 0..(TURNS - turns_made % TURNS) % TURNS {
                c.apply(Verb::Rotate { piece });
                out.presses += 1;
            }
        }

        let Some((worth, piece, slot, x, y, turn)) = best else { break };
        if worth <= before {
            // Nothing left in the tray makes this board better.
            break;
        }
        for _ in 0..turn {
            c.apply(Verb::Rotate { piece });
            out.presses += 1;
        }
        if c.apply(Verb::Place { piece, slot, x, y }).ok {
            out.presses += 1;
            out.seated += 1;
        } else {
            break;
        }
    }

    let v = c.view();
    out.left_in_tray = v.tray.len();
    out.items = Sense::of(&v).items;
    out
}

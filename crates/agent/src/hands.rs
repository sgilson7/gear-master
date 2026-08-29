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

use crate::lesson::{describe, Lesson};
use crate::sense::Sense;
use gearmaster_console::{Console, PieceId, SlotKind, Verb};

/// What one pass of the hands did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Packed {
    pub presses: usize,
    pub seated: usize,
    pub left_in_tray: usize,
    pub items: usize,
    pub cleared: usize,
}

/// Make room for what will not fit, by taking a grid apart and rebuilding it.
///
/// **The board fills up.** A run at rung 45 carries twelve to sixteen items
/// across 228 of 240 cells and fifteen thousand gold it cannot spend, because
/// `pack` only ever *adds* - it has no move that removes. Buying is pointless
/// when there is nowhere to put anything, which is why a deep run churns the
/// shop (seven hundred bought, seven hundred sold) and the board does not
/// change.
///
/// `ClearSlot` is the answer and it is one press. It is also one of the four
/// verbs **no interface had** before A1 (`console/tests/parity.rs`) - the
/// engine could empty one grid and no person could ask it to. The verb that
/// unblocks the deep game is the verb nobody could reach.
///
/// Greedy and unguarded: the slot is emptied and re-packed from everything
/// available, which is strictly more choice than it had when it was filled one
/// piece at a time. It can come out worse, and the only honest check is the
/// clear rate.
pub fn reseat(c: &mut Console, budget: usize) -> Packed {
    reseat_with(c, budget, None)
}

/// The same, with a prior ranking the seats of the rebuilt grid.
pub fn reseat_with(c: &mut Console, budget: usize, prior: Option<&dyn Prior>) -> Packed {
    let mut out = Packed::default();
    let tray = c.tray_ids();
    if tray.is_empty() || budget == 0 {
        return out;
    }
    // The grid the leftovers want. A piece belongs to one slot, so this is a
    // count rather than a choice.
    let v = c.view();
    let mut wanted: [usize; 5] = [0; 5];
    for p in &v.tray {
        wanted[SlotKind::ALL.iter().position(|&k| k == p.slot).unwrap_or(0)] += 1;
    }
    let Some((at, &n)) = wanted.iter().enumerate().max_by_key(|(_, &n)| n) else {
        return out;
    };
    if n == 0 {
        return out;
    }
    let slot = SlotKind::ALL[at];
    if !c.apply(Verb::ClearSlot { slot }).ok {
        return out;
    }
    out.presses += 1;
    out.cleared += 1;
    let again = match prior {
        Some(p) => pack_with(c, budget.saturating_sub(1), p),
        None => pack(c, budget.saturating_sub(1)),
    };
    out.presses += again.presses;
    out.seated += again.seated;
    out.items = again.items;
    out.left_in_tray = again.left_in_tray;
    out
}

/// Seat as much of the tray as helps, writing down what each trial taught.
///
/// The hands already try every seat and read the board; this is the same walk
/// with the readings kept. A run produces a hundred and fifty thousand of
/// them, which is the training set nobody was collecting.
pub fn pack_recording(
    c: &mut Console,
    budget: usize,
    out: &mut Vec<Lesson>,
) -> Packed {
    pack_inner(c, budget, Some(out), None)
}

/// Seat as much of the tray as helps, one piece at a time.
///
/// Greedy over pieces and exhaustive over seats: for each piece in the tray,
/// every legal anchor is tried, the board is read, and the best one is kept.
/// A piece that improves nothing anywhere is left in the tray - because a
/// board is not a container, and a cell spent on something inert is a cell the
/// next piece cannot have.
pub fn pack(c: &mut Console, budget: usize) -> Packed {
    pack_inner(c, budget, None, None)
}

/// A prior over seats: given a candidate, how good does it look?
///
/// Used to try the promising seats first and stop early, which is the whole
/// point of a learned one - the hands' cost is the number of seats they try.
pub trait Prior {
    fn score(&self, x: &[f32; crate::lesson::FEATURES]) -> f32;
    /// How many seats to actually try, per piece, in rank order.
    fn keep(&self) -> usize {
        8
    }
}

/// Seat with a prior deciding which seats are worth the press.
pub fn pack_with(c: &mut Console, budget: usize, prior: &dyn Prior) -> Packed {
    pack_inner(c, budget, None, Some(prior))
}

fn pack_inner(
    c: &mut Console,
    budget: usize,
    mut record: Option<&mut Vec<Lesson>>,
    prior: Option<&dyn Prior>,
) -> Packed {
    let mut out = Packed::default();
    // Rotations are pressed rather than assumed: a player turns the piece in
    // their hand and then puts it down, so four turns is four presses and the
    // fourth returns it to where it started.
    const TURNS: usize = 4;

    loop {
        if out.presses >= budget {
            break;
        }
        let before = Sense::quick(c).worth();
        let tray = c.tray_ids();
        if tray.is_empty() {
            break;
        }

        let mut best: Option<(i64, PieceId, SlotKind, u8, u8, usize)> = None;
        let view = (record.is_some() || prior.is_some()).then(|| c.view());
        for &piece in &tray {
            for turn in 0..TURNS {
                if turn > 0 {
                    if !c.apply(Verb::Rotate { piece }).changed {
                        // It will not turn where it is. Nothing to explore.
                        continue;
                    }
                    out.presses += 1;
                }
                // Every seat this rotation offers, and - where a prior is
                // guiding - only the ones it thinks are worth the press.
                let mut seats: Vec<(SlotKind, u8, u8)> = Vec::new();
                for slot in SlotKind::ALL {
                    for (x, y) in c.anchors_for(piece, slot) {
                        seats.push((slot, x, y));
                    }
                }
                if let (Some(p), Some(v)) = (prior, view.as_ref()) {
                    if let Some(card) = v.tray.iter().find(|t| t.id == Some(piece)) {
                        let mut ranked: Vec<((SlotKind, u8, u8), f32)> = seats
                            .iter()
                            .map(|&(s, x, y)| ((s, x, y), p.score(&describe(v, card, s, x, y))))
                            .collect();
                        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                        seats = ranked.into_iter().take(p.keep()).map(|(s, _)| s).collect();
                    }
                }
                for (slot, x, y) in seats {
                    if out.presses >= budget {
                        break;
                    }
                    let before = Sense::quick(c).worth();
                    let v = Verb::Place { piece, slot, x, y };
                    if !c.apply(v).ok {
                        continue;
                    }
                    out.presses += 1;
                    let worth = Sense::quick(c).worth();
                    c.apply(Verb::Undo);
                    out.presses += 1;
                    if let (Some(rec), Some(vw)) = (record.as_deref_mut(), view.as_ref()) {
                        if let Some(card) = vw.tray.iter().find(|t| t.id == Some(piece)) {
                            rec.push(Lesson {
                                x: describe(vw, card, slot, x, y),
                                // Scaled so the target is order-one rather
                                // than in the thousands.
                                y: (worth - before) as f32 / 400.0,
                            });
                        }
                    }
                    if best.is_none_or(|(b, ..)| worth > b) {
                        best = Some((worth, piece, slot, x, y, turn));
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

    let end = Sense::quick(c);
    out.left_in_tray = c.tray_ids().len();
    out.items = end.items;
    out
}

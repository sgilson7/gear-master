//! A run, written down.
//!
//! One string that says what somebody built and how far they got, short enough
//! to paste into a message. It is not a save file - it does not restore a run
//! in progress - it is a record of a board, so a build can be sent to somebody
//! else and looked at.
//!
//! Deliberately plain: the alphabet is base-32 with the ambiguous letters
//! removed, so a code survives being read aloud, retyped, or mangled by a chat
//! client that thinks it knows about capitals.

use crate::piece::{PieceRegistry, SlotKind, CATALOG};
use crate::run::Run;

/// No I, L, O, U - the four that get misread or turn a code into a word.
const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Bumped when the shape of a code changes. Version 2 carries the board
/// height: a run that has been given extra rows packs pieces into them, and a
/// reader that assumed eight would drop everything below that line without
/// saying so.
const VERSION: u32 = 2;

fn encode(vals: &[u32]) -> String {
    let mut out = String::new();
    for (i, v) in vals.iter().enumerate() {
        // Five bits at a time, most significant first, dropping leading zeros
        // but never emitting nothing.
        let mut buf = Vec::new();
        let mut v = *v;
        loop {
            buf.push(ALPHABET[(v & 31) as usize] as char);
            v >>= 5;
            if v == 0 {
                break;
            }
        }
        out.extend(buf.iter().rev());
        if i + 1 < vals.len() {
            out.push('-');
        }
    }
    out
}

fn decode(s: &str) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for part in s.split('-') {
        if part.is_empty() {
            return None;
        }
        let mut v: u32 = 0;
        for c in part.chars() {
            let up = c.to_ascii_uppercase() as u8;
            let at = ALPHABET.iter().position(|&a| a == up)?;
            v = v.checked_mul(32)?.checked_add(at as u32)?;
        }
        out.push(v);
    }
    Some(out)
}

/// What a shared code says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    pub rung: usize,
    /// Rows this run had been given beyond the usual eight.
    pub extra_rows: u8,
    pub wins: u32,
    pub losses: u32,
    pub gold: i32,
    /// Theme id, so a code from a themed run reads back in its own words.
    pub theme: String,
    pub classes: Vec<String>,
    /// Every placed component: catalogue index, slot, x, y, rotation.
    pub placed: Vec<(usize, SlotKind, u8, u8, u8)>,
}

impl Shared {
    /// The board this code describes, laid out for looking at.
    pub fn loadout(&self) -> (PieceRegistry, crate::loadout::Loadout) {
        let mut reg = PieceRegistry::new();
        let mut lo = crate::loadout::Loadout::new();
        // Grow first, or every piece the sharer had put in the extra rows is
        // quietly refused by `can_place` and the board reads as half-empty.
        lo.grow(self.extra_rows);
        for &(def, slot, x, y, rot) in &self.placed {
            if def >= CATALOG.len() {
                continue;
            }
            let id = reg.alloc(def);
            reg.set_rotation(id, rot);
            if lo.can_place(&reg, id, slot, x, y).is_ok() {
                lo.slot_mut(slot).place(&reg, id, x, y);
            }
        }
        for kind in SlotKind::ALL {
            crate::loadout::lock_assembled_in(&mut lo, &reg, kind);
        }
        (reg, lo)
    }
}

fn slot_index(s: SlotKind) -> u32 {
    SlotKind::ALL.iter().position(|&k| k == s).unwrap_or(0) as u32
}

fn slot_of(i: u32) -> SlotKind {
    SlotKind::ALL[(i as usize).min(SlotKind::ALL.len() - 1)]
}

/// Write a run down.
pub fn export(run: &Run) -> String {
    let mut vals: Vec<u32> = vec![VERSION, run.rung as u32, run.wins, run.losses, run.gold.max(0) as u32];
    // Theme and classes by index, so the code carries no words of its own.
    vals.push(
        crate::theme::THEMES.iter().position(|t| t.id == run.theme.id).unwrap_or(0) as u32,
    );
    vals.push(run.extra_rows as u32);
    vals.push(run.classes.len() as u32);
    for c in &run.classes {
        vals.push(crate::class::CLASSES.iter().position(|k| k.name == c.name).unwrap_or(0) as u32);
    }
    let mut placed: Vec<(usize, SlotKind, u8, u8, u8)> = Vec::new();
    for kind in SlotKind::ALL {
        let slot = run.loadout.slot(kind);
        for id in slot.pieces() {
            let Some((x, y)) = slot.anchor_of(id) else { continue };
            placed.push((run.registry.def_index(id), kind, x, y, run.registry.rotation(id)));
        }
    }
    vals.push(placed.len() as u32);
    for (def, kind, x, y, rot) in &placed {
        // One number a piece: index, slot, x, y and rotation packed together,
        // which keeps a full five-slot board inside a code you can paste.
        //
        // `y` takes four bits and `x` three. It used to be the other way
        // round, which was fine while every board was eight rows tall and
        // silently wrong the moment one was nine: row eight overflowed into
        // the column field and the piece came back somewhere else entirely.
        // Six columns need three bits; sixteen rows is room to spare.
        vals.push(
            (*def as u32) << 12
                | slot_index(*kind) << 9
                | (*x as u32) << 6
                | (*y as u32) << 2
                | *rot as u32,
        );
    }
    encode(&vals)
}

/// Read one back. `None` if it is not a code, or not one this build knows.
pub fn import(code: &str) -> Option<Shared> {
    let vals = decode(code.trim())?;
    let mut it = vals.into_iter();
    let mut next = || it.next();
    if next()? != VERSION {
        return None;
    }
    let rung = next()? as usize;
    let wins = next()?;
    let losses = next()?;
    let gold = next()? as i32;
    let theme = crate::theme::THEMES
        .get(next()? as usize)
        .map(|t| t.id.to_string())
        .unwrap_or_else(|| "plain".into());
    let extra_rows = next()? as u8;
    let n_classes = next()?;
    let mut classes = Vec::new();
    for _ in 0..n_classes {
        let i = next()? as usize;
        classes.push(crate::class::CLASSES.get(i)?.name.to_string());
    }
    let n_placed = next()?;
    let mut placed = Vec::new();
    for _ in 0..n_placed {
        let v = next()?;
        placed.push((
            (v >> 12) as usize,
            slot_of((v >> 9) & 7),
            ((v >> 6) & 7) as u8,
            ((v >> 2) & 15) as u8,
            (v & 3) as u8,
        ));
    }
    Some(Shared { rung, extra_rows, wins, losses, gold, theme, classes, placed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_survives_the_round_trip() {
        let mut run = Run::with_all_pieces();
        run.apply_preset();
        run.skip_to(12);
        run.gold = 417;

        let code = export(&run);
        let back = import(&code).expect("it reads back");
        assert_eq!(back.rung, run.rung);
        assert_eq!(back.gold, run.gold);
        assert_eq!(back.placed.len(), run.loadout.slots.iter().map(|s| s.pieces().len()).sum::<usize>());

        // And the board it describes is the board that was written down.
        let (reg, lo) = back.loadout();
        for kind in SlotKind::ALL {
            let want: Vec<&str> = run
                .loadout
                .slot(kind)
                .pieces()
                .iter()
                .map(|&p| run.registry.def(p).name)
                .collect();
            let got: Vec<&str> =
                lo.slot(kind).pieces().iter().map(|&p| reg.def(p).name).collect();
            assert_eq!(got, want, "{:?} came back different", kind);
        }
    }

    #[test]
    fn the_alphabet_has_no_letters_anyone_confuses() {
        for bad in [b'I', b'L', b'O', b'U'] {
            assert!(!ALPHABET.contains(&bad), "{} is in the alphabet", bad as char);
        }
        assert_eq!(ALPHABET.len(), 32);
    }

    #[test]
    fn nonsense_is_refused_rather_than_guessed_at() {
        assert!(import("").is_none());
        assert!(import("not a code").is_none());
        assert!(import("ZZZZ-ZZZZ").is_none(), "a well-formed code of the wrong version");
        // version, rung, wins, losses, gold, theme, extra rows, no classes,
        // no pieces. Spelled out rather than round-tripped, so a change to the
        // format has to be noticed here too.
        assert!(import("2-0-0-0-0-0-0-0-0").is_some(), "an empty board is still a run");
        assert!(import("1-0-0-0-0-0-0-0").is_none(), "a version 1 code is not a version 2 one");
    }

    #[test]
    fn a_code_is_short_enough_to_paste() {
        let mut run = Run::with_all_pieces();
        run.apply_preset();
        let code = export(&run);
        assert!(code.len() < 400, "a full board came to {} characters", code.len());
    }

    #[test]
    fn it_reads_back_the_same_however_it_was_typed() {
        let mut run = Run::with_all_pieces();
        run.apply_preset();
        let code = export(&run);
        assert_eq!(import(&code.to_lowercase()), import(&code));
        assert_eq!(import(&format!("  {}  ", code)), import(&code));
    }
}

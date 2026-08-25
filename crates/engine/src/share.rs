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
/// Bumped when the shape of a code changes.
///
/// Version 2 carries the board height: a run that has been given extra rows
/// packs pieces into them, and a reader that assumed eight would drop
/// everything below that line without saying so.
///
/// Version 3 carries five of them. One number was the whole answer while the
/// only thing handing out room gave a row to every board at once; the Depth
/// gives one row to a board of your choice, and a code that averaged that
/// would put pieces in a row the sharer's board did not have - or drop the
/// ones in the row it did. Same fault as version 2's, one slot down.
const VERSION: u32 = 3;

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
    /// Rows every board was given beyond the usual eight.
    pub extra_rows: u8,
    /// Rows each board has beyond `extra_rows`, indexed by `SlotKind::index`.
    ///
    /// Zero everywhere for a version 1 or 2 code, which is exactly right: a
    /// code written before one board could outgrow the others describes a
    /// board where none of them had.
    pub slot_rows: [u8; 5],
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
        for k in SlotKind::ALL {
            lo.grow_one(k, self.slot_rows[k.index()]);
        }
        for &(def, slot, x, y, rot) in &self.placed {
            if def >= CATALOG.len() {
                continue;
            }
            let id = reg.alloc(def);
            reg.set_rotation(id, rot);
            if lo.can_place(&reg, id, slot, x, y).is_ok() {
                lo.slot_mut(slot).place(&reg, id, x, y);
                // Lock as it completes, not once at the end.
                //
                // A finished board is packed to within a cell or two of full,
                // so nearly everything on it touches nearly everything else.
                // Deriving items from that in a single pass at the end asks
                // which pieces are connected, and on a dense board the answer
                // is "most of them" - which is how a weapon grid of nineteen
                // pieces came back holding one item. Locking each item the
                // moment it assembles is what the player was doing while
                // building it: a locked item is finished, nothing may join it,
                // and the next piece packs flush against it rather than into
                // it.
                crate::loadout::lock_assembled_in(&mut lo, &reg, slot);
            }
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
/// A second complete run, built by somebody else entirely.
///
/// Worth keeping beside `A_WINNING_RUN` because it wins the same game a
/// different way, and having only one finished board to measure against meant
/// every tuning decision was really a decision about one person's taste.
///
/// Seventy-six pieces at ninety-eight percent of the cells - marginally
/// tighter than the owner's - but the shape is not the same. Half of what is
/// on this board is deliberately *not* assembled: twelve finished items and
/// twelve loose groups seated purely for their flat stats, because a piece
/// that cannot finish an item still pays its base stats and a cell left empty
/// pays nothing. The owner's board finishes almost everything it seats.
///
/// It also went through the towns - it carries Piety and Tired - and took
/// Trundle at the roadside, which no board here had done.
///
/// The four class tokens differ from the string the player copied. Classes are
/// written down as positions in `CLASSES`, and for one day that array had the
/// three town classes at the front of it, which is when this run was shared -
/// see `the_class_order_is_append_only`. Every placement in the code is
/// untouched; only those four numbers were re-pointed.
pub const A_FRIENDS_RUN: &str = "2-1J-1J-2-74K-0-0-4-4-R-M-P-2C-1GR02-11441-H881-11405-11065-ZC28-E80D-J84C-12C8C-1KC2H-10WAH-ZC0R-E44R-1ERAS-7RG1-1JGM1-144P0-1MGP6-14WT5-1MGP8-138GC-1PCMC-78MG-ERTH-148GN-1JGMN-154PN-1PCRR-1PCPW-P501-1F141-FN81-1NX24-6S08-X4A-15X0D-1718C-P52K-1FS6G-HN8H-650M-1HN6M-6S8M-1A90R-1NX2R-5H6R-498T-5X0W-18HG1-16HM0-10XT1-85J4-7XR7-1P1GB-49MA-161RF-3NJG-115MH-11NGM-FNPN-5STM-119JR-1KY00-Q220-BT63-V606-SY0A-RY48-SY88-C26D-1HT0G-D64K-1GP6P-RY0R-TY4T-RT8S";

/// A run that cleared the whole ladder, shared by the game's owner.
///
/// Kept here as a fixture rather than in one test file, because it is the only
/// high-end board in the project that a human actually built: seventy-five
/// pieces across five boards at ninety-seven percent of the cells, which is
/// roughly twice what the packing solver manages. Anything that needs to know
/// what a finished build looks like should start here.
pub const A_WINNING_RUN: &str = "1-1J-1J-2-72W-1-2-2-0-2B-1H400-1D831-7441-10W15-13036-GM0C-B03D-M81G-1GR3J-740Q-1GG2R-11C4R-A0G3-HCH0-144H4-18M4-14RK9-1CGC-1K4MD-1JGGG-148GM-A0KN-158MR-KMGY-1HN00-16D11-KD20-FD41-6104-1AD34-17D08-16129-8X48-1813F-250H-1A15G-494M-2X0R-992R-16X5S-1FS0W-1FN4W-111G2-85J1-185M1-10NN1-18HG4-119J8-15XM8-85GC-HNJH-KHMG-5SGN-HSMN-BDGS-1E9KR-DJ00-X611-DA20-Y233-YE40-XP51-1KY18-WP48-YJ1C-YT2C-1JT0G-K61G-XY1H-1H23H-YA3M-XP4Q-XT5N-X62R-GY0X";

/// A run that lost nothing at all.
///
/// Fifty fights, fifty wins, and it is the only board in the project that never
/// gave a rung back - the owner's dropped one and the friend's two. Sixty-two
/// pieces and four titles.
///
/// Kept for the same reason as the other two, and for one more: the acceptance
/// gate the monster packer runs on reads difficulty off a single reference
/// board, because the owner's was the only one clearing far enough up the
/// ladder to give a reading at every rung. This one clears all of it.
pub const A_PERFECT_RUN: &str = "2-1J-1J-0-8G6-0-0-4-A-C-M-P-1Y-11400-12M22-ZG60-11MA0-ZM08-1MW49-7468-1GR8E-1180G-H84K-1306H-1180R-1PWAS-10G4Y-EWG0-134R0-1N0M4-1MP8-7RGC-150PC-78TD-A0GN-KMJN-1F4MQ-14MPM-7RPR-19N44-7D67-HN4B-1HNA8-6D0C-1T90E-N98D-8X4G-1NX0M-1X4R-8H8R-650W-19S2W-1T56W-1FSAW-1RHG2-3SJ0-1J1P0-FSGE-3SJC-JHRJ-F1MM-75GR-191JW-3NRX-DJ01-8A81-DY04-3244-DA84-1GYA4-1BT2H-X68H-4TAN-PT6T-KA6X";

pub fn export(run: &Run) -> String {
    let mut vals: Vec<u32> = vec![VERSION, run.rung as u32, run.wins, run.losses, run.gold.max(0) as u32];
    // Theme and classes by index, so the code carries no words of its own.
    vals.push(
        crate::theme::THEMES.iter().position(|t| t.id == run.theme.id).unwrap_or(0) as u32,
    );
    vals.push(run.extra_rows as u32);
    // What each board has *beyond* the uniform grant. Read off the boards
    // rather than tracked, so it cannot disagree with them.
    let per = run.slot_rows();
    for k in SlotKind::ALL {
        vals.push(per[k.index()].saturating_sub(run.extra_rows) as u32);
    }
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
    // Version 1 codes are still read. They were shared before the board could
    // be more than eight rows tall, so they carry no row count and pack `x`
    // into four bits and `y` into three - the other way round from now. Codes
    // people have already saved are not ours to invalidate.
    let version = next()?;
    if version == 0 || version > VERSION {
        return None;
    }
    let v1 = version == 1;
    let rung = next()? as usize;
    let wins = next()?;
    let losses = next()?;
    let gold = next()? as i32;
    let theme = crate::theme::THEMES
        .get(next()? as usize)
        .map(|t| t.id.to_string())
        .unwrap_or_else(|| "plain".into());
    let extra_rows = if v1 { 0 } else { next()? as u8 };
    let mut slot_rows = [0u8; 5];
    if version >= 3 {
        for k in SlotKind::ALL {
            slot_rows[k.index()] = next()? as u8;
        }
    }
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
        let (x, y) = if v1 {
            (((v >> 5) & 15) as u8, ((v >> 2) & 7) as u8)
        } else {
            (((v >> 6) & 7) as u8, ((v >> 2) & 15) as u8)
        };
        placed.push((
            (v >> 12) as usize,
            slot_of((v >> 9) & 7),
            x,
            y,
            (v & 3) as u8,
        ));
    }
    Some(Shared { rung, extra_rows, slot_rows, wins, losses, gold, theme, classes, placed })
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
        // A version 1 code still reads: it has one field fewer and packs its
        // coordinates the other way round, and codes people saved before the
        // boards could grow are not ours to invalidate.
        assert!(import("1-0-0-0-0-0-0-0").is_some(), "a version 1 code stopped reading");
        assert!(import("3-0-0-0-0-0-0-0-0").is_none(), "a version from the future");
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

//! A board, and the one way to rebuild one.
//!
//! **The reconstruction fault** (`CLAUDE.md` §6 trap 4) has been learned four
//! times in this repo, the last at M17 when a reference build came back as
//! zero items. A list of names is not a board: pieces have to be seated *and
//! locked as each item completes*, because a lock formed at the end describes
//! a different board from one formed as it was built. Everything here goes
//! through `rebuild`, and nothing else in this crate seats a piece.

use gearmaster_engine::loadout::{lock_assembled_in, ItemProfile, Loadout};
use gearmaster_engine::piece::{PieceRegistry, SlotKind, CATALOG};
use gearmaster_engine::stats::Stats;

/// A placement, in the tuple `share.rs` exports and `combat.rs` stores.
pub type Placement = (usize, SlotKind, u8, u8, u8);

/// A board as a search holds it: where every piece sits, and how the pieces
/// are cut into items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Board {
    pub gear: Vec<Placement>,
    /// How many of `gear`, in order, make up each item. Empty means "let the
    /// recipe decide", which is what a player's board does.
    pub chunks: Vec<usize>,
    /// Rows each grid has beyond the eight it starts with.
    pub rows: [u8; 5],
}

impl Board {
    /// A stable key over the placements, sorted, so two orders of the same
    /// board are one board.
    ///
    /// The same tuple `share.rs:218` packs, for the same reason: it is the
    /// whole of what a board is.
    pub fn key(&self) -> u64 {
        // `SlotKind` is not `Ord`, so the sort goes through its index - which
        // is the number the share code packs anyway.
        let mut sorted: Vec<(usize, usize, u8, u8, u8)> = self
            .gear
            .iter()
            .map(|&(d, s, x, y, r)| (d, s.index(), x, y, r))
            .collect();
        sorted.sort_unstable();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        };
        for (def, slot, x, y, rot) in sorted {
            eat(def as u64);
            eat(slot as u64);
            eat(x as u64);
            eat(y as u64);
            eat(rot as u64);
        }
        for r in self.rows {
            eat(r as u64 | 0x1_0000);
        }
        h
    }

    pub fn is_empty(&self) -> bool {
        self.gear.is_empty()
    }

    /// Seat every piece, locking each item as it completes.
    pub fn rebuild(&self) -> (PieceRegistry, Loadout) {
        let mut reg = PieceRegistry::new();
        let mut lo = Loadout::new();
        for k in SlotKind::ALL {
            lo.grow_one(k, self.rows[k.index()]);
        }
        for &(def, slot, x, y, rot) in &self.gear {
            if def >= CATALOG.len() {
                continue;
            }
            let id = reg.alloc(def);
            reg.set_rotation(id, rot);
            if lo.can_place(&reg, id, slot, x, y).is_ok() {
                lo.slot_mut(slot).place(&reg, id, x, y);
                // As it completes, not once at the end.
                lock_assembled_in(&mut lo, &reg, slot);
            }
        }
        (reg, lo)
    }

    /// What a fight runs on, from the player's side.
    pub fn profiles(&self) -> (Stats, Vec<ItemProfile>) {
        let (reg, lo) = self.rebuild();
        (lo.total_stats(&reg), lo.combat_items(&reg))
    }

    /// How many cells of the five grids are filled.
    pub fn cells(&self) -> usize {
        self.gear.iter().map(|&(d, ..)| CATALOG[d].cells.len()).sum()
    }

    /// The named form, for splicing into `combat.rs`.
    pub fn named(&self) -> Vec<(&'static str, SlotKind, u8, u8, u8)> {
        self.gear.iter().map(|&(d, s, x, y, r)| (CATALOG[d].name, s, x, y, r)).collect()
    }
}

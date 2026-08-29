//! What the two agents see, as numbers.
//!
//! Everything here is read off the `View`, which is the screen. The pools are
//! the point (Q1): a piece is described by **what it produces and what it
//! consumes**, and a board by the match between the two, because that is what
//! a build is and it is the thing THE APPRENTICE's objective was blind to.

use gearmaster_console::view::{BoardPools, Piece, View};
use gearmaster_console::{SlotKind, Verb};

use crate::brief::{Brief, BRIEF};

/// How many numbers describe a board.
pub const BOARD: usize = 28;
/// How many describe one candidate move.
pub const MOVE: usize = 26;
/// A state-action pair, which is what a Q network scores.
///
/// The brief rides on the state side: it is part of the situation, not part of
/// the move, because the same move is worth different amounts depending on
/// what was asked for. That is the whole claim Q8 measures.
pub const PAIR: usize = BOARD + BRIEF + MOVE;

fn s(v: i32, by: f32) -> f32 {
    (v as f32 / by).clamp(-4.0, 4.0)
}
fn s64(v: i64, by: f32) -> f32 {
    (v as f32 / by).clamp(-4.0, 4.0)
}

/// The board, the purse and what is coming.
pub fn board(v: &View) -> [f32; BOARD] {
    let mut f = [0.0f32; BOARD];
    let p: &BoardPools = &v.pools;
    // ---- the pool economy, which is the whole reason this exists ----
    f[0] = s(p.total_matched(), 12.0);
    f[1] = s(p.total_stranded(), 12.0);
    f[2] = s(p.total_starved(), 12.0);
    f[3] = p.flowing() as f32 / 4.0;
    for i in 0..4 {
        f[4 + i] = s(p.produces[i], 12.0);
        f[8 + i] = s(p.consumes[i], 12.0);
        f[12 + i] = s(p.matched[i], 12.0);
    }
    // ---- what the board is ----
    let items: usize =
        v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum();
    let filled: usize =
        v.grids.iter().map(|g| g.cells.iter().filter(|c| c.piece.is_some()).count()).sum();
    let cells: usize = v.grids.iter().map(|g| g.cells.len()).sum();
    f[16] = items as f32 / 8.0;
    f[17] = filled as f32 / cells.max(1) as f32;
    f[18] = s(v.stats.health, 800.0);
    f[19] = s(v.stats.strength, 60.0);
    f[20] = s64(v.figures.physical_dps + v.figures.magic_dps, 40_000.0);
    f[21] = s64(v.figures.armour_ps, 20_000.0);
    f[22] = s64(v.figures.flow, 20_000.0);
    f[23] = s(v.figures.curse_resist, 100.0);
    // ---- the purse, the tray, and what is coming ----
    f[24] = s(v.gold, 200.0);
    f[25] = v.tray.len() as f32 / v.tray_cap.max(1) as f32;
    f[26] = s(v.coming.stats.health, 4_000.0);
    f[27] = v.coming.brings.len() as f32 / 12.0;
    f
}

/// One candidate move, described so a network can compare two of them.
pub fn mv(v: &View, m: Verb) -> [f32; MOVE] {
    let mut f = [0.0f32; MOVE];
    // Which kind of move it is, one-hot over the shapes that matter.
    let kind = match m {
        Verb::Place { .. } => 0,
        Verb::Buy { .. } => 1,
        Verb::Sell { .. } => 2,
        Verb::Barter { .. } => 3,
        Verb::Reroll => 4,
        Verb::Rotate { .. } | Verb::RotateLocked { .. } => 5,
        Verb::Unequip { .. } | Verb::UnequipLocked { .. } => 6,
        Verb::ClearSlot { .. } | Verb::ClearAll => 7,
        _ => 8,
    };
    f[kind] = 1.0;

    // The piece the move is about, where there is one - and above all what it
    // does with the pools, and whether the board wants that.
    let piece: Option<&Piece> = match m {
        Verb::Place { piece, .. } | Verb::Sell { piece } | Verb::Rotate { piece } => {
            v.tray.iter().find(|p| p.id == Some(piece))
        }
        Verb::Buy { shelf } | Verb::Barter { shelf, .. } => {
            v.shop.iter().find(|s| s.index == shelf).map(|s| &s.piece)
        }
        _ => None,
    };
    if let Some(p) = piece {
        f[9] = p.cells as f32 / 8.0;
        f[10] = s(p.price, 60.0);
        f[11] = p.triggers.len() as f32 / 4.0;
        f[12] = p.pools.produces_any() as u8 as f32;
        f[13] = p.pools.consumes_any() as u8 as f32;
        f[14] = p.pools.self_feeding() as u8 as f32;
        f[15] = p.pools.conditional as f32 / 4.0;
        f[16] = s(p.stats.health, 200.0);
        f[17] = s(p.stats.physical_damage + p.stats.magic_damage, 40.0);
        f[18] = s(p.stats.armor, 40.0);
        // **Does this piece feed something the board already wants, or want
        // something the board already makes?** The two numbers a build is made
        // of, and neither is a property of the piece alone.
        let mut feeds = 0i32;
        let mut fed = 0i32;
        for i in 0..8 {
            if p.pools.produces[i] > 0 {
                feeds += v.pools.starved[i].min(p.pools.produces[i]);
            }
            if p.pools.consumes[i] > 0 {
                fed += v.pools.stranded[i].min(p.pools.consumes[i]);
            }
        }
        f[19] = s(feeds, 8.0);
        f[20] = s(fed, 8.0);
        f[21] = ((feeds > 0) || (fed > 0)) as u8 as f32;
    }

    // Where it is going, for a placement: which grid, how full it already is,
    // and how many neighbours the seat touches.
    if let Verb::Place { slot, x, y, .. } = m {
        f[22] = SlotKind::ALL.iter().position(|&k| k == slot).unwrap_or(0) as f32 / 5.0;
        if let Some(g) = v.grids.iter().find(|g| g.slot == slot) {
            let w = gearmaster_console::view::GRID_W as usize;
            let fill = g.cells.iter().filter(|c| c.piece.is_some()).count() as f32;
            f[23] = fill / g.cells.len().max(1) as f32;
            let mut near = 0.0;
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= g.rows as i32 {
                    continue;
                }
                if g.cells.get(ny as usize * w + nx as usize).is_some_and(|c| c.piece.is_some()) {
                    near += 1.0;
                }
            }
            f[24] = near / 4.0;
            f[25] = g.items.iter().filter(|i| !i.assembled).count() as f32 / 3.0;
        }
    }
    f
}

/// The board, plus what was asked for.
pub fn briefed(b: &[f32; BOARD], w: &Brief) -> [f32; BOARD + BRIEF] {
    let mut f = [0.0f32; BOARD + BRIEF];
    f[..BOARD].copy_from_slice(b);
    f[BOARD..].copy_from_slice(&w.0);
    f
}

/// The pair a Q network scores.
pub fn pair(b: &[f32; BOARD + BRIEF], m: &[f32; MOVE]) -> [f32; PAIR] {
    let mut out = [0.0f32; PAIR];
    out[..BOARD + BRIEF].copy_from_slice(b);
    out[BOARD + BRIEF..].copy_from_slice(m);
    out
}

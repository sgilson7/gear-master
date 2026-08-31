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
/// How many cells of one grid the layout carries.
///
/// Six wide by eight down, which is `SLOT_W` by the base `SLOT_H`. Grids grow
/// rows past eight and those rows are not in here - a growable board would need
/// a ragged encoding and the eight that every grid always has are the eight
/// that matter for a recipe.
pub const GRID_CELLS: usize = 6 * 8;

/// The five grids, cell by cell.
pub const LAYOUT: usize = 5 * GRID_CELLS;

/// Thirty numbers about what the board *is*, and then what it **looks like**.
///
/// **The layout was not in here at all.** A forty-eight-cell grid was one
/// number - the fraction of it that was full - so a board with its weapon grid
/// packed solid down the left and one with the same count scattered were the
/// same input. Packing is a spatial problem and the state had no spatial
/// representation of it: the move gained a column and a row, but the state
/// never said what was already there for the piece to sit against.
pub const BOARD: usize = 30 + LAYOUT;
/// The kinds a packing move comes in, in the order they are one-hot.
///
/// **Thirteen, and it was nine.** Four verbs shared the catch-all and none of
/// them carried a piece, so `Lock`, `Grow`, `Undo` and `Pin` were one identical
/// vector - and a network that cannot tell two actions apart cannot prefer one.
/// See the comment on the match in `mv` for what that cost.
const KINDS: usize = 13;
/// Where the band describing the *piece* a move is about starts.
const PIECE: usize = KINDS;
/// Where the band describing *where it goes* starts.
const WHERE: usize = PIECE + 13;
/// Where the band describing *what locking would fix* starts.
const LOCK: usize = WHERE + 10;
/// How many describe one candidate move.
pub const MOVE: usize = LOCK + 2;
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
    // ---- and where on the ladder this is ----
    //
    // **The rung was not in here at all.** Packing at rung three and packing at
    // rung seventeen are different problems - different creature, different
    // purse, different room for error - and the nearest thing this vector had
    // was the coming creature's health, which conflates "deep" with "tanky" and
    // says nothing about a rung-3 Wall against a rung-17 Striker.
    //
    // So every Q value a placement earned was earned without knowing where on
    // the road it was being made. It is the same fault the road agent had, in
    // the other trade.
    f[28] = v.rung_shown as f32 / 50.0;
    // How many lives are left, which in Rogue is how much room a board has to
    // be wrong. `None` in Grinder, where there is no such thing.
    f[29] = v.lives_left.unwrap_or(0) as f32 / gearmaster_console::ROGUE_LIVES as f32;
    // ---- and what the board looks like ----
    //
    // One column a cell, five grids of six by eight, in `SlotKind::ALL` order
    // so a grid is always in the same place. Occupied is one and empty is nought
    // - not *which* piece, because a piece reaches this vector as its properties
    // rather than its name everywhere else and a cell is no place to start
    // naming them.
    //
    // What this buys is the question a packer actually asks: is there room here,
    // and is that room next to the thing I am trying to finish.
    for (gi, slot) in SlotKind::ALL.iter().enumerate() {
        let Some(g) = v.grids.iter().find(|g| g.slot == *slot) else { continue };
        let w = gearmaster_console::view::GRID_W as usize;
        let base = 30 + gi * GRID_CELLS;
        for y in 0..8usize {
            for x in 0..w {
                if let Some(c) = g.cells.get(y * w + x) {
                    if c.piece.is_some() {
                        f[base + y * w + x] = 1.0;
                    }
                }
            }
        }
    }
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
        // **The four that used to share the catch-all.** Measured over forty
        // episodes: locking was on the menu for 6,198 decisions and scored
        // identically to pinning a shelf on **every one of them**, because
        // `Lock`, `Grow`, `Undo` and `Pin` were one bucket and none of them is
        // in the `piece` match below, so all four were the same vector.
        // `max_by` returns the last maximum and `Pin` is pushed onto the menu
        // after `Lock`, so the agent pressed that vector 1,653 times and got
        // `Pin` every time - by menu order rather than by policy. It never
        // locked once. `analysis/the-collapse.md` M1.
        Verb::Lock { .. } => 8,
        Verb::Grow { .. } => 9,
        Verb::Undo => 10,
        Verb::Pin { .. } => 11,
        // `PlaceLocked` is not here on purpose: `Console::menu` cannot
        // enumerate where a whole assembled item may be dropped without
        // lifting it first, so it is a verb the console accepts and the menu
        // never offers. It is not in the action space and needs no shape.
        _ => 12,
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
        f[PIECE + 0] = p.cells as f32 / 8.0;
        f[PIECE + 1] = s(p.price, 60.0);
        f[PIECE + 2] = p.triggers.len() as f32 / 4.0;
        f[PIECE + 3] = p.pools.produces_any() as u8 as f32;
        f[PIECE + 4] = p.pools.consumes_any() as u8 as f32;
        f[PIECE + 5] = p.pools.self_feeding() as u8 as f32;
        f[PIECE + 6] = p.pools.conditional as f32 / 4.0;
        f[PIECE + 7] = s(p.stats.health, 200.0);
        f[PIECE + 8] = s(p.stats.physical_damage + p.stats.magic_damage, 40.0);
        f[PIECE + 9] = s(p.stats.armor, 40.0);
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
        f[PIECE + 10] = s(feeds, 8.0);
        f[PIECE + 11] = s(fed, 8.0);
        f[PIECE + 12] = ((feeds > 0) || (fed > 0)) as u8 as f32;
    }

    // Where it is going, for a placement: which grid, how full it already is,
    // and how many neighbours the seat touches.
    if let Verb::Place { slot, x, y, .. } = m {
        f[WHERE + 0] = SlotKind::ALL.iter().position(|&k| k == slot).unwrap_or(0) as f32 / 5.0;
        // **Where the piece is going, which was not in here.** A move said which
        // grid, how full it was, how many neighbours the seat touched and
        // whether it completed a recipe - so the same piece at (3,2) and at
        // (4,2) described identically unless one of those four happened to
        // differ. Two seats with one neighbour each, both completing nothing,
        // were the same input.
        //
        // The column, the row, and how close the anchor sits to an edge -
        // because a shape against an edge has fewer ways to grow and the panel
        // draws exactly that.
        {
            let gw = gearmaster_console::view::GRID_W as f32;
            let rows = v.grids.iter().find(|g| g.slot == slot).map(|g| g.rows).unwrap_or(8) as f32;
            f[WHERE + 7] = x as f32 / gw.max(1.0);
            f[WHERE + 8] = y as f32 / rows.max(1.0);
            let dx = (x as f32).min(gw - 1.0 - x as f32);
            let dy = (y as f32).min(rows - 1.0 - y as f32);
            f[WHERE + 9] = dx.min(dy) / 3.0;
        }
        if let Some(g) = v.grids.iter().find(|g| g.slot == slot) {
            let w = gearmaster_console::view::GRID_W as usize;
            let fill = g.cells.iter().filter(|c| c.piece.is_some()).count() as f32;
            f[WHERE + 1] = fill / g.cells.len().max(1) as f32;
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
            f[WHERE + 2] = near / 4.0;
            f[WHERE + 3] = g.items.iter().filter(|i| !i.assembled).count() as f32 / 3.0;

            // **Does this placement finish something?**
            //
            // Q7 ended on the claim that the packer could not learn because a
            // move said *where* and *what* and never said whether the piece
            // completed a recipe with what was already seated - so the one
            // property that makes a placement good was not in the input, and
            // three training runs at three action spaces produced one curve.
            //
            // This is the missing number, and it is read off the panel: the
            // recipes are printed beside the grid (`recipe_tip`), and the
            // roles of the pieces in an unfinished group are printed in it.
            // A player has both. Nothing here consults the engine.
            if let Some(p) = piece {
                // Which groups this seat borders, by the neighbours' cells.
                //
                // **Diagonals count.** Orthogonal neighbours alone found 37 of
                // the 58 real completions; `slot::sets_touch_diagonally` is why,
                // and the panel draws groups the same way.
                //
                // And a piece is not one cell. `Place` names the anchor and
                // the piece occupies a `width` by `height` box from it, so the
                // seat borders whatever any of that box borders. The anchor
                // alone found 46 of 58.
                let mut ids: Vec<usize> = Vec::new();
                let box_cells: Vec<(i32, i32)> = (0..p.height as i32)
                    .flat_map(|dy| (0..p.width as i32).map(move |dx| (dx, dy)))
                    .collect();
                let around: Vec<(i32, i32)> = box_cells
                    .iter()
                    .flat_map(|&(cx, cy)| {
                        [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]
                            .into_iter()
                            .map(move |(dx, dy)| (cx + dx, cy + dy))
                    })
                    .collect();
                for (dx, dy) in around {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= g.rows as i32 {
                        continue;
                    }
                    if let Some(i) =
                        g.cells.get(ny as usize * w + nx as usize).and_then(|c| c.item)
                    {
                        if !ids.contains(&i) {
                            ids.push(i);
                        }
                    }
                }
                let mut touching: Vec<Vec<String>> = ids
                    .iter()
                    .filter_map(|&i| g.items.get(i))
                    .filter(|i| !i.assembled)
                    .map(|i| i.roles.clone())
                    .collect();
                if touching.is_empty() {
                    touching.push(Vec::new());
                }
                let mut best_missing = 9.0f32;
                let mut completes = 0.0f32;
                let mut wanted = 0.0f32;
                for r in &g.recipes {
                    if r.required.iter().any(|q| eq_role(q, &p.role))
                        || r.optional.iter().any(|q| eq_role(q, &p.role))
                    {
                        wanted = 1.0;
                    }
                    // **Only the group this seat would actually join.** The
                    // first version asked every unfinished group in the grid
                    // and fired on 218 placements of which 58 completed
                    // anything - recall 100%, precision 27%, because a piece
                    // in the top-left cannot finish an item in the bottom
                    // right and the feature did not know that. A piece joins
                    // what it touches; if it touches nothing it starts a new
                    // one.
                    for mut have in touching.clone() {
                        have.push(p.role.clone());
                        let missing = r
                            .required
                            .iter()
                            .filter(|q| !have.iter().any(|h| eq_role(q, h)))
                            .count() as f32;
                        if missing < best_missing {
                            best_missing = missing;
                        }
                        if missing == 0.0 {
                            completes = 1.0;
                        }
                    }
                }
                f[WHERE + 4] = wanted;
                f[WHERE + 5] = completes;
                f[WHERE + 6] = 1.0 - (best_missing.min(4.0) / 4.0);
            }
        }
    }

    // **What locking would fix, and where.**
    //
    // A lock is a decision about a particular item on a particular grid, and
    // the two things that decide whether it is worth pressing are how much
    // there is to lose and how crowded the grid already is - an unlocked item
    // negotiates with whatever it is touching, so an item alone on an empty
    // grid is in no danger and one of five pieces on a full grid is.
    //
    // The slot and the fill share their fields with a placement's, because
    // they mean the same thing for both and a network that has learned what a
    // crowded weapon grid looks like should not have to learn it twice.
    if let Verb::Lock { piece } = m {
        if let Some((g, item)) = v
            .grids
            .iter()
            .find_map(|g| g.items.iter().find(|i| i.pieces.contains(&piece)).map(|i| (g, i)))
        {
            f[WHERE] = SlotKind::ALL.iter().position(|&k| k == g.slot).unwrap_or(0) as f32 / 5.0;
            f[WHERE + 1] = g.cells.iter().filter(|c| c.piece.is_some()).count() as f32
                / g.cells.len().max(1) as f32;
            f[LOCK] = item.pieces.len() as f32 / 6.0;
            f[LOCK + 1] = g.items.iter().filter(|i| i.assembled).count() as f32 / 3.0;
        }
    }

    // A grow is about a grid too, and it had no shape at all.
    if let Verb::Grow { slot } = m {
        f[WHERE] = SlotKind::ALL.iter().position(|&k| k == slot).unwrap_or(0) as f32 / 5.0;
        if let Some(g) = v.grids.iter().find(|g| g.slot == slot) {
            f[WHERE + 1] = g.cells.iter().filter(|c| c.piece.is_some()).count() as f32
                / g.cells.len().max(1) as f32;
        }
    }
    f
}

/// Whether a recipe part and a piece's role are the same thing.
///
/// A recipe is printed as "Handle + 1-2 Damaging + 0-2 Accessory", so its
/// parts arrive carrying counts and capitals that a role does not.
fn eq_role(part: &str, role: &str) -> bool {
    let clean = |s: &str| {
        s.trim()
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == ' ')
            .trim_end_matches('s')
            .to_ascii_lowercase()
    };
    let (a, b) = (clean(part), clean(role));
    a == b || a.starts_with(&b) || b.starts_with(&a)
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

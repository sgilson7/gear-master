//! What a trial seat teaches.
//!
//! The hands try a seat, read the board, and take it back. That is a labelled
//! example and there are a hundred and fifty thousand of them in one run:
//! **(what the piece is, where it went, what the board was) → what it was
//! worth**. Nothing has ever been done with them.
//!
//! Which is the whole of why the pilot costs thirty-two seconds a seed. It
//! rediscovers, for every piece and every rung and every run, which seats are
//! worth trying - and a seat's worth is mostly a property of the piece and the
//! grid rather than of the run. That is a thing a model can hold and a search
//! cannot.
//!
//! ## Everything here is player-visible
//!
//! Each feature below is read off the `View`, which is the screen. The model
//! trained on these is a prior over what a person can see, not a lookup into
//! anything the pilot could not have read.

use gearmaster_console::view::{Piece, View};
use gearmaster_console::SlotKind;

/// How many numbers describe one candidate seat.
pub const FEATURES: usize = 24;

/// One trial seat, and what it was worth.
#[derive(Clone, Debug, PartialEq)]
pub struct Lesson {
    pub x: [f32; FEATURES],
    /// The change in the board's worth, as the pilot measures it.
    pub y: f32,
}

/// Describe a candidate seat as numbers.
///
/// Deliberately coarse and deliberately local: what kind of piece, how big,
/// what it carries, which grid, how full that grid already is, and where in it
/// the piece is going. A model over these can learn "a two-cell accessory in a
/// nearly-full weapon grid is worth trying and a five-cell base is not"
/// without knowing anything about the run it is in.
pub fn describe(v: &View, p: &Piece, slot: SlotKind, x: u8, y: u8) -> [f32; FEATURES] {
    let mut f = [0.0f32; FEATURES];
    // ---- the piece ---------------------------------------------------
    f[0] = p.cells as f32 / 8.0;
    f[1] = p.width as f32 / 6.0;
    f[2] = p.height as f32 / 6.0;
    f[3] = (p.price as f32 / 60.0).min(2.0);
    f[4] = p.triggers.len() as f32 / 4.0;
    f[5] = p.effect.is_some() as u8 as f32;
    f[6] = p.assembly_bonus.is_some() as u8 as f32;
    f[7] = (p.stats.health as f32 / 200.0).clamp(-2.0, 2.0);
    f[8] = (p.stats.physical_damage as f32 / 40.0).clamp(-2.0, 2.0);
    f[9] = (p.stats.magic_damage as f32 / 40.0).clamp(-2.0, 2.0);
    f[10] = (p.stats.armor as f32 / 40.0).clamp(-2.0, 2.0);
    f[11] = (p.stats.mana as f32 / 20.0).clamp(-2.0, 2.0);
    f[12] = (p.stats.strength as f32 / 20.0).clamp(-2.0, 2.0);
    // Does the card say any of it is handed over on every activation? That is
    // the difference between a rate and a quantity, and the interface draws it.
    f[13] = p.when.iter().any(|(w, _)| w == "onactivation") as u8 as f32;
    f[14] = p.when.iter().any(|(w, _)| w == "damage") as u8 as f32;

    // ---- the grid it is going into --------------------------------------
    if let Some(g) = v.grids.iter().find(|g| g.slot == slot) {
        let filled = g.cells.iter().filter(|c| c.piece.is_some()).count() as f32;
        f[15] = filled / g.cells.len().max(1) as f32;
        f[16] = g.items.iter().filter(|i| i.assembled).count() as f32 / 4.0;
        f[17] = g.items.iter().filter(|i| !i.assembled).count() as f32 / 4.0;
        f[18] = g.rows as f32 / 12.0;
        // How much of this grid touches the seat: neighbours are what make an
        // item, and a piece dropped in a corner touches less.
        let w = gearmaster_console::view::GRID_W as usize;
        let mut near = 0.0;
        for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= g.rows as i32 {
                continue;
            }
            let at = ny as usize * w + nx as usize;
            if g.cells.get(at).is_some_and(|c| c.piece.is_some()) {
                near += 1.0;
            }
        }
        f[19] = near / 4.0;
    }
    f[20] = x as f32 / 6.0;
    f[21] = y as f32 / 12.0;
    f[22] = slot as u8 as f32 / 5.0;
    // ---- the board as a whole -------------------------------------------
    f[23] = v.tray.len() as f32 / 12.0;
    f
}

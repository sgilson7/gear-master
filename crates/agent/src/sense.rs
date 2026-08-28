//! What a board looks like from the chair.
//!
//! The same arithmetic `gearmaster-oracle`'s S0 does, computed from the
//! `View` instead of from a `Loadout` - because every figure it needs is on a
//! screen a person looks at:
//!
//! * the six county figures, drawn on the county tab: what the board does *a
//!   second*, which is the question a river, a ford and a scarp are versions
//!   of;
//! * the character sheet;
//! * how many items assembled, drawn on every slot;
//! * and, since the card rewrite, **which of a piece's figures are rates and
//!   which are quantities** - `+2 nature` on a 2.8-second item against
//!   `+175 hp` - grouped by the interface itself.
//!
//! That last one is why this can be honest. Before the card said *when* each
//! figure happens, an agent scoring a board out of the screen would have had
//! to guess, and guessing wrong prices a rate as a quantity.
//!
//! The point of computing it here rather than importing it: the pilot may not
//! name the oracle. Two implementations of one idea is a thing that drifts, so
//! `crates/oracle/tests/s0.rs` and this module's own test hold them to the
//! same answer on the same board.

use gearmaster_console::view::View;

/// A board's worth, read off the screen.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Sense {
    /// Items that actually assembled. A loose piece pays its passive stats and
    /// never acts, and only weapons swing.
    pub items: usize,
    /// The six figures, in milli-units a second.
    pub flow: i64,
    pub damage_ps: i64,
    pub armour_ps: i64,
    pub fastest_ms: Option<u32>,
    pub curse_resist: i32,
    pub health: i32,
    pub strength: i32,
    /// Cells filled across the five grids, and cells there are.
    pub filled: usize,
    pub cells: usize,
}

/// How long a fight is assumed to last when turning a rate into a total.
///
/// Sudden death owns everything past thirty seconds, so a board is judged over
/// the window a fight is actually decided in.
pub const WINDOW_MS: i64 = 10_000;

impl Sense {
    pub fn of(v: &View) -> Sense {
        let items = v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum();
        let filled = v
            .grids
            .iter()
            .map(|g| g.cells.iter().filter(|c| c.piece.is_some()).count())
            .sum();
        Sense {
            items,
            flow: v.figures.flow,
            damage_ps: v.figures.physical_dps + v.figures.magic_dps,
            armour_ps: v.figures.armour_ps,
            fastest_ms: v.figures.fastest_ms,
            curse_resist: v.figures.curse_resist,
            health: v.stats.health,
            strength: v.stats.strength,
            filled,
            cells: v.grids.iter().map(|g| g.cells.len()).sum(),
        }
    }

    /// One number, for choosing between two boards.
    ///
    /// Deliberately crude, and deliberately **not** a prediction of a fight:
    /// its job is to tell a better seat from a worse one, several hundred
    /// times a rung. An agent that could predict the fight would not need to
    /// have it.
    pub fn worth(&self) -> i64 {
        let over_the_window = |per_second: i64| per_second * WINDOW_MS / 1_000_000;
        // An item is worth a great deal more than a cell: a board of loose
        // pieces pays flat stats and never acts, and the ladder is not clear
        // by flat stats. Weighted so that finishing an item always beats
        // filling four cells with something that does not.
        self.items as i64 * 400
            + over_the_window(self.damage_ps) * 4
            + over_the_window(self.armour_ps)
            + over_the_window(self.flow)
            + self.health as i64
            + self.strength as i64 * 8
            + self.curse_resist as i64 * 2
            + self.filled as i64
    }

    /// Nothing assembled: nothing acts, and a board that does not act does not
    /// win a fight it is not already winning.
    pub fn inert(&self) -> bool {
        self.items == 0
    }
}

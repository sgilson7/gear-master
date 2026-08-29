//! Standing a run at a rung by **walking** there rather than skipping to it.
//!
//! `qpack` has always used `Run::skip_to`, and its own comment says what that
//! costs: *"`skip_to` pays the bounties and leaves the tray holding a handle
//! and a blade"*. The bounties arrive, the shops do not. A run stood at rung
//! twenty-five that way has 1,516 gold, seven pieces, one shelf and **no
//! assembled item at all** - which is not a hard packing problem, it is an
//! impossible one, and a reward that is always the same number teaches nothing.
//!
//! That was survivable while the reward was one fight against one creature.
//! It is not survivable for `scoring`, whose whole question is what a board
//! does across a window of rungs, because there is no board.
//!
//! And it is the mission's own §3.1: *"walking the row instead means the shop
//! economy is real - the tray at rung 20 is whatever the run bought on the way,
//! which is the thing the current curriculum papers over"*. Rogue is what makes
//! it load-bearing rather than merely truer: a Rogue run cannot farm gold back,
//! so what it is carrying at rung twenty is what four lives could afford.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode};

/// How a walk ended.
#[derive(Clone, Debug)]
pub struct Walked {
    /// The rung the run is standing at, shown the way the screen shows it.
    pub rung: usize,
    /// Whether it got where it was sent.
    pub arrived: bool,
    /// Whether a Rogue run ran out of lives on the way.
    pub died: bool,
    /// Presses spent getting there.
    pub steps: usize,
}

/// What the pilot is allowed to spend getting there.
///
/// The pilot's own default is 600,000 presses for a whole run; a walk that
/// stops at a rung wants a fraction of that, and a curriculum wants many walks.
const BUDGET: usize = 400_000;

/// Walk a fresh run up to `rung` **the way the control walks**.
///
/// The first version of this pressed the first road verb and packed once a
/// rung, and it was not the control: the pilot barters, sells, rerolls, grows
/// and rearranges after a defeat. Measured (`--bin qrogue`), to rung 28 in
/// Grinder against that walker's 13 - and in Rogue to rung **18** against its
/// **1**, which is the difference between a curriculum and nothing.
///
/// Returns the console wherever it got to, so a caller can judge the board a
/// run **actually had** at that rung rather than one a bounty conjured. Check
/// `Walked::arrived` first: a run that died at rung four is not evidence about
/// rung twenty.
pub fn walk_to(seed: u64, mode: Mode, difficulty: Difficulty, rung: usize) -> (Console, Walked) {
    // `coverage: 0.0` because a curriculum wants the ordinary road rather than
    // the unusual branch - what is being built is the economy a run has at a
    // rung, not a tour of the doors.
    let d = Doctrine { patience: 12, budget: BUDGET, coverage: 0.0 };
    let (c, ended) = pilot::play_to(seed, mode, difficulty, d, rung);
    let at = c.view().rung_shown;
    let out = Walked {
        rung: at,
        arrived: at > rung,
        died: c.view().wiped || ended.why == "the run ended",
        steps: ended.presses,
    };
    (c, out)
}

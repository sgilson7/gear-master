//! What a board is worth, and why that differs by mode.
//!
//! ## The thing that has to be said first
//!
//! **Passing `Mode::Rogue` where `qpack` passes `Mode::Grinder` produces the
//! same packer.** The packer's reward is a fight, and `crates/engine/src/combat.rs`
//! never names `Mode`: a fight is a pure function of two boards. So the mode
//! cannot reach the reward, and no amount of training against a Rogue run
//! changes a single thing the network learns.
//!
//! That is not a bug anywhere. It is what makes a Rogue quartermaster a
//! **reward** question rather than a flag.
//!
//! ## What actually differs
//!
//! In Grinder the question is *does this board win often enough*. A loss costs
//! a rung and still pays the bounty, so a run can farm the fight below the one
//! that beat it until it can take the one that did.
//!
//! In Rogue the question is *can this board be surprised*. Combat is
//! deterministic, so the risk is not a dice roll - it is that the screen shows
//! you the **next** creature and nothing beyond it (`console::view`'s own note
//! about `Coming`). A board tuned exactly to the rung in front of it that falls
//! over on the one after costs a quarter of the run, and there is no farming it
//! back. Four such surprises and everything is gone.
//!
//! So a board is judged against a **window** of the rungs ahead rather than
//! against the one it is standing at, and the two modes read the window
//! differently: Grinder averages it, Rogue averages it and then pays for the
//! worst thing in it. The window is the honest form of "what a player cannot
//! see yet".
//!
//! This lives in `lab` rather than in the trainer because a reward that decides
//! what an agent becomes should be checkable without training one -
//! `crates/lab/tests/scoring.rs` is two hand-built boards and the claim that the
//! two judges rank them in opposite orders.

use gearmaster_engine::combat::{simulate_at, MonsterSpec, Outcome, LADDER, SUDDEN_DEATH_MS};
use gearmaster_engine::loadout::ItemProfile;
use gearmaster_engine::stats::Stats;

/// How many rungs ahead a board is judged against, itself included.
///
/// Three. One is the reward as it was, which prices a board against the only
/// creature it can see and is what the two modes had in common. Much more than
/// three and every board at rung two is being asked to beat rung ten, which is
/// a reward that is always the same number - the fault the curriculum comment
/// in `qpack` was written about.
pub const WINDOW: usize = 3;

/// How far ahead `reach` will walk before it stops counting.
///
/// Ten. The walk stops at the first loss anyway, so this only binds on a board
/// that is beating everything - and a board that clears ten consecutive rungs
/// from where it stands has answered the question. It is also the cost control:
/// `reach` simulates a fight per rung, so an uncapped walk on a strong board is
/// forty fights an episode where the window is three.
pub const REACH_CAP: usize = 10;

/// How steeply the depth reward grows. Two is a square, three a cube.
pub const DEPTH_POW: f32 = 2.0;

/// What clearing the whole ladder from rung nought would be worth.
///
/// The depth term is `((reached/50)^p - (from/50)^p) * DEPTH`, so it is zero for
/// a board that clears nothing and grows faster the deeper the board already
/// is. Three rungs cleared from rung 4 is worth 0.13; the same three from rung
/// 40 is worth 1.0, near eight times as much.
pub const DEPTH: f32 = 10.0;

/// How many consecutive rungs this board clears from `rung`, up to `REACH_CAP`.
///
/// **This is the Rogue question, asked directly.** A Rogue run dies on a loss,
/// so how far a board gets before it loses one is not a proxy for anything - it
/// is the thing. `repack.rs` has walked the ladder this way since THE
/// APPRENTICE to say how far a packed board got; this is the same walk, started
/// where the run is standing and stopped where the run would stop.
pub fn reach(stats: Stats, items: &[ItemProfile], rung: usize) -> usize {
    let mut cleared = 0;
    for i in 0..REACH_CAP {
        let at = rung + i;
        if at >= LADDER.len() {
            break;
        }
        if one_fight(stats, items, &LADDER[at]) <= 0.0 {
            break;
        }
        cleared += 1;
    }
    cleared
}

/// What getting from `rung` to `rung + cleared` is worth, growing with depth.
///
/// Zero for a board that clears nothing, so an empty board and a board that
/// loses where it stands are separated by the fight score rather than by this -
/// and there is no floor to drift out of range, which is the fault that had a
/// trained packer pressing `ClearAll` two hundred times.
pub fn depth_gain(rung: usize, cleared: usize) -> f32 {
    let l = LADDER.len() as f32;
    let to = ((rung + cleared) as f32 / l).powf(DEPTH_POW);
    let from = (rung as f32 / l).powf(DEPTH_POW);
    (to - from) * DEPTH
}

/// How much the worst rung in the window counts, over and above the mean.
///
/// A Rogue board that beats two of three and dies on the third has not "scored
/// two out of three" - it has ended the run on the third. The coefficient is
/// what turns an average into a judgement about the worst case, and it is a
/// dial rather than a truth: `--bin qjudge` is what moves it.
pub const DREAD: f32 = 1.0;

/// Which question is being asked of a board.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Judge {
    /// Does this win often enough? A loss costs a rung and pays a bounty.
    Grinder,
    /// Can this be surprised? A loss costs a quarter of everything.
    Rogue,
}

impl Judge {
    pub fn of_mode(m: gearmaster_console::Mode) -> Judge {
        match m {
            gearmaster_console::Mode::Grinder => Judge::Grinder,
            gearmaster_console::Mode::Rogue => Judge::Rogue,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Judge::Grinder => "grinder",
            Judge::Rogue => "rogue",
        }
    }
}

/// What one fight was worth: `+1` and up for a win, negative for a loss, and
/// the closer the loss came the less it costs.
///
/// Unchanged from the reward this replaces - a win is worth more the faster and
/// the more decisively it is won, and a loss is worth more the closer it came,
/// which is the gradient without which every losing board scores the same.
pub fn one_fight(stats: Stats, items: &[ItemProfile], spec: &MonsterSpec) -> f32 {
    let log = simulate_at(stats, items, spec, gearmaster_console::Difficulty::Medium);
    let enemy_max = spec.health.max(1) as f32;
    if log.outcome == Outcome::Victory {
        let quick = 1.0 - (log.duration_ms as f32 / SUDDEN_DEATH_MS as f32).min(1.0);
        let decided = if log.duration_ms < SUDDEN_DEATH_MS { 0.3 } else { 0.0 };
        1.0 + quick * 0.5 + decided
    } else {
        let left = log.enemy().health.max(0) as f32 / enemy_max;
        -1.0 + (1.0 - left) * 0.8
    }
}

/// Every rung in the window from `rung`, scored.
pub fn window(stats: Stats, items: &[ItemProfile], rung: usize) -> Vec<f32> {
    (0..WINDOW)
        .map(|i| {
            let at = (rung + i).min(LADDER.len() - 1);
            one_fight(stats, items, &LADDER[at])
        })
        .collect()
}

/// What a board is worth to a run in this mode, standing at this rung.
///
/// **There is no sentinel for an empty board and there must not be one.**
///
/// There was: `if items.is_empty() { return -1.5 }`, a number calibrated when
/// the reward was a single fight and the range was `[-1.0, +2.3]`, so `-1.5`
/// sat strictly below anything a real board could score. `Judge::Rogue`
/// subtracts the worst rung in the window and takes the range down to about
/// `-2.0`, and the constant was never re-checked against it.
///
/// So a board that lost its window scored `-1.78` and an empty board scored
/// `-1.5`, and **owning nothing was worth a quarter of a point over owning
/// something mediocre**. `ClearAll` is free, always legal and takes a board
/// there in one press. The trained packer found it: 206 `clear` presses in a
/// 262-key run, sixteen buys, fifteen sells and **not one placement**. It had
/// learned the reward exactly.
///
/// An empty board loses every fight in the window on its own merits, so
/// simulating it gives the true floor and the ordering stays monotone in board
/// quality. A constant standing in for a measurement is a constant that goes
/// stale the next time the measurement's range moves, and this one did.
pub fn score(stats: Stats, items: &[ItemProfile], rung: usize, judge: Judge) -> f32 {
    match judge {
        // Grinder is not what this mission spends its machine on: a loss costs
        // a rung and still pays a bounty, so a run farms past anything it can
        // eventually beat. The window mean is left as it was.
        Judge::Grinder => {
            let each = window(stats, items, rung);
            each.iter().sum::<f32>() / each.len() as f32
        }
        // **How far this board gets before it dies, and each rung dearer than
        // the last.** A Rogue run ends on a loss, so consecutive rungs cleared
        // is the objective rather than a proxy for it - and squaring the depth
        // means reaching a rung nothing has reached leaves a trace far larger
        // than the one before it, which is what a value function can be
        // followed back along.
        //
        // The fight where the run is standing is still scored, so a board that
        // clears nothing is graded on how close it came - the gradient A6 found
        // missing, without which every losing board scores the same.
        Judge::Rogue => {
            let here = one_fight(stats, items, &LADDER[rung.min(LADDER.len() - 1)]);
            here + depth_gain(rung, reach(stats, items, rung))
        }
    }
}

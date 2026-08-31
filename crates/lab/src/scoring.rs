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
    let each = window(stats, items, rung);
    let mean = each.iter().sum::<f32>() / each.len() as f32;
    match judge {
        Judge::Grinder => mean,
        // The worst thing in the window, weighted - because in Rogue the worst
        // thing in the window is the thing that happens.
        Judge::Rogue => {
            let worst = each.iter().cloned().fold(f32::INFINITY, f32::min);
            mean + DREAD * worst.min(0.0)
        }
    }
}

//! The row: an episode is a **run**, from rung one until it dies.
//!
//! `qpack`'s episode was one packing at one rung, drawn from a pool of
//! situations the *pilot* had walked to and then swept clean. So the packer
//! never started at the beginning, never played a run, and never met the
//! consequence of its own board on the next rung - the pilot had already walked
//! that road and the packer was dropped into a snapshot of it. Its evaluation
//! counted how many of twenty such puzzles produced a board scoring above zero,
//! which is packing quality and not depth.
//!
//! This is the mission's own §3.1, written in `design/HANDOFF-two-agents.md` and
//! never built: *"Not the current curriculum, which draws a rung and stands a
//! run there with `skip_to`. Every fight in the row... because that is how a run
//! meets them, and because a board that clears rung 12 and dies at 13 is a
//! different lesson from a board that was dropped at 13 cold."*
//!
//! ## What an episode is now
//!
//! Start a fresh Rogue run at rung one. Pack, fight, advance; pack, fight,
//! advance; until the run runs out of lives. The road between the packings is
//! driven programmatically - doors answered, gates walked past - because this
//! is the **packer's** episode and the road agent is not in it.
//!
//! Three things follow that the pool could never give:
//!
//! * **The economy is the agent's own.** What the tray holds at rung twenty is
//!   what this packer bought on the way there, out of gold its own boards won.
//! * **Depth is the score.** How far the run got is not a proxy for how good the
//!   boards were; it is the thing, and in Rogue it is the whole of the thing.
//! * **The curriculum is not bottom-heavy by accident.** The pool sat at rungs
//!   [1, 1, 3, 3, 5, 19, 24] because a walk only arrived if the *pilot* got that
//!   deep. Here the agent goes as far as its own boards carry it.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_trades::env::{Move, Packing};

/// The most rungs one run may be asked to walk.
///
/// Fifty-one is the whole ladder and the rung past Francis. A run that gets
/// there has answered every question this harness can ask.
const CEILING: usize = 51;

/// The most road presses spent between two packings before the walk gives up.
///
/// A door that will not answer, a gate that will not open: something is wrong
/// and the episode should end rather than spin. Bounded because a walk that
/// runs until it runs out is a hang (`CLAUDE.md` trap 24).
const ROAD_PRESSES: usize = 40;

/// What one run did.
#[derive(Clone, Debug, Default)]
pub struct Ran {
    /// The deepest rung it stood on, shown the way the screen shows it.
    pub deepest: usize,
    /// Fights won and lost.
    pub wins: u32,
    pub losses: u32,
    /// Packings asked for.
    pub packs: usize,
    /// Whether it ran out of lives rather than the ceiling or the budget.
    pub died: bool,
}

/// Walk the road between two fights, without touching the board.
///
/// Answers whatever is asking, takes the first open choice, walks past gates,
/// throws a lever, drinks a fountain. **Not a policy** - it is the road being
/// got out of the way so the packer's episode is about packing. When the road
/// agent joins this loop it takes this over.
fn walk_on(c: &mut Console) {
    for _ in 0..ROAD_PRESSES {
        let v = c.view();
        if v.question.is_some() {
            // The first choice that is open. A door left standing blocks
            // everything under it.
            let pick = v.question.as_ref().and_then(|q| q.choices.iter().find(|ch| ch.open));
            let Some(ch) = pick else { break };
            if !c.apply(Verb::Answer { choice: ch.index }).ok {
                break;
            }
            continue;
        }
        if v.town.is_some() {
            // Walk past. A town is one action and spending it is a decision
            // this loop is not qualified to make.
            if !c.apply(Verb::WalkOn).ok {
                break;
            }
            continue;
        }
        if v.fountain.is_some() {
            if !c.apply(Verb::Drink).ok {
                break;
            }
            continue;
        }
        if let Some(p) = v.points.as_ref() {
            let _ = p;
            if !c.apply(Verb::ThrowPoints { exit: 0 }).ok {
                break;
            }
            continue;
        }
        if v.in_dungeon {
            // Out, rather than down. A dungeon is a detour and this episode is
            // about the ladder.
            if !c.apply(Verb::Leave).ok {
                break;
            }
            continue;
        }
        break;
    }
}

/// One run, packed by `pack` at every rung, walked until it dies.
///
/// `pack` is handed in so the caller decides which packer is playing - the same
/// parameter the two-agent split turns on everywhere else.
pub fn run(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    pack: &mut dyn FnMut(&mut Console),
) -> (Console, Ran) {
    let mut c = Console::start(seed, mode, difficulty);
    let mut out = Ran { deepest: 1, ..Ran::default() };

    for _ in 0..CEILING * 4 {
        let v = c.view();
        out.deepest = out.deepest.max(v.rung_shown);
        if v.rung_shown > CEILING || c.over() {
            break;
        }
        // A wipe is the end of *this* run, whatever the engine does next.
        if v.wiped {
            out.died = true;
            break;
        }
        walk_on(&mut c);
        pack(&mut c);
        out.packs += 1;
        let before = c.view();
        let fight = if before.brawl_waiting { Verb::FightParty } else { Verb::Fight };
        if !c.menu().contains(&fight) || !c.apply(fight).ok {
            // Nothing to fight and nothing in the way: the road has run out.
            break;
        }
        let after = c.view();
        if after.rung_shown > before.rung_shown {
            out.wins += 1;
        } else {
            out.losses += 1;
        }
    }
    out.deepest = out.deepest.max(c.view().rung_shown);
    (c, out)
}

/// How steeply a run's worth grows with the rung it reached.
///
/// Squared, which is the owner's shape: reaching a rung nothing has reached is
/// worth more than the rung before it, so a value function has a trace to
/// follow back to whatever made the new depth possible.
pub const POW: f32 = 2.0;

/// What one rung squared is worth.
///
/// **Not `(rung/50)^2` scaled to ten.** That was the first version and it is
/// the wrong normalisation for an agent that starts at the bottom: it puts the
/// whole of the curve's growth in the top half of a ladder the agent has never
/// seen, so rung 4 was worth 0.064 and rung 5 was worth 0.100 while a run's
/// step charges came to six. The thing the episode was about was one percent of
/// what it was charged for, and the charge grew with depth because a deeper run
/// is more packings.
///
/// A twenty-fifth of the square bites where the agent actually lives - rung 5
/// is 1.0, rung 10 is 4, rung 20 is 16 - and still grows fast enough that rung
/// 47 is 88. The Huber knee in the trainer covers it.
pub const RUNG: f32 = 1.0 / 25.0;

/// What one spent life costs.
///
/// A Rogue run has four, so a run that reached rung ten on its last is worth
/// less than one that reached it on its first.
pub const LIFE: f32 = 0.5;

/// What a run was worth, growing with the depth it reached.
pub fn worth(ran: &Ran) -> f32 {
    let d = ran.deepest as f32;
    d.powf(POW) * RUNG - ran.losses as f32 * LIFE
}

/// The packing an episode does at one rung, as a closure over a policy.
///
/// Separate from `run` so the same walk can be driven by the written control,
/// by a learned net, or by a trainer that is recording what it pressed.
pub fn pack_with(
    c: &mut Console,
    budget: usize,
    mut choose: impl FnMut(&Console, &[Move]) -> usize,
) {
    let mut e = Packing::new(budget);
    loop {
        let ms: Vec<Move> = e
            .moves(c)
            .into_iter()
            .filter(|m| !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. })))
            .collect();
        if ms.is_empty() {
            break;
        }
        let at = choose(c, &ms);
        e.step(c, ms[at.min(ms.len() - 1)]);
        if e.finished {
            break;
        }
    }
}

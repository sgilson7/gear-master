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
    /// Every key the run pressed that stuck, in order.
    ///
    /// **Verbs rather than lines.** A `Verb` is `Copy`, so a run's tape is one
    /// `Vec` and no formatting; the same tape as strings would be two hundred
    /// `format!`s an episode, four thousand times a training run, to serve the
    /// one episode in twenty-five that anybody looks at. A proof is written out
    /// of this when one is wanted, and not before.
    ///
    /// This is a proof in the making: `(seed, mode, difficulty, [verb])` is all
    /// a proof is, and the other three are the arguments to `run`.
    pub tape: Vec<Verb>,
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
fn walk_on(c: &mut Console, tape: &mut Vec<Verb>) {
    // Only what stuck. A refused key is not something a person could press
    // again to the same effect, and a transcript carrying one does not replay.
    let press = |c: &mut Console, v: Verb, tape: &mut Vec<Verb>| {
        let ok = c.apply(v).ok;
        if ok {
            tape.push(v);
        }
        ok
    };
    for _ in 0..ROAD_PRESSES {
        let v = c.view();
        if v.question.is_some() {
            // The first choice that is open. A door left standing blocks
            // everything under it.
            let pick = v.question.as_ref().and_then(|q| q.choices.iter().find(|ch| ch.open));
            let Some(ch) = pick else { break };
            if !press(c, Verb::Answer { choice: ch.index }, tape) {
                break;
            }
            continue;
        }
        if v.town.is_some() {
            // Walk past. A town is one action and spending it is a decision
            // this loop is not qualified to make.
            if !press(c, Verb::WalkOn, tape) {
                break;
            }
            continue;
        }
        if v.fountain.is_some() {
            if !press(c, Verb::Drink, tape) {
                break;
            }
            continue;
        }
        if let Some(p) = v.points.as_ref() {
            let _ = p;
            if !press(c, Verb::ThrowPoints { exit: 0 }, tape) {
                break;
            }
            continue;
        }
        if v.in_dungeon {
            // Out, rather than down. A dungeon is a detour and this episode is
            // about the ladder.
            if !press(c, Verb::Leave, tape) {
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
/// **`pack` returns what it pressed**, because the thing that presses is the
/// only thing that knows. The road's keys and the fight's are pressed in here
/// and taped in here; the packing's are pressed by a closure the caller owns,
/// and a tape assembled anywhere else would have to guess the order.
///
/// A caller that does not care returns an empty vector and gets a tape with the
/// road and the fights in it, which is not a proof of anything - a transcript
/// missing the packing replays into a different board. `keys` turns
/// `pack_with`'s own record into the vector this wants.
pub fn run(
    seed: u64,
    mode: Mode,
    difficulty: Difficulty,
    pack: &mut dyn FnMut(&mut Console) -> Vec<Verb>,
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
        walk_on(&mut c, &mut out.tape);
        let pressed = pack(&mut c);
        out.tape.extend(pressed);
        out.packs += 1;
        let before = c.view();
        let fight = if before.brawl_waiting { Verb::FightParty } else { Verb::Fight };
        if !c.menu().contains(&fight) || !c.apply(fight).ok {
            // Nothing to fight and nothing in the way: the road has run out.
            break;
        }
        out.tape.push(fight);
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
/// **One. The square, undivided.**
///
/// It was `(rung/50)^2` scaled to ten, which put the whole of the curve's
/// growth in the top half of a ladder the agent had never seen. That was
/// replaced by a twenty-fifth of the square on the argument that it "bites
/// where the agent actually lives", and it was the same mistake one notch
/// along - four thousand episodes measured what it actually paid:
///
/// | rung reached | what the run was worth |
/// |---:|---:|
/// | 2 | **-1.84** |
/// | 5 | -1.00 |
/// | 7 | **-0.04** |
/// | 11 | +2.84 |
///
/// A Rogue run always ends with its four lives spent, so `LIFE` takes a flat
/// 2.0 off every episode - and against that, reaching **rung 7 paid less than
/// nothing**, and less than assembling a single item. `analysis/the-collapse.md`
/// M3 and M4: the agent learned to farm assemblies and ignore the ladder,
/// because that is what it was paid for. It spent 45% of its presses undoing
/// its own placements and its mean rung did not move in four thousand episodes
/// of a value function that was finally learning.
///
/// Undivided, a rung is worth far more than anything else on the board: rung 2
/// pays 2.0 where an item pays 1 to 3, and every rung after that widens the gap
/// - 3 pays 7, 7 pays 47. Clearing the next creature is the objective and now
/// it is priced like one.
///
/// **This moves the target range about twenty-five fold**, and a Huber knee has
/// to be sized to the range it is fitting or it either clips everything
/// (`CLAUDE.md` trap 53) or scales every gradient away (M2). `qrow` prints the
/// range against the knee every block; that column is the check on this
/// constant.
pub const RUNG: f32 = 1.0;

/// What finishing an item is worth at all, before its quality.
pub const ASSEMBLED: f32 = 1.0;

/// The most an item's quality can add on top.
pub const QUALITY: f32 = 2.0;

/// What one spent life costs.
///
/// A Rogue run has four, so a run that reached rung ten on its last is worth
/// less than one that reached it on its first.
pub const LIFE: f32 = 0.5;

/// What finishing an item is worth, once, on the press that finishes it.
///
/// **Depth is the objective and assembling is the means**, and a reward for the
/// end alone is a reward an agent cannot climb from the bottom: a run that
/// assembles its first item and still dies at rung three has done something
/// right and the depth term barely notices. So finishing an item pays on the
/// spot, and pays more for a better one.
///
/// Quality is read as the **change in what the board does a second** -
/// `Figures` is the game's own "what this board does in a second", drawn on the
/// county tab - so an item that adds damage, flow or armour is worth more than
/// one that adds nothing. No lookup and no table: the same numbers a player
/// sees, differenced across the press that assembled it.
///
/// It is capped, because an item that doubles the board's output is still one
/// item and the run's depth is what the episode is about.
pub fn assembly_bonus(before: &gearmaster_console::view::Figures, after: &gearmaster_console::view::Figures) -> f32 {
    let d = |a: i64, b: i64| (a - b).max(0) as f32;
    let gain = d(after.physical_dps + after.magic_dps, before.physical_dps + before.magic_dps)
        + d(after.armour_ps, before.armour_ps)
        + d(after.flow, before.flow);
    ASSEMBLED + (gain / 10_000.0).min(QUALITY)
}

/// What a run was worth, growing with the depth it reached.
pub fn worth(ran: &Ran) -> f32 {
    let d = ran.deepest as f32;
    d.powf(POW) * RUNG - ran.losses as f32 * LIFE
}

/// What one press did to the board, for a reward that pays per press.
#[derive(Copy, Clone, Debug)]
pub struct Pressed {
    pub before: gearmaster_console::view::Figures,
    pub after: gearmaster_console::view::Figures,
    pub items_after: usize,
    /// The key it was. `None` is `Move::Done` - a decision, and not a key.
    ///
    /// A `Verb` is `Copy`, so carrying it costs nothing and answers two
    /// questions at once: what belongs on a tape, and what a key histogram
    /// would count. The second was asked for in the collapse brief and there
    /// was no field that could answer it.
    pub verb: Option<Verb>,
    /// Whether the console took it.
    ///
    /// `Packing::step` documents `false` as a bug in the caller, and nothing
    /// anywhere noticed one. A refused press must stay off the tape, so this
    /// had to be looked at, and now it can be counted.
    pub stuck: bool,
}

/// The keys out of a packing, for a tape.
///
/// What stuck, in order, `Done` dropped. This is what `run`'s `pack` closure
/// hands back.
pub fn keys(pressed: &[Pressed]) -> Vec<Verb> {
    pressed.iter().filter(|p| p.stuck).filter_map(|p| p.verb).collect()
}

fn items_of(c: &Console) -> usize {
    c.view().grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum()
}

/// The packing an episode does at one rung, as a closure over a policy.
///
/// Separate from `run` so the same walk can be driven by the written control,
/// by a learned net, or by a trainer that is recording what it pressed.
///
/// Returns what each press did - the board's figures either side of it and how
/// many items stood afterwards - because a reward that pays for finishing an
/// item has to know **which press finished it**, and the chooser cannot see the
/// other side of its own decision.
pub fn pack_with(
    c: &mut Console,
    budget: usize,
    mut choose: impl FnMut(&Console, &[Move]) -> usize,
) -> Vec<Pressed> {
    let mut e = Packing::new(budget);
    let mut out = Vec::new();
    loop {
        let ms: Vec<Move> = e
            .moves(c)
            .into_iter()
            .filter(|m| !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. })))
            .collect();
        if ms.is_empty() {
            break;
        }
        let before = c.view().figures;
        let at = choose(c, &ms);
        let m = ms[at.min(ms.len() - 1)];
        let stuck = e.step(c, m);
        out.push(Pressed {
            before,
            after: c.view().figures,
            items_after: items_of(c),
            verb: match m {
                Move::Press(v) => Some(v),
                Move::Done => None,
            },
            stuck,
        });
        if e.finished {
            break;
        }
    }
    out
}

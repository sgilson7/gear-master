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
    /// Where each packing ended, as an index into `tape`.
    ///
    /// **A tape of keys is not a replayable episode on its own.** Every key in
    /// it is legal in several different packings, so a replay that does not
    /// know where one packing stopped will hand the next one somebody else's
    /// presses - and the keys keep matching while the board quietly diverges.
    /// Measured: a rung-20 tape followed 317 keys correctly and then asked to
    /// buy from a shelf it could no longer afford, because a fight four rungs
    /// earlier had been fought with the wrong board.
    pub pack_ends: Vec<usize>,
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
    run_from(Console::start(seed, mode, difficulty), pack)
}

/// The same loop, carried on from a run somebody else has already walked.
///
/// `Console` is `Clone`, so a probe can take a copy of a run standing at a rung
/// and ask what happens to it from there - which is the only way to ask a
/// **counterfactual** about one decision. `run` is this with a fresh console.
///
/// The `Ran` it hands back is about the stretch it walked and not about the
/// whole run: `deepest` is still the deepest rung *stood on*, so a probe that
/// starts at rung nine and clears three more reads twelve.
pub fn run_from(
    mut c: Console,
    pack: &mut dyn FnMut(&mut Console) -> Vec<Verb>,
) -> (Console, Ran) {
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
        out.pack_ends.push(out.tape.len());
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

/// The exponent actually used, which `QROW_POW` may override.
///
/// An experiment rather than a setting: squaring took the floor mean from 2.10
/// to 3.02 by making a rung worth more than anything else on the board, and the
/// question is whether cubing it - rung 2 pays 8, rung 3 pays 27 - buys more of
/// the same or simply outruns the loss. Read once, so a run cannot change its
/// own objective half way through.
///
/// **Whatever this is, the Huber knee has to be sized to it.** Cubed, a rung-13
/// run is worth 2,195 where squared it was 167, and a knee left at 5 would clip
/// every rung above the first (`CLAUDE.md` trap 53) while one large enough to
/// hold the tail scales every ordinary gradient away (`analysis/the-collapse.md`
/// M2). `qrow` prints the range against the knee every block.
pub fn pow() -> f32 {
    static P: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *P.get_or_init(|| {
        std::env::var("QROW_POW").ok().and_then(|v| v.parse().ok()).unwrap_or(POW)
    })
}

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

/// What the **n**th piece of taken-back work costs inside one packing.
///
/// The nth costs `n * CHURN`, so the first is nearly free and the twentieth is
/// twenty times as dear. Taking a piece out to reseat it is a legitimate move;
/// doing it twenty times is the thrash `analysis/the-collapse.md` M4 measured -
/// `undo` at 45.5% of every press, and still 24.7% after the reward was fixed,
/// against `place` at 44.7%. The packer seats a piece and takes the move back.
///
/// **Increasing, because a flat charge cannot tell those apart**, and `CLAUDE.md`
/// trap 44 is the record of trying: a no-op cost 0.01 against value estimates
/// spread over 1.70 and nothing happened, and taking the verb out of the action
/// space just moved the thrash to the next cheapest key - `Rotate`, then `Pin`,
/// then `Undo`.
///
/// **And per packing, not per run.** The last flat step charge punished the
/// objective: a deeper run is more packings, so it paid more for going further.
/// The count resets at every packing, so depth costs nothing and only churn
/// inside one visit does.
pub fn churn() -> f32 {
    static C: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *C.get_or_init(|| {
        std::env::var("QROW_CHURN").ok().and_then(|v| v.parse().ok()).unwrap_or(0.02)
    })
}

/// Whether a press takes work back rather than doing any.
fn takes_back(v: Option<Verb>) -> bool {
    matches!(
        v,
        Some(Verb::Unequip { .. })
            | Some(Verb::UnequipLocked { .. })
            | Some(Verb::Undo)
            | Some(Verb::ClearSlot { .. })
            | Some(Verb::ClearAll)
    )
}

/// What each press of one packing costs in churn, in the order it was pressed.
///
/// Hand it the presses of a **single** packing; the count is what resets.
pub fn churn_penalty(packing: &[Pressed]) -> Vec<f32> {
    let c = churn();
    let mut n = 0.0f32;
    packing
        .iter()
        .map(|p| {
            if p.stuck && takes_back(p.verb) {
                n += 1.0;
                -c * n
            } else {
                0.0
            }
        })
        .collect()
}

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
    d.powf(pow()) * RUNG - ran.losses as f32 * LIFE
}

/// A run's worth, paid where it was earned instead of all at the end.
///
/// **Return decomposition, and here it is exact rather than learned.**
///
/// `worth` is `deepest^pow - losses*LIFE` and it arrives on the very last press
/// of the episode. `analysis/the-action-gap.md` A.3 measures what that does: an
/// episode is about 350 decisions and at `gamma = 0.999` the terminal reward
/// reaches the first of them at 0.70 of its face value and the last at 1.00, so
/// three hundred and fifty state-action pairs receive the same number to within
/// thirty percent. There is no temporal structure in the credit at all, and
/// every transition but one has a reward of zero - so the target is `gamma *
/// max Q(s',.)` almost everywhere and the value has to walk backwards three
/// hundred steps by bootstrapping alone.
///
/// RUDDER's answer is to redistribute the episode's return onto the decisions
/// that predicted it, by *regressing* the return on the whole sequence. Here no
/// regression is needed, because the return telescopes in closed form:
///
/// ```text
///   D^2  =  sum over k of  (k^2 - (k-1)^2)
/// ```
///
/// So the depth term decomposes exactly into a payment made at the moment the
/// run first stood on each rung, and the life term into one made at the fight
/// that cost it. The sum is unchanged, which is what makes the redistributed
/// process **return-equivalent** and its optimal policy the same one.
///
/// What it buys is that the reward now *differs between transitions inside an
/// episode*, and differs in the direction of the thing being learned. What it
/// does not buy is credit at the *decision*: it moves the payment from the end
/// of the run to the end of the rung, and A.1 measures 78% of rungs as coming
/// out the same whatever was pressed at them. This is the affordable half.
///
/// `rungs[p]` is the rung standing when packing `p` began and `ended_at` is the
/// rung the run finished on. The residual - anything the telescoping does not
/// account for, such as a last packing whose fight never happened - is put on
/// the final packing, so the sum is `worth` by construction and a caller can
/// check it.
pub fn spread(rungs: &[usize], ended_at: usize, worth: f32) -> Vec<f32> {
    let mut pay = vec![0.0f32; rungs.len()];
    if rungs.is_empty() {
        return pay;
    }
    let p = pow();
    // The rung you start on is already stood upon, and paying for it is what
    // makes the telescoping sum to `deepest^pow` rather than to
    // `deepest^pow - 1`. A constant cannot change which policy is best; it is
    // here so the identity is exact and the check below means something.
    let mut most = rungs[0];
    pay[0] += (most as f32).powf(p) * RUNG;
    for i in 0..rungs.len() {
        let after = if i + 1 < rungs.len() { rungs[i + 1] } else { ended_at };
        let then = most.max(after);
        if then > most {
            pay[i] += ((then as f32).powf(p) - (most as f32).powf(p)) * RUNG;
            most = then;
        }
        // A fight that did not move the rung is a fight that was lost, which is
        // the same rule `run` counts `losses` by.
        if after <= rungs[i] {
            pay[i] -= LIFE;
        }
    }
    let sum: f32 = pay.iter().sum();
    if let Some(last) = pay.last_mut() {
        *last += worth - sum;
    }
    pay
}

/// What the board **is**, as one number.
///
/// Everything a packing decision can change and nothing it cannot: which piece
/// sits in which cell of which grid, which items are locked, what is in the
/// tray and what is in the purse. Two boards with the same fingerprint are the
/// same board, so a press that reproduces an earlier fingerprint put the board
/// back somewhere it had already been.
///
/// **Read off the `View`**, which is the screen, so this is a fact about what a
/// player would see rather than about engine internals. It is a hash and not an
/// equality test because a packing is forty presses and keeping forty whole
/// boards to compare against is the expensive way to ask a cheap question; a
/// 64-bit collision inside one packing is not a thing that will happen.
pub fn fingerprint(c: &Console) -> u64 {
    let v = c.view();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(0x100_0000_01b3);
    };
    for g in &v.grids {
        eat(g.slot as u64 + 1);
        eat(g.rows as u64);
        for cell in &g.cells {
            // The piece in it, and nought for an empty cell.
            eat(cell.piece.map(|p| p.0 as u64 + 1).unwrap_or(0));
        }
        // **Locked, because locking is a real change and must not read as a
        // no-op.** A lock alters nothing about where the pieces sit, so a
        // fingerprint over cells alone would charge for locking and then charge
        // again for unlocking, which is the opposite of what is wanted: one of
        // those two is a legitimate press.
        for item in &g.items {
            eat(item.locked as u64);
            for p in &item.pieces {
                eat(p.0 as u64 + 1);
            }
        }
    }
    for p in &v.tray {
        eat(p.id.map(|i| i.0 as u64 + 1).unwrap_or(0));
    }
    // **The shop, because `Buy`, `Sell`, `Reroll` and `Pin` are packing keys.**
    //
    // Left out of the first version, and the printed diagnostic said so at
    // once: 85.8% of presses read as revisits, because a reroll changes only
    // the shelves and a pin changes only a flag, so both looked like a board
    // that had gone nowhere. A reroll is a real decision that costs real gold,
    // and charging for it would have been a step charge on the one key that
    // buys new options.
    //
    // A shelf is **what is on it**, and deliberately not whether it is held.
    //
    // The first version hashed `shelf.pinned` too, on the reasoning that a pin
    // is a key a packing may press. That made every pin configuration a board
    // nobody had seen: six shelves is sixty-four of them, which is more novel
    // states than the forty-press budget can spend, and the agent found it -
    // `pin` went from 0.0% of choices to **22.7%** under the charge. A finer
    // fingerprint is not a better one; it invented a free-action generator out
    // of a flag that changes nothing the board does.
    //
    // A pin holds a shelf through a restock and alters nothing about what the
    // board is or what it can fight, so a pin press *is* a press that left the
    // board where it was, and charging it is correct. A reroll changes the
    // stock and is novel, which is also correct - it costs gold and buys new
    // options. Excluding the flag gets both right; including it got one wrong.
    for shelf in &v.shop {
        eat(shelf.index as u64);
        eat(shelf.piece.id.map(|i| i.0 as u64 + 1).unwrap_or(0));
        // A shelf the shop has restocked carries a different piece under the
        // same index; one that has not carries the same name.
        for b in shelf.piece.name.bytes() {
            eat(b as u64);
        }
    }
    eat(v.gold as u64);
    h
}

/// What the **n**th press that puts the board somewhere it has already been
/// costs, inside one packing.
///
/// **The general form of `CLAUDE.md` trap 44, which five relocations earned.**
/// `Rotate` was pressed 400 times in 420 and taken out of the action space;
/// then `Pin` 410 in 420 and given its own features; then `Undo` at 45.5%; then
/// `Lock` at 45% with half of it unlocking; and then, once locking and
/// unlocking could be told apart, `Undo` again at 31.0%. Two verb removals and
/// two feature fixes, and every one of them moved the thrash to whatever the
/// menu offered next.
///
/// The trap's own sentence is *charge for what the board does, not for what the
/// verb is called*, and this is that sentence: a press is a no-op if the board
/// it produces is one this packing has already produced, whatever key it was.
/// Place-then-undo, lock-then-unlock, a rotation and back, a three-press cycle
/// - all one rule, and none of them needs naming.
///
/// It does not charge a legitimate press. Locking an item changes the
/// fingerprint, so the first lock is free; it is the *unlock* that returns the
/// board to where it was, and only that one pays.
///
/// **Flat, and per packing**, and the flatness was chosen by the printed
/// figure rather than by argument. The first version charged the nth revisit
/// `n * REVISIT`, which is the shape `churn_penalty` uses and the right one for
/// a sparse event. Revisits are not sparse: measured, **80.3% of presses put
/// the board somewhere it had already been**, because in a
/// place-undo-place-undo cycle both halves revisit after the first two. An
/// increasing charge over that is quadratic in the length of the packing, and
/// at 0.005 a press it already came to -12.67 an episode against episode
/// returns of four to nine - which is the "step charge a hundred times the
/// objective" that `design/HANDOFF-the-collapse.md` lists as a known way to
/// kill a run.
///
/// The set resets at every packing, because a board legitimately passes through
/// the same state at two different rungs and a run that goes deeper must not
/// pay for going deeper.
pub fn revisit() -> f32 {
    static R: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *R.get_or_init(|| {
        std::env::var("QROW_REVISIT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.02)
    })
}

/// What each press of one packing costs for putting the board back somewhere it
/// has been, in the order it was pressed.
///
/// `start` is the fingerprint before the packing began, so a press that undoes
/// the whole packing pays like any other revisit.
pub fn revisit_penalty(packing: &[Pressed], start: u64) -> Vec<f32> {
    let c = revisit();
    let mut seen: Vec<u64> = vec![start];
    packing
        .iter()
        .map(|p| {
            if !p.stuck {
                return 0.0;
            }
            if seen.contains(&p.state) {
                -c
            } else {
                seen.push(p.state);
                0.0
            }
        })
        .collect()
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
    /// What the board was after this press. See `fingerprint`.
    pub state: u64,
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
            state: fingerprint(c),
        });
        if e.finished {
            break;
        }
    }
    out
}

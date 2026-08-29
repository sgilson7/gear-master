//! The packers a road agent can be handed, and the one that is the control.
//!
//! `Step::Pack` is one macro-action into a **frozen** packer, and which packer
//! is frozen is the parameter the whole two-agent split turns on. There are two
//! today and they are not close: the written control assembles seventeen items
//! and clears 48/50 rungs, and the learned one assembles 2.8
//! (`analysis/the-two-trades.md`, post-merge).
//!
//! ## Why a road agent's `Pack` has to buy
//!
//! `hands::pack` seats what is in the tray and buys nothing, because in the
//! pilot the shopping is a separate stretch of the same loop. A pathfinder
//! cannot do that stretch itself: `Buy` is the quartermaster's verb
//! (`partition.rs`) and offering it to the road agent would put the split back
//! where it was. So the macro-action is buy-and-seat, and both halves are the
//! pilot's own - `pilot::want_to_buy` and `hands::pack` - so that "the control"
//! keeps meaning the thing every benchmark was measured against.
//!
//! This is **not** §3.3's shopping list. That is a planner naming a piece it
//! needs for a door three rungs away, and it is C3's. This is the control's own
//! greedy shelf policy, which knows nothing about any chain.

use gearmaster_agent::{hands, pilot};
use gearmaster_console::{Console, Verb};

/// How many shelves one `Pack` may take.
///
/// The pilot allows twenty-four presses across a whole stretch; a road agent
/// gets one macro-action per decision and will get many of them, so a small
/// number here is a rate rather than a cap. Six is one shop's worth.
const SHELVES: usize = 6;

/// How many presses the written control gets.
///
/// **Not the learned packer's budget**, and handing it that was worth an
/// afternoon: `hands::pack` is greedy over pieces and *exhaustive over seats*,
/// so its cost is the number of anchors it tries, and Q0 measured it at a
/// median of 492 presses an episode with a ninetieth percentile of 798. Given
/// forty it bought four pieces, seated none of them, and lost rung one for ever
/// - which reads exactly like a policy that will not pack.
///
/// The learned packer's forty is a budget on *decisions*. This is a budget on
/// *presses*, and the two numbers are not the same kind of thing.
const CONTROL_PRESSES: usize = 2_000;

/// Buy what the shelf is worth and seat what the tray holds.
///
/// The written control, and the packer to freeze while the road policy is what
/// is being learned - `HANDOFF-two-agents.md` §C1: composing with the learned
/// packer first makes every failure ambiguous.
pub fn control(c: &mut Console, learned_budget: usize) {
    control_recording(c, learned_budget, &mut Vec::new());
}

/// The same, writing every key it presses where a transcript can find it.
pub fn control_recording(c: &mut Console, _learned_budget: usize, said: &mut Vec<String>) {
    // A row that has been granted and not spent is six cells nobody has. The
    // pilot presses this and the macro-action did not, which is one of the
    // reasons a walked curriculum stalled around rung ten where the pilot
    // clears forty-eight.
    while let Some(pick) = c.menu().iter().find(|v| matches!(v, Verb::Grow { .. })).copied() {
        let line = pick.line();
        if !c.apply(pick).ok {
            break;
        }
        said.push(line);
    }
    let before = c.view();
    let mut spent = 0;
    while spent < SHELVES {
        let v = c.view();
        let Some(shelf) = pilot::want_to_buy(&v) else { break };
        let pick = Verb::Buy { shelf };
        let line = pick.line();
        if !c.menu().contains(&pick) || !c.apply(pick).ok {
            break;
        }
        said.push(line);
        spent += 1;
    }
    // **Nothing bought and nothing loose is nothing to seat.** `hands::pack` is
    // exhaustive over anchors, so calling it on an empty tray costs a full
    // sweep of five grids to discover there was nothing to sweep. A road agent
    // presses `Pack` on perhaps half its decisions and most of them are like
    // this, which was five seconds an episode of finding out.
    if spent == 0 && before.tray.is_empty() {
        return;
    }
    hands::pack_saying(c, CONTROL_PRESSES, said);
    // **`hands::reseat` does not belong here, and it was measured.** The board
    // fills up and `pack` only ever adds, so rebuilding a full grid is the
    // obvious next move - and calling it whenever adding failed took a walked
    // Grinder curriculum from 46 seconds to **494** and from nine arrivals in
    // twenty to eight. Ten times the wall clock for slightly fewer boards.
    //
    // It is not that reseating is wrong. It is that it costs a full sweep of a
    // grid every time the tray has something that will not fit, which on a
    // walked run is most rungs, and the thing it buys is worth less than the
    // rungs the time could have walked instead.
}

/// A learned packer, or nothing at all.
///
/// With no checkpoint this seats nothing, which is a real control and a bad
/// packer: a road policy trained against it learns to walk with no board.
pub fn learned(net: Option<&gearmaster_trades::QNet>, c: &mut Console, budget: usize) {
    learned_recording(net, c, budget, &mut Vec::new());
}

/// The same, writing every key it presses where a transcript can find it.
pub fn learned_recording(
    net: Option<&gearmaster_trades::QNet>,
    c: &mut Console,
    budget: usize,
    said: &mut Vec<String>,
) {
    use gearmaster_trades::brief::Brief;
    use gearmaster_trades::env::{Move, Packing};
    use gearmaster_trades::feature;

    let Some(net) = net else { return };
    let mut e = Packing::new(budget);
    loop {
        // Rotations are not decisions - Q7 took them out of the action space
        // and the curve did not move, but the presses did.
        let ms: Vec<Move> = e
            .moves(c)
            .into_iter()
            .filter(|m| !matches!(m, Move::Press(Verb::Rotate { .. } | Verb::RotateLocked { .. })))
            .collect();
        if ms.is_empty() {
            break;
        }
        let v = c.view();
        let b = feature::briefed(&feature::board(&v), &Brief::NONE);
        let at = ms
            .iter()
            .map(|m| match m {
                Move::Press(verb) => net.q(&feature::pair(&b, &feature::mv(&v, *verb))),
                Move::Done => net.q(&feature::pair(&b, &[0.0; feature::MOVE])),
            })
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| i)
            .expect("the list is not empty");
        if let Move::Press(verb) = ms[at] {
            said.push(verb.line());
        }
        e.step(c, ms[at]);
        if e.finished {
            break;
        }
    }
}

/// The packer a name asks for.
///
/// `control` is the written one. Anything else is a path to a checkpoint, and
/// a path that does not load is nothing at all - which is said out loud rather
/// than falling back, because a benchmark that quietly ran the wrong packer is
/// a number about nothing.
pub enum Packer {
    Control,
    Learned(Option<gearmaster_trades::QNet>),
}

impl Packer {
    pub fn named(what: &str) -> Packer {
        match what {
            "control" | "" => Packer::Control,
            path => Packer::Learned(gearmaster_trades::QNet::load(path)),
        }
    }

    pub fn describe(&self, what: &str) -> String {
        match self {
            Packer::Control => "the written control - buys and seats".into(),
            Packer::Learned(Some(_)) => format!("learned, from {what}"),
            Packer::Learned(None) => {
                format!("NOTHING - {what} did not load, so the road walks on what it is given")
            }
        }
    }

    pub fn pack(&self, c: &mut Console, budget: usize) {
        self.pack_recording(c, budget, &mut Vec::new());
    }

    pub fn pack_recording(&self, c: &mut Console, budget: usize, said: &mut Vec<String>) {
        match self {
            Packer::Control => control_recording(c, budget, said),
            Packer::Learned(net) => learned_recording(net.as_ref(), c, budget, said),
        }
    }
}

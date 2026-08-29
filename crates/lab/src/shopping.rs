//! The shopping list, derived from a chain rather than typed.
//!
//! `design/HANDOFF-two-agents.md` §3.3: **the quartermaster does not learn to
//! buy rumours.** The pathfinder learns what the run needs; when it has
//! decided, the purchase is executed programmatically and the packer is handed
//! a tray that already contains it.
//!
//! That is not a convenience, it is the only way round a hole in the partition.
//! `Buy` and `Barter` are the quartermaster's verbs (`partition.rs`) and *why*
//! to barter for a word is a fact about a door seven rungs away, which the
//! packer cannot see and should not be asked to. Meanwhile the pathfinder, who
//! can see it, is not allowed to press the key. So the driver presses it, in
//! neither agent's action space, which §3.3 calls the simpler of the two
//! options and is.
//!
//! ## What is automatic and what is not
//!
//! The **word** is automatic: standing at a bar that has it, with something to
//! hand over, the driver hands it over. The **timing** is not, and the timing
//! is the whole of the task - THE MANSE's gate stands on one rung and every
//! chain through it is due by rung 25 (`crates/engine/tests/quest.rs`). An
//! agent that walks into the first town on rung 8 finishes; one that saves the
//! visit for rung 30 has the same word and no house to open with it.
//!
//! So this makes a key buyable and leaves *going to get it in time* the thing
//! being learned, which is the part a reward can be about.
//!
//! The list is read off the quest's own unpassed stops, so it cannot name a
//! piece the chain does not want and cannot go stale when a door moves.

use gearmaster_console::{Console, Verb};
use gearmaster_trades::quest::{Mark, Progress, Quest};

/// What the chain still wants, by component name.
///
/// Only the stops not yet passed: a word already in the tray is not shopping.
pub fn wanted(q: &Quest, p: &Progress) -> Vec<String> {
    q.stops
        .iter()
        .enumerate()
        .filter(|(i, _)| !p.has(*i))
        .filter_map(|(_, s)| match &s.mark {
            Mark::Holding(name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// Buy or barter for anything on the list that is on this shelf.
///
/// Returns the keys pressed, as `Verb::line` writes them, so a run written out
/// as a proof carries the shopping as well as the road and the packing. A
/// transcript missing any of the three replays into a different run and is not
/// a proof of anything.
///
/// Nothing happens anywhere but a shelf that is actually stocking the thing, so
/// calling it every time the road asks for a packing costs a view read.
pub fn fetch(q: &Quest, p: &Progress, c: &mut Console) -> Vec<String> {
    let want = wanted(q, p);
    let mut said = Vec::new();
    if want.is_empty() {
        return said;
    }
    // One pass. A shelf restocks when the run moves, so a second sweep over the
    // same shelf can only buy the same thing twice.
    for name in &want {
        let v = c.view();
        let Some(shelf) = v.shop.iter().find(|s| &s.piece.name == name) else { continue };
        // Gold first. A word costs one coin, so this is nearly always the way -
        // but the bar does not take money for the ones it tells you, and those
        // are bartered for.
        let by_gold = Verb::Buy { shelf: shelf.index };
        if shelf.affordable && c.menu().contains(&by_gold) {
            let line = by_gold.line();
            if c.apply(by_gold).ok {
                said.push(line);
                continue;
            }
        }
        // Otherwise hand something over - and not something the chain also
        // wants. Every rumour in the game costs one gold, which is what made
        // "hand over the cheapest" hand over the key it had just been given
        // (`pilot.rs`, the note about forty barters and not one rumour kept).
        let paying = shelf.barter.iter().copied().find(|id| {
            let holds_it = |n: &String| {
                v.tray.iter().any(|pc| pc.id == Some(*id) && &pc.name == n)
            };
            !want.iter().any(holds_it)
        });
        let Some(paying) = paying else { continue };
        let swap = Verb::Barter { shelf: shelf.index, paying };
        let line = swap.line();
        if c.menu().contains(&swap) && c.apply(swap).ok {
            said.push(line);
        }
    }
    said
}

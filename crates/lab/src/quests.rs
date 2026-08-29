//! Turning a chain the engine derived into a quest the pathfinder can read.
//!
//! This lives in `lab` and not in `trades` for the same reason `themes.rs`
//! does: it reads the engine, and neither agent may. What crosses the boundary
//! is a list of things that can be seen on a screen, with a tier and a window
//! on each, and no table behind any of them.
//!
//! The chain itself is not written here. `engine::quest::chain_to` walks
//! `EVENTS`, `TOWNS`, `DUNGEONS` and `RUMOURS` backwards from a goal; this is
//! the translation of what it found, one arm per `Mark`, and the arms are
//! total so a mark the engine grows cannot be silently dropped.

use gearmaster_engine::event::Requirement;
use gearmaster_engine::quest as src;
use gearmaster_trades::quest::{Mark, Quest, Stop, Tier};

/// The three the mission names (`design/HANDOFF-two-agents.md` §3.5).
///
/// A name and a goal, and nothing else: what the chain *is* comes from the
/// derivation, so this table cannot go stale the way a written chain would.
pub const NAMED: &[(&str, src::Objective)] = &[
    ("pathfinder_threshold", src::Objective::Class("Threshold-Sighted")),
    ("pathfinder_unwound", src::Objective::Door("the-unwound")),
    ("pathfinder_drover", src::Objective::CountyChain(gearmaster_engine::county::Chain::Drove)),
];

/// The quest by the name a model is written under.
pub fn by_name(name: &str) -> Result<Quest, Undressable> {
    match NAMED.iter().find(|(n, _)| *n == name) {
        Some((n, g)) => quest(n, *g),
        None => Err(Undressable::NoSuchName(name.to_string())),
    }
}

/// Why a derived chain could not be handed to an agent.
#[derive(Clone, Debug)]
pub enum Undressable {
    NoSuchName(String),
    /// The chain derived, and the thing at the end of it is not on the screen.
    ///
    /// **This is a refusal and not a fallback.** A quest whose finish cannot be
    /// recognised is one an agent can never be paid for finishing, so it would
    /// train on the cheap tiers and nothing else - which is the exact failure
    /// the tiers exist to avoid, arrived at from the other direction. Better to
    /// say so here than to hand out a chain with its head missing.
    FinishIsNotOnTheScreen { name: String, mark: String },
}

/// Derive a chain and dress it for the other side of the boundary.
pub fn quest(name: &str, goal: src::Objective) -> Result<Quest, Undressable> {
    let chain = src::chain_to(goal);
    let stops: Vec<Stop> = chain.stations.iter().filter_map(stop).collect();
    match (chain.finish(), stops.last()) {
        (Some(f), s) if s.map(|s| s.tier) != Some(Tier::Finish) => {
            Err(Undressable::FinishIsNotOnTheScreen {
                name: name.to_string(),
                mark: format!("{:?}", f.mark),
            })
        }
        _ => Ok(Quest { name: name.to_string(), stops }),
    }
}

fn stop(s: &src::Station) -> Option<Stop> {
    Some(Stop { tier: tier(s.tier), mark: mark(&s.mark)?, window: s.window })
}

fn tier(t: src::Tier) -> Tier {
    match t {
        src::Tier::Offered => Tier::Offered,
        src::Tier::Prerequisite => Tier::Prerequisite,
        src::Tier::Chose => Tier::Chose,
        src::Tier::Finish => Tier::Finish,
    }
}

/// One arm a mark, and `None` for the ones the screen cannot show.
///
/// **A flag is the one that does not cross.** `View` carries no flags, and it
/// should not - a flag is bookkeeping the run does and a player is never shown
/// the list. So a station marked by a flag is dropped rather than translated
/// into something near it, because a stop an agent cannot see is a stop it
/// cannot aim at and paying for one would be paying for noise.
///
/// What that costs is named where it costs it: `pathfinder_unwound`'s
/// `threshold-cleared` is a flag, and the stop it would have been is carried by
/// the dungeon that sets it, which the screen does show.
fn mark(m: &src::Mark) -> Option<Mark> {
    Some(match m {
        src::Mark::Offered(id) => Mark::Offered(id.to_string()),
        src::Mark::Gate(id) => Mark::Gate(gearmaster_engine::town::by_id(id)?.name.to_string()),
        src::Mark::Entered(id) => Mark::Entered(id.to_string()),
        src::Mark::Wearing(n) => Mark::Wearing(n.to_string()),
        src::Mark::Cleared(n) => Mark::Cleared(*n),
        src::Mark::InCounty => Mark::InCounty,
        src::Mark::Asked(Requirement::Holding(n)) => Mark::Holding(n.to_string()),
        // Flags, counters, county tiles, rarities: bookkeeping the screen does
        // not draw. See the note above.
        src::Mark::Asked(_) => return None,
    })
}

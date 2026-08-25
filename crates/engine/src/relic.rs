//! Rewards that are not gear: run-relics, and things you crush.
//!
//! Everything the road hands out used to be a component, a class, gold or a
//! row. Those are four good answers and they are all the same shape - a thing
//! that makes your board better - and a road with only that vocabulary can
//! only ever say one sentence louder.
//!
//! Two more shapes here.
//!
//! **A run-relic** is a one-cell unique whose stats are a *function of the
//! run*: how many events you have answered, how far you have climbed, what is
//! left in your purse. It costs a cell like anything else, and what it is
//! worth changes while you carry it - which makes it the only piece in the
//! game whose card is different at rung forty from what it was at rung four.
//!
//! **A crushable** is a one-cell unique you destroy to use. It breaks a rule
//! once and is gone: a second town action, a second look at a door you
//! refused, a rung passed without fighting it. Nothing else in the game is
//! spent, and that is what makes carrying one a decision about *when*.
//!
//! The components land with the rest of the mission's catalogue. What is here
//! is the machinery, which is testable without them: a relic's arithmetic is a
//! function over a run, and a run is a thing a test can build.

use crate::run::Run;
use crate::stats::Stats;

/// What a relic is paying right now.
///
/// Two halves, because the two things a relic can be worth live in different
/// places. Stats are the wearer's and go through `player_stats`; speed is not
/// a `Stats` field at all - it is a percentage on an item's cooldown, which is
/// where every other speed in the game lives - so it is carried separately and
/// applied to the profiles.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Payout {
    pub stats: Stats,
    /// Percentage points off every item's cooldown.
    pub speed_pct: i32,
}

/// A one-cell unique worth whatever the run has done so far.
pub struct Relic {
    pub name: &'static str,
    /// What the card says, in the same voice a piece's does.
    pub blurb: &'static str,
    /// Read fresh every time the board is totalled, which is what makes it a
    /// relic rather than a stat line.
    pub pays: fn(&Run) -> Payout,
}

/// Strength per event this run has answered.
pub const TALLY_PER_EVENT: i32 = 2;
/// Rungs climbed per percentage point off every cooldown.
pub const ODOMETER_PER: usize = 10;
/// Gold left unspent per point of power, in hundredths.
pub const LEDGER_PER_POINT: i32 = 40;

pub const RELICS: &[Relic] = &[
    Relic {
        name: "The Tally",
        blurb: "+2 strength for every question you have answered on this road.",
        // Answered, not offered: what it counts is decisions, and walking past
        // a door is a decision the same as going through it.
        pays: |run| Payout {
            stats: Stats {
                strength: run.answered.len() as i32 * TALLY_PER_EVENT,
                ..Stats::ZERO
            },
            speed_pct: 0,
        },
    },
    Relic {
        name: "The Odometer",
        blurb: "+1 speed for every ten rungs you have climbed.",
        // The rung you are standing on rather than the deepest you reached: a
        // Grinder knocked back down is, as far as an odometer is concerned,
        // somewhere lower.
        pays: |run| Payout {
            stats: Stats::ZERO,
            speed_pct: (run.rung / ODOMETER_PER) as i32,
        },
    },
    Relic {
        name: "The Ledger",
        blurb: "Power grows with the gold you have not spent.",
        // The one piece in the game that punishes shopping, which is the whole
        // idea: everything else on the road wants your purse open.
        pays: |run| Payout {
            stats: Stats { power: run.gold.max(0) / LEDGER_PER_POINT, ..Stats::ZERO },
            speed_pct: 0,
        },
    },
];

pub fn relic(name: &str) -> Option<&'static Relic> {
    RELICS.iter().find(|r| r.name == name)
}

pub fn is_relic(name: &str) -> bool {
    relic(name).is_some()
}

/// What breaking one does.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Crush {
    /// A second action in one town. The only legal breach of the one-action
    /// rule, ever - and it is legal because it costs you the key.
    SecondKey,
    /// Ask again at a door you walked away from.
    Appeal,
    /// Pass a rung without fighting it, and without its bounty.
    SkipStone,
}

/// A one-cell unique that is spent rather than worn.
pub struct Crushable {
    pub name: &'static str,
    pub blurb: &'static str,
    pub what: Crush,
}

pub const CRUSHABLES: &[Crushable] = &[
    Crushable {
        name: "the Second Key",
        blurb: "One more door, in one town. Then it is gone.",
        what: Crush::SecondKey,
    },
    Crushable {
        name: "the Appeal",
        blurb: "Somebody will hear you out a second time. Once.",
        what: Crush::Appeal,
    },
    Crushable {
        name: "the Skip Stone",
        blurb: "Step over a rung. It pays nothing, and it is behind you.",
        what: Crush::SkipStone,
    },
];

pub fn crushable(name: &str) -> Option<&'static Crushable> {
    CRUSHABLES.iter().find(|c| c.name == name)
}

pub fn is_crushable(name: &str) -> bool {
    crushable(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vacuous until the components land, and written to stop being vacuous.
    #[test]
    fn anything_named_here_that_exists_is_a_one_cell_unique_off_the_shelves() {
        for (name, what) in RELICS
            .iter()
            .map(|r| (r.name, "relic"))
            .chain(CRUSHABLES.iter().map(|c| (c.name, "crushable")))
        {
            let Some(d) = crate::piece::CATALOG.iter().find(|d| d.name == name) else { continue };
            assert_eq!(d.cells.len(), 1, "{} is a {} and takes {} cells", name, what, d.cells.len());
            assert!(
                crate::piece::is_event_only(name),
                "{} is a {} and could be bought off a shelf",
                name,
                what
            );
        }
    }

    #[test]
    fn no_two_of_these_share_a_name() {
        let mut names: Vec<&str> =
            RELICS.iter().map(|r| r.name).chain(CRUSHABLES.iter().map(|c| c.name)).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
    }

    #[test]
    fn every_one_of_them_says_what_it_is_for() {
        for b in RELICS.iter().map(|r| r.blurb).chain(CRUSHABLES.iter().map(|c| c.blurb)) {
            assert!(b.len() > 20, "a card nobody can read: {:?}", b);
        }
    }
}

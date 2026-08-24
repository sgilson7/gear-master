//! Towns: a rung with nothing on it to fight.
//!
//! Everything else that interrupts the road hands the road straight back. An
//! event stands *in front of* a rung and the rung is still there afterwards; a
//! dungeon stands *beside* one and coming out puts you where you went in. A
//! town is the first thing in the game that is a rung of its own - you clear
//! rung seven, and then you are somewhere, and then you go on to rung eight.
//!
//! You answer one question at the gate: go in, or walk on. Walking on pays the
//! bounty again. Going in buys exactly one of four actions, and then you are
//! back on the road.
//!
//! The one-action rule is the whole design. Four doors and one key makes a town
//! a decision rather than a shopping trip, and it means the four can be tuned
//! against each other instead of against nothing.

/// One of the four things you can do with a visit.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    /// Pray. A stack of Piety, and at five of them, Ticket to Ride.
    Chapel,
    /// The rumour shelves. Paid for by bartering, never with money.
    Pub,
    /// A shift's work: double the last bounty, and a stack of Tired.
    Factory,
    /// Five shelves of gear the ordinary shop does not stock.
    Shop,
}

impl Action {
    pub const ALL: [Action; 4] = [Action::Chapel, Action::Pub, Action::Factory, Action::Shop];

    /// The key a theme looks the name up under. Never shown raw.
    pub fn key(self) -> &'static str {
        match self {
            Action::Chapel => "town-chapel",
            Action::Pub => "town-pub",
            Action::Factory => "town-factory",
            Action::Shop => "town-shop",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Action::Chapel => "THE CHAPEL",
            Action::Pub => "THE PUB",
            Action::Factory => "THE FACTORY",
            Action::Shop => "THE SHOP",
        }
    }

    /// One line under the name: what you walk out with.
    pub fn blurb(self) -> &'static str {
        match self {
            Action::Chapel => {
                "Kneel. A stack of Piety, which starts every fight with a point \
                 of devotion. Five of them and it becomes something else."
            }
            Action::Pub => {
                "Rumours, sold for gear rather than for money. Each one is a \
                 condition on an event that will not happen otherwise."
            }
            Action::Factory => {
                "A shift on the line. Twice what the last fight paid, and a \
                 stack of Tired: three mana of debt at the start of every \
                 fight from here on."
            }
            Action::Shop => {
                "Five shelves of things the road does not stock. Ordinary \
                 money, unusual gear."
            }
        }
    }
}

/// A stop on the road.
#[derive(Copy, Clone, Debug)]
pub struct Town {
    pub id: &'static str,
    /// The rung index you have to have *cleared* for the gate to be here. The
    /// town stands between this rung and the next.
    pub after: usize,
    pub name: &'static str,
    /// Read at the gate, before you decide.
    pub blurb: &'static [&'static str],
}

/// Three of them, spaced so no two compete for the same run.
///
/// Sump Bottom is early enough that a Piety stack has somewhere to go.
/// Kettleworks sits where a doubled bounty is worth something. High Wick is
/// past the VIP area, so a run is never asked to choose between them.
pub const TOWNS: &[Town] = &[
    Town {
        id: "sump-bottom",
        after: 6,
        name: "SUMP BOTTOM",
        blurb: &[
            "The road gives out into standing water and then into a street, \
             which is the same thing here with buildings on it. Everything is \
             on stilts and nothing is on the level.",
            "There is a chapel, a pub, a works, and a man selling out of a \
             cart. You have time for one of them before the water comes up.",
        ],
    },
    Town {
        id: "kettleworks",
        after: 17,
        name: "KETTLEWORKS",
        blurb: &[
            "You can hear it a rung before you can see it. The whole valley is \
             one shift working and one shift asleep, and the two swap over \
             without either of them stopping.",
            "They will take a pair of hands for an hour and pay well for it. \
             They will also take considerably more than an hour, if you are \
             not careful about when you put them down.",
        ],
    },
    Town {
        id: "high-wick",
        after: 31,
        name: "HIGH WICK",
        blurb: &[
            "Above the smoke, finally. High Wick is one street on a ridge with \
             a chapel at one end and a pub at the other, and everybody in it \
             has come up from somewhere worse.",
            "Nobody here asks what you are climbing towards. They have all \
             seen somebody go past on the way to it.",
        ],
    },
];

/// The town standing between `rung - 1` and `rung`, if there is one.
///
/// Read after a rung is cleared: clearing rung six leaves `run.rung` at seven,
/// and Sump Bottom is the thing between them.
pub fn between(rung: usize) -> Option<&'static Town> {
    if rung == 0 {
        return None;
    }
    TOWNS.iter().find(|t| t.after + 1 == rung)
}

pub fn by_id(id: &str) -> Option<&'static Town> {
    TOWNS.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_towns_stand_in_the_same_gap() {
        let mut seen: Vec<usize> = TOWNS.iter().map(|t| t.after).collect();
        seen.sort_unstable();
        let n = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), n, "two towns on one rung");
    }

    #[test]
    fn every_town_is_on_the_road() {
        for t in TOWNS {
            assert!(
                t.after < crate::combat::LADDER.len() - 1,
                "{} stands after the last rung, so nobody ever gets to it",
                t.id
            );
        }
    }

    #[test]
    fn a_town_is_found_by_the_rung_you_arrive_on() {
        for t in TOWNS {
            assert_eq!(between(t.after + 1).map(|x| x.id), Some(t.id));
            assert!(between(t.after).map(|x| x.id) != Some(t.id), "{}: one rung early", t.id);
        }
        assert!(between(0).is_none(), "a fresh run starts in a town");
    }

    #[test]
    fn no_town_shares_a_rung_with_an_event() {
        // Both would want the screen. The event fires on arriving at its rung
        // and so does the town, and there is no sensible order for that.
        for t in TOWNS {
            let clash = crate::event::EVENTS.iter().find(|e| e.at == t.after + 1);
            assert!(clash.is_none(), "{} lands on {}", t.id, clash.map(|e| e.id).unwrap_or(""));
        }
    }

    #[test]
    fn every_action_says_what_it_is_for() {
        for a in Action::ALL {
            assert!(!a.name().is_empty());
            assert!(a.blurb().len() > 30, "{:?} does not explain itself", a);
            assert!(!a.key().is_empty());
        }
    }
}

//! Things that happen on a rung instead of a fight starting.
//!
//! An event stands in front of a rung and asks a question. It never adds a
//! rung of its own - the road is fifty long whichever answers you give - and
//! it never resolves itself: every one of them is a choice the player makes,
//! because an event that decides for you is just a cutscene with extra steps.
//!
//! Adding one is adding an entry to `EVENTS`. The engine works out whether a
//! choice can be taken, `Run::take_choice` applies it, and the interface draws
//! whatever is there. Nothing else has to know.

/// What a choice needs before it can be taken.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Requirement {
    /// Always available.
    None,
    /// A loose component in the tray whose footprint is exactly `w` by `h`,
    /// at some rotation. Handing something over has to cost you something you
    /// could have used.
    LooseItemOfSize { w: u8, h: u8 },
    /// A choice taken at an earlier event, by its label. What you did three
    /// rungs ago is allowed to change what is on offer now.
    Took(&'static str),
}

/// What taking a choice does.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Outcome {
    /// Fight the creature this rung was always going to hand you.
    FightAsWritten,
    /// Fight something else instead. The rung is still one rung.
    FightInstead(&'static str),
    /// Skip the fight. The bounty is paid `times` over, and whatever the
    /// requirement named is taken off you.
    BuyOff { times: i32 },
    /// Walk into a mini dungeon. The rung does not move, so coming out puts
    /// you back in front of the fight you had not got to.
    Enter(&'static str),
}

#[derive(Copy, Clone, Debug)]
pub struct Choice {
    pub label: &'static str,
    /// One line under the label. What it costs, or what you are in for.
    pub blurb: &'static str,
    pub requires: Requirement,
    pub outcome: Outcome,
    /// Shown instead of the choice when the requirement is not met, so a
    /// greyed-out button always says why.
    pub unmet: &'static str,
}

#[derive(Copy, Clone, Debug)]
pub struct LadderEvent {
    /// Stable id, so a run can remember it has answered this one.
    pub id: &'static str,
    /// Rung index it stands on.
    pub at: usize,
    /// The creature whose rung this is - checked against the ladder so a
    /// renumbering cannot leave an event pointing at the wrong fight.
    pub expects: &'static str,
    pub title: &'static str,
    pub prose: &'static [&'static str],
    pub choices: &'static [Choice],
}

pub const EVENTS: &[LadderEvent] = &[
    LadderEvent {
        id: "the-toads-offer",
        at: 2,
        expects: "Bone Archer",
        title: "IT WOULD RATHER NOT",
        prose: &[
            "It has been standing here a while, and it has been thinking.",
            "It does not especially want to fight you. What it wants is the \
             square thing in your bag - it will not say why, and it will not \
             take anything else, and it will not be talked down on the shape.",
            "It has money. It has, in fact, twice what it is worth, which \
             suggests it has done this before.",
        ],
        choices: &[
            Choice {
                label: "TAKE THE DEAL",
                blurb: "Hand over a 2x2 component. No fight, and double the bounty.",
                requires: Requirement::LooseItemOfSize { w: 2, h: 2 },
                outcome: Outcome::BuyOff { times: 2 },
                unmet: "Nothing square enough in the tray.",
            },
            Choice {
                label: "FIGHT IT ANYWAY",
                blurb: "It was going to be a fight. Make it one.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-shrine-fork",
        at: 9,
        expects: "Warded Idol",
        title: "TWO THINGS IN THE SHRINE",
        prose: &[
            "The idol is where the idol always is, warded to the teeth and \
             entirely willing.",
            "Behind it, past a gap in the stone that you would swear was not \
             there a moment ago, something else is asleep. It is armoured like \
             a seed is armoured. It is not dreaming about you, and you get the \
             strong impression that it would be worse if it were.",
            "You can take the idol. Or you can go round the back, and find out \
             what a thing like that has to say.",
        ],
        choices: &[
            Choice {
                label: "FIGHT THE IDOL",
                blurb: "The rung as written.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
            Choice {
                label: "FOLLOW THE THING YOU SOLD",
                blurb: "Three floors, and something at the bottom nobody else can be given.",
                requires: Requirement::Took("TAKE THE DEAL"),
                outcome: Outcome::Enter("the-crevice"),
                unmet: "You kept whatever it wanted, so it never came this way.",
            },
            Choice {
                label: "GO ROUND THE BACK",
                blurb: "A boss, this early, and it leaves something behind.",
                requires: Requirement::None,
                outcome: Outcome::FightInstead("The Dreaming Idiot"),
                unmet: "",
            },
        ],
    },
];

/// The event standing on `rung`, if there is one.
pub fn at(rung: usize) -> Option<&'static LadderEvent> {
    EVENTS.iter().find(|e| e.at == rung)
}

impl Requirement {
    /// Does `shape` - a component's footprint, in cells - satisfy this?
    pub fn met_by_shape(self, cells: &[(u8, u8)]) -> bool {
        match self {
            Requirement::None => true,
            Requirement::Took(_) => true, // answered by the run, not the shape
            Requirement::LooseItemOfSize { w, h } => {
                let (mut mx, mut my) = (0u8, 0u8);
                for &(x, y) in cells {
                    mx = mx.max(x);
                    my = my.max(y);
                }
                let (fw, fh) = (mx + 1, my + 1);
                cells.len() as u32 == w as u32 * h as u32
                    && ((fw == w && fh == h) || (fw == h && fh == w))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::LADDER;

    /// An event points at a rung by number and names the creature it expects
    /// to find there. Renumbering the ladder - which has happened twice - must
    /// not silently leave an event in front of the wrong fight.
    #[test]
    fn every_event_stands_where_it_thinks_it_does() {
        for e in EVENTS {
            let m = LADDER.get(e.at).unwrap_or_else(|| panic!("{}: rung {} is off the end", e.id, e.at));
            assert_eq!(m.name, e.expects, "{} expects {} at rung {}", e.id, e.expects, e.at + 1);
        }
    }

    #[test]
    fn no_two_events_stand_on_the_same_rung() {
        let mut seen = Vec::new();
        for e in EVENTS {
            assert!(!seen.contains(&e.at), "two events on rung {}", e.at + 1);
            seen.push(e.at);
        }
    }

    /// Every event has to offer a way through that needs nothing, or a player
    /// with an empty tray is stuck in front of it forever.
    #[test]
    fn every_event_has_a_way_through_that_costs_nothing() {
        for e in EVENTS {
            assert!(
                e.choices.iter().any(|c| c.requires == Requirement::None),
                "{} can be locked shut",
                e.id
            );
            for c in e.choices {
                if c.requires != Requirement::None {
                    assert!(!c.unmet.is_empty(), "{}: {} never says why", e.id, c.label);
                }
            }
        }
    }

    /// Whatever an event puts in front of you has to exist.
    #[test]
    fn every_alternate_named_by_an_event_is_real() {
        for e in EVENTS {
            for c in e.choices {
                if let Outcome::FightInstead(name) = c.outcome {
                    assert!(
                        crate::combat::alternate(name).is_some(),
                        "{} names {}, which is not an alternate",
                        e.id,
                        name
                    );
                }
                if let Outcome::Enter(id) = c.outcome {
                    assert!(
                        crate::dungeon::by_id(id).is_some(),
                        "{} opens {}, which is not a dungeon",
                        e.id,
                        id
                    );
                }
                // A requirement naming an earlier choice has to name one that
                // exists, or the door is nailed shut and nothing says so.
                if let Requirement::Took(label) = c.requires {
                    assert!(
                        EVENTS.iter().any(|o| o.choices.iter().any(|k| k.label == label)),
                        "{} waits on {:?}, which no choice offers",
                        e.id,
                        label
                    );
                }
            }
        }
    }

    #[test]
    fn a_two_by_two_is_the_only_thing_that_satisfies_a_two_by_two() {
        let r = Requirement::LooseItemOfSize { w: 2, h: 2 };
        assert!(r.met_by_shape(&[(0, 0), (1, 0), (0, 1), (1, 1)]));
        assert!(!r.met_by_shape(&[(0, 0), (1, 0), (2, 0), (3, 0)]));
        assert!(!r.met_by_shape(&[(0, 0), (1, 0), (0, 1)]), "an L is not a square");
        assert!(!r.met_by_shape(&[(0, 0)]));
    }
}

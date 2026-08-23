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
    /// A named component, anywhere you own it - worn or loose. Unlike
    /// `LooseItemOfSize` this is not handed over: the door opens because you
    /// have the key, and you keep the key.
    Holding(&'static str),
}

/// A fight an event sets up, against however many creatures it likes.
///
/// It is not a rung. The ladder does not move whichever way it goes, because
/// an event putting two creatures in front of you is a detour and not a step -
/// whatever the rung was going to hand you is still waiting afterwards.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Brawl {
    /// Everything across the table, by name.
    pub with: &'static [&'static str],
    /// The component you keep if you win. Empty for a fight worth nothing.
    pub win: &'static str,
    /// Rows added to every grid on a win, on top of the component.
    pub and_grow: u8,
    /// Whether losing costs you a life.
    ///
    /// The casino does not: a branch that punishes you for taking the
    /// interesting option is a branch nobody takes twice. What you lose by
    /// losing is the thing you would have won.
    pub forgiving: bool,
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
    /// One more loss before the run ends.
    Spare,
    /// A class handed over on the spot, which no fountain offers.
    Claim(&'static str),
    /// A component handed over on the spot. It arrives loose, in the tray,
    /// where it takes up room like anything else - a reward you have to find
    /// space for is a reward you have to think about.
    Give(&'static str),
    /// Step into a fight the event has arranged. See `Brawl`.
    Step(&'static Brawl),
    /// Put these on the shelves, and hand over what agreeing to them costs.
    ///
    /// The shop is emptied and restocked with exactly `shelves`, which is how
    /// a curated offer works without needing a screen of its own: you walk out
    /// and the shop is different. `class` is the price of the arrangement.
    Stock { shelves: &'static [&'static str], class: &'static str },
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

/// What has to be true before an event will stand in front of you.
///
/// Most events are pinned to a rung and that is the whole condition. Some are
/// earned: the casino opens because of something you did, not because of where
/// you are, and if you never do it the casino never happens.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Trigger {
    /// Stands on `at`, every run, no questions.
    Rung,
    /// Stands on the next rung at or before `at`, but only once you have won a
    /// fight in under `within_ms`. Miss the window and it never fires.
    QuickKill { within_ms: u32 },
}

#[derive(Copy, Clone, Debug)]
pub struct LadderEvent {
    /// Stable id, so a run can remember it has answered this one.
    pub id: &'static str,
    /// Rung index it stands on - or, for an earned event, the last rung it
    /// will still stand on.
    pub at: usize,
    /// What has to be true for it to appear at all.
    pub trigger: Trigger,
    /// The creature whose rung this is - checked against the ladder so a
    /// renumbering cannot leave an event pointing at the wrong fight.
    pub expects: &'static str,
    pub title: &'static str,
    pub prose: &'static [&'static str],
    pub choices: &'static [Choice],
}

/// The two at the third table, and what stepping between them is worth.
///
/// Calibrated against a complete auto-built board, which beats this pair and
/// loses to the next one up. That is the line to hold: the chip is the key to
/// the whole VIP event, so a pair nobody can beat would quietly delete a later
/// event rather than making an early one exciting.
///
/// The casino can open as early as rung one, where a starter board loses this
/// badly - and that is the tension worth having. Step in early and you will
/// probably lose; wait and your build is better, but the door shuts at rung
/// nine. Losing costs nothing either way.
pub static TABLE_THREE: Brawl = Brawl {
    with: &["Bone Archer", "Frost Wisp"],
    win: "Platinum Chip",
    and_grow: 0,
    forgiving: true,
};

/// The two standing over the sprocketmen in the back room.
///
/// Not forgiving. The casino's table is a bet you can walk away from; this is
/// a decision about somebody else, and it costs what losing costs.
pub static THE_BACK_ROOM: Brawl = Brawl {
    with: &["Obsidian Colossus", "Vermin Sovereign"],
    win: "Sprocketman's Gratitude",
    and_grow: 1,
    forgiving: false,
};

pub const EVENTS: &[LadderEvent] = &[
    // Always stands here, whether or not you can go in. A door you cannot
    // open still tells you there was a door, and a player who skipped the
    // casino learns the casino existed - which is the whole reason the chip
    // is worth carrying thirty rungs.
    LadderEvent {
        id: "the-vip-area",
        at: 29,
        trigger: Trigger::Rung,
        expects: "Silence",
        title: "MEMBERS AND GUESTS",
        prose: &[
            "The rope is velvet and the man behind it is not. He looks at you \
             the way a lock looks at a key: with no opinion at all until the \
             right thing is presented.",
            "Behind him, down a corridor lit the colour of weak tea, something \
             is running. Not machinery. You know what machinery sounds like - \
             you were mined out of a cave full of it. This is the sound gear-folk \
             make when they have been at it a very long time and are not \
             expected to stop.",
            "There are five things on a table down there that nobody sells. He \
             would be delighted to show you. He is watching your face while he \
             says it.",
        ],
        choices: &[
            Choice {
                label: "Keep your face still",
                blurb: "Look at the table. Do not look down the corridor.",
                requires: Requirement::Holding("Platinum Chip"),
                outcome: Outcome::Stock {
                    shelves: &[
                        "Overseer's Circlet",
                        "Foreman's Harness",
                        "Tallykeeper's Weave",
                        "Treadmill Sole",
                        "Quota Edge",
                    ],
                    class: "Immense Guilt",
                },
                unmet: "the rope does not move - members only",
            },
            Choice {
                label: "Get them out",
                blurb: "Two of them are paid to stop you. This one costs.",
                requires: Requirement::Holding("Platinum Chip"),
                outcome: Outcome::Step(&THE_BACK_ROOM),
                unmet: "the rope does not move - members only",
            },
            Choice {
                label: "Walk on",
                blurb: "Whatever that is, it is behind a rope and you are not.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // Earned, not scheduled: it turns up the moment you have flattened
    // something inside two seconds, so long as you are still in the shallow
    // end. Build something sharp early and the door is there; do not, and you
    // will finish the run without ever knowing the casino was in the game.
    //
    // `at` is the deadline rather than the address - the last rung it will
    // still stand on.
    LadderEvent {
        id: "the-casino",
        at: 8,
        trigger: Trigger::QuickKill { within_ms: 2_000 },
        expects: "Whisperling",
        title: "A ROOM WITH NO CLOCKS",
        prose: &[
            "Somebody saw what you did to that thing, and somebody told \
             somebody, and now there is a door in a wall you have walked past \
             eleven times. Inside: no clocks, no windows, and a carpet chosen \
             by a man who wanted you to look up.",
            "You are here to play. You have the fnorp for it and everything.",
            "At the third table along, two of them have stopped playing and \
             started on each other, and the room has arranged itself into a \
             ring the way rooms do. Nobody is stopping it. The staff have gone \
             very carefully back to counting.",
        ],
        choices: &[
            Choice {
                label: "Step in",
                blurb: "Both of them. Win and they will remember you.",
                requires: Requirement::None,
                outcome: Outcome::Step(&TABLE_THREE),
                unmet: "",
            },
            Choice {
                label: "Keep out of it",
                blurb: "Their business. Cash out and take the chip.",
                requires: Requirement::None,
                outcome: Outcome::Give("Gold Chip"),
                unmet: "",
            },
        ],
    },
    // Stands on the rung *after* Henpeck, which is where you are once he is
    // down. The theme's cutscene has already played by then - he has told you
    // he sold them, and told you twice - so this is the moment after that,
    // with him still on the floor and still talking.
    LadderEvent {
        id: "what-to-do-with-henpeck",
        at: 15,
        trigger: Trigger::Rung,
        expects: "The Curator",
        title: "HE IS STILL TALKING",
        prose: &[
            "Lord Drabley Henpeck is not dead, and is not especially worried \
             about becoming dead, which between the two of you is the more \
             annoying fact.",
            "He has more. He has names, and routes, and the shape of what was \
             done, and he will trade all of it for the obvious thing. He is \
             enjoying this. He has been enjoying it since he hit the floor.",
            "Or you can stop him enjoying it.",
        ],
        choices: &[
            Choice {
                label: "LET HIM TALK",
                blurb: "What he knows is worth a life. One more loss before the run ends.",
                requires: Requirement::None,
                outcome: Outcome::Spare,
                unmet: "",
            },
            Choice {
                label: "FINISH IT",
                blurb: "Nothing he says is worth this. You walk on angry, and stay angry.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Avenged"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-toads-offer",
        at: 2,
        trigger: Trigger::Rung,
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
        trigger: Trigger::Rung,
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
/// The event standing on `rung`, given what the run has managed so far.
///
/// `best_fight_ms` is the quickest win the run has had, or `None` if it has
/// not won one yet. An earned event fires on the first rung after it qualifies
/// rather than on a fixed one, so it turns up when you have earned it.
pub fn at(rung: usize, best_fight_ms: Option<u32>) -> Option<&'static LadderEvent> {
    EVENTS.iter().find(|e| match e.trigger {
        Trigger::Rung => e.at == rung,
        Trigger::QuickKill { within_ms } => {
            rung <= e.at && best_fight_ms.is_some_and(|ms| ms <= within_ms)
        }
    })
}

impl Requirement {
    /// Does `shape` - a component's footprint, in cells - satisfy this?
    pub fn met_by_shape(self, cells: &[(u8, u8)]) -> bool {
        match self {
            Requirement::None => true,
            // Both of these are answered by the run rather than by a shape.
            Requirement::Took(_) | Requirement::Holding(_) => true,
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

    /// `at` has to stay unique even though an earned event roams.
    ///
    /// `event::at` returns the *first* match, so two events that can both be
    /// standing on one rung means one of them silently never fires - and an
    /// earned event that never fires looks exactly like an earned event
    /// nobody has earned yet.
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
                if let Outcome::Claim(name) = c.outcome {
                    let class = crate::class::CLASSES
                        .iter()
                        .find(|k| k.name == name)
                        .unwrap_or_else(|| panic!("{} claims {}, no such class", e.id, name));
                    // Claimed, not qualified for - so nothing you build can
                    // reach it and a fountain must never offer it.
                    assert!(
                        class.requires.is_empty(),
                        "{} is claimable but also has requirements, so a fountain could pour it",
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

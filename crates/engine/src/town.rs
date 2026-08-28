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

    // ---- the Slagworks ---------------------------------------------------
    //
    // A foundry that keeps melting down what it keeps sending up. Its four
    // doors are all about *changing* what you already own rather than adding
    // to it, which is what a foundry is for and what makes it a different
    // town rather than the same town somewhere else.
    /// Throw one piece into the melt and take back another.
    Crucible,
    /// A curated shelf of enchantments and platings, one visit.
    MoldLine,
    /// Pay, and one piece comes out worth more.
    Tempering,
    /// He has heard something below.
    Foreman,

    // ---- the Manse -------------------------------------------------------
    //
    // A house over a door. Its four are about what you are willing to give up
    // - a piece, a hundred of your health, the use of something - and every
    // one of them is a trade rather than a purchase.
    /// Listen at the cellar door. The man inside sounds insane and is right.
    CellarDoor,
    /// Sell one piece at double, and be noticed if it was a good one.
    Gallery,
    /// Eat. It is a universal constant.
    LongTable,
    /// One piece is cursed for good and worth more for it.
    Library,

    // ---- Extra Large -----------------------------------------------------
    //
    // A store the size of a weather system, all ground floor, no windows. Its
    // four doors follow the one-action rule like every town's. The pedestal
    // does not, and is the only thing in the game that does not.
    /// A curated shelf of Orb-kind pieces, and the two relics that restock.
    Aisle9,
    /// Sell at full price, or leave it on consignment.
    ReturnsDesk,
    /// A free common piece, seeded.
    SampleCounter,
    /// He confirms the store is the only one, on any plane.
    Manager,
    /// Feed it an orb and go where the orb goes.
    ///
    /// **One of the two things outside the one-action rule.** It is not a
    /// door: it stands in the entryway and takes its own key, and a run that
    /// walks in without an orb sees furniture. Two of them exist and they
    /// share one visited-set, because the second is there so a run whose orbs
    /// arrived late can still spend them and not so a patient one spends them
    /// twice.
    Pedestal,
    /// The way down into THE HUNDRED, and every town has one.
    ///
    /// **The other thing outside the one-action rule**, and for the pedestal's
    /// reason rather than a new one: it is not a door either. The county is
    /// *under* the town, five moves of it a trip, one trip per town for the
    /// whole run - so charging a visit for it would make the county something
    /// a run does instead of a town rather than as well as one, and six towns
    /// would become six decisions the county always loses.
    ///
    /// A second use is refused with a line and costs nothing.
    County,
}

impl Action {
    /// The four doors every pinned town has.
    pub const ALL: [Action; 4] = [Action::Chapel, Action::Pub, Action::Factory, Action::Shop];

    /// Every door in the game, so a lint over "does this explain itself" does
    /// not quietly stop covering the ones a hidden town brought.
    pub const EVERY: [Action; 18] = [
        Action::Chapel,
        Action::Pub,
        Action::Factory,
        Action::Shop,
        Action::Crucible,
        Action::MoldLine,
        Action::Tempering,
        Action::Foreman,
        Action::CellarDoor,
        Action::Gallery,
        Action::LongTable,
        Action::Library,
        Action::Aisle9,
        Action::ReturnsDesk,
        Action::SampleCounter,
        Action::Manager,
        Action::Pedestal,
        Action::County,
    ];

    /// Does using this cost you the town's one action?
    ///
    /// Everything does, except the two things that are not doors - the
    /// pedestal in the entryway and the way down into the county - and the
    /// Second Key, which is not a door either and is the only *thing* that
    /// ever breaks the rule.
    pub fn costs_the_visit(self) -> bool {
        !matches!(self, Action::Pedestal | Action::County)
    }

    /// The key a theme looks the name up under. Never shown raw.
    pub fn key(self) -> &'static str {
        match self {
            Action::Chapel => "town-chapel",
            Action::Pub => "town-pub",
            Action::Factory => "town-factory",
            Action::Shop => "town-shop",
            Action::Crucible => "town-crucible",
            Action::MoldLine => "town-mold-line",
            Action::Tempering => "town-tempering",
            Action::Foreman => "town-foreman",
            Action::CellarDoor => "town-cellar-door",
            Action::Gallery => "town-gallery",
            Action::LongTable => "town-long-table",
            Action::Library => "town-library",
            Action::Aisle9 => "town-aisle-nine",
            Action::ReturnsDesk => "town-returns",
            Action::SampleCounter => "town-samples",
            Action::Manager => "town-manager",
            Action::Pedestal => "town-pedestal",
            Action::County => "town-county",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Action::Chapel => "THE CHAPEL",
            Action::Pub => "THE PUB",
            Action::Factory => "THE FACTORY",
            Action::Shop => "THE SHOP",
            Action::Crucible => "THE CRUCIBLE",
            Action::MoldLine => "THE MOLD LINE",
            Action::Tempering => "THE TEMPERING",
            Action::Foreman => "THE FOREMAN",
            Action::CellarDoor => "THE CELLAR DOOR",
            Action::Gallery => "THE GALLERY",
            Action::LongTable => "THE LONG TABLE",
            Action::Library => "THE LIBRARY",
            Action::Aisle9 => "AISLE 9",
            Action::ReturnsDesk => "THE RETURNS DESK",
            Action::SampleCounter => "THE SAMPLE COUNTER",
            Action::Manager => "THE MANAGER",
            Action::Pedestal => "THE PEDESTAL",
            Action::County => "THE WAY DOWN",
        }
    }

    /// The word this door hands over, if it hands one over.
    ///
    /// Read by `no_rumour_is_a_key_to_nothing`'s other half, which asks
    /// whether every word in the game can be come by at all. A door is a third
    /// route beside the bar and an event's gift.
    pub fn gives(self) -> Option<&'static str> {
        match self {
            Action::Foreman => Some("A Word About the Cellar"),
            Action::Gallery => Some("A Word About the Glow"),
            _ => None,
        }
    }

    /// The counter this door moves, if it moves one.
    ///
    /// The reverse index for `Requirement::Counter`. A door that quietly
    /// increments something is the watcher pattern working as designed - the
    /// receipt says nothing and the door that reads the tally is thirty rungs
    /// later - and the cost of that is that nothing could check the tally was
    /// *reachable*. THE FOUNDRY REMEMBERS asked for two melts where the road
    /// offers one. This is what `completable.rs` counts.
    pub fn counts(self) -> Option<&'static str> {
        match self {
            Action::Crucible => Some("crucible-melts"),
            _ => None,
        }
    }

    /// The dungeon this door opens, if it opens one.
    ///
    /// The gallery's is conditional - it wants something Legendary to sell -
    /// and this says what it *can* open rather than what it will, which is the
    /// question "is there any way into that dungeon at all" needs.
    pub fn opens(self) -> Option<&'static str> {
        match self {
            Action::CellarDoor => Some("the-threshold"),
            Action::Gallery => Some("the-undertow"),
            _ => None,
        }
    }

    /// One line under the name: what you walk out with.
    pub fn blurb(self) -> &'static str {
        match self {
            Action::Chapel => {
                "The floor is stone and it cuts. A stack of Piety, which banks \
                 you a point of devotion before every fight from here on. Five \
                 stacks and it turns into something else."
            }
            Action::Pub => {
                "Nobody here takes money. They take what you are carrying, and \
                 what they give back is a rumour: a condition on a door that \
                 will not otherwise be there."
            }
            Action::Factory => {
                "One shift, and they pay on the hour. Twice what the last fight \
                 paid, and a stack of Tired: three mana of debt at the start of \
                 every fight for the rest of the run."
            }
            Action::Shop => {
                "Whoever has the cart this week has five things on it that the \
                 road does not stock. He does take money, and he does want all \
                 of it."
            }
            Action::Crucible => {
                "Throw something in. What comes out is the same slot and about \
                 the same worth and is not the same thing, and nobody here \
                 will tell you what it will be."
            }
            Action::MoldLine => {
                "Ground, and the things that go under gear. One shelf, laid \
                 out once, and there is always a Lightning Rod on it."
            }
            Action::Tempering => {
                "Half a rung's bounty and one piece comes out of the fire \
                 worth ten more. Its name may grow a word. That is the point."
            }
            Action::Foreman => {
                "Ossery has been down there. He will say what he heard, or - \
                 if you already know - he will pay you not to say it back."
            }
            Action::CellarDoor => {
                "Stand at it and listen. Corvin, on the other side, sounds \
                 insane, and everything Corvin says turns out to be true, and \
                 the door is not locked."
            }
            Action::Gallery => {
                "They will take one piece off you at twice what anybody else \
                 pays. If it was a good one, somebody will mention where the \
                 last one like it was fished up."
            }
            Action::LongTable => {
                "Eat. A hundred more maximum health for the rest of the run, \
                 and the pudding is a universal constant."
            }
            Action::Library => {
                "One book, one piece, and the piece carries a curse of misfire \
                 for good and is worth twenty-five more for having read it. \
                 The book was worth it. Probably."
            }
            Action::Aisle9 => {
                "Orbs, and two things that are not orbs and are shelved with \
                 them anyway. It is the only place on any plane that reliably \
                 has an Orb of Travel in stock."
            }
            Action::ReturnsDesk => {
                "They take anything back at what it cost, which nobody else \
                 does - or they will put it out on consignment and you will \
                 see it again three shops later, worth more."
            }
            Action::SampleCounter => {
                "A free one. It is a common and it is genuinely free and \
                 whoever is on the counter would like you to take two."
            }
            Action::Manager => {
                "Mawes will confirm, at length and with documents, that this \
                 store is the only one, on any plane, and that the sign behind \
                 the sign is not a second store."
            }
            Action::Pedestal => {
                "It stands in the entryway and takes an Orb of Travel. Feed it \
                 one and you go where the orb goes and come back here. It is \
                 not a door and it does not cost you your one."
            }
            Action::County => {
                "Steps, and under them a county. Five moves of it, once from \
                 this town, and what you clear down there stays cleared for \
                 the rest of the run. Not a door either, and it costs you \
                 nothing to look."
            }
        }
    }
}

/// How a town comes to be on the road.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Unlock {
    /// Always there. The three shipped towns.
    Pinned,
    /// Not there until something puts it there - an event outcome, usually a
    /// word somebody gave you. Once revealed it stands at its own `after`
    /// like any other town and behaves like one in every other respect.
    Hidden,
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
    pub unlock: Unlock,
    /// The doors this one has.
    ///
    /// The three shipped towns have the same four, which is why this was a
    /// constant for as long as there were only three. A hidden town is hidden
    /// because it is *somewhere else*, and somewhere else has its own doors -
    /// a crucible, a mold line, a cellar - so the list belongs to the town
    /// rather than to the idea of a town.
    pub actions: &'static [Action],
}

/// Three of them, spaced so no two compete for the same run.
///
/// Sump Bottom is early enough that a Piety stack has somewhere to go.
/// Kettleworks sits where a doubled bounty is worth something. High Wick is
/// past the VIP area, so a run is never asked to choose between them.
/// The four doors, and the way down under them.
///
/// `Action::ALL` is still the four, because that is what it means and a dozen
/// tests read it that way. This is what a pinned town without a pedestal
/// actually has: the county stands outside the one-action rule rather than
/// competing inside it, so adding it takes nothing away.
///
/// **Not the pedestal.** Two towns have one - High Wick and Extra Large - and
/// `pedestal::the_pedestal_costs_no_visit_and_is_the_only_thing_that_does_not`
/// is the test that caught this constant handing two more towns a socket.
const PINNED_DOORS: [Action; 5] =
    [Action::Chapel, Action::Pub, Action::Factory, Action::Shop, Action::County];

pub const TOWNS: &[Town] = &[
    Town {
        id: "sump-bottom",
        unlock: Unlock::Pinned,
        actions: &PINNED_DOORS,
        after: 6,
        name: "SUMP BOTTOM",
        blurb: &[
            "Sump Bottom is nine buildings on stilts and one that gave up. \
             The water is at knee height in the street, and the answer here \
             has been to raise every doorstep by a foot each year rather than \
             deal with the water, so the doors are all at different heights \
             and none of them are at yours.",
            "There is a chapel, a pub, a works, and a man selling out of a \
             cart. The chapel bell rings at a quarter past instead of on the \
             hour, because the tide moves it, and nobody has the heart to \
             take that up with anyone.",
            "You have time for one of them before the water comes up.",
        ],
    },
    Town {
        id: "kettleworks",
        unlock: Unlock::Pinned,
        actions: &PINNED_DOORS,
        after: 17,
        name: "KETTLEWORKS",
        blurb: &[
            "You hear Kettleworks a rung before you see it. Two shifts, one \
             working and one asleep, and they change over without either of \
             them stopping: the sleeping shift is walked to the line by the \
             waking shift and shaken awake at the machine.",
            "There is a board bolted to the gate. It says DAYS SINCE and then \
             a number, and the number is 0, and somebody has chalked a small \
             sad face beside it that has been rained on enough to have gone \
             soft.",
            "They will take a pair of hands for an hour and pay properly for \
             it. They will also take considerably more than an hour.",
        ],
    },
    Town {
        id: "high-wick",
        unlock: Unlock::Pinned,
        // The four, and the second pedestal - which is not a door. It is here
        // because the orbs are shop finds: a run whose orbs arrived late still
        // gets to spend them, and one passing here at rung 32 meets the
        // destinations at the band they were packed for.
        actions: &[
            Action::Chapel,
            Action::Pub,
            Action::Factory,
            Action::Shop,
            Action::Pedestal,
            Action::County,
        ],
        after: 31,
        name: "HIGH WICK",
        blurb: &[
            "Above the smoke, finally. High Wick is one street on a ridge: a \
             chapel at the top end, a pub at the bottom, a works that closed, \
             and a shop in what used to be the works.",
            "Everybody here came up from somewhere worse and will tell you \
             which somewhere if you stand still long enough. There is a wall \
             of small brass plates in the pub with names on them, and nobody \
             will explain the wall.",
            "Nobody asks what you are climbing towards. They have all watched \
             somebody go past on the way to it.",
        ],
    },
    // ---- the two the chain finds -----------------------------------------
    //
    // Hidden, and standing where they do for the same reason the pinned three
    // do: nowhere near each other, and nowhere near a town that is already
    // there. The Manse is early because the cellar behind it is what opens the
    // mind lane, and a lane earned at rung forty is a lane nobody uses. The
    // Slagworks is one clear of High Wick so the two never share a stretch of
    // road.
    // EXTRA LARGE. Behind the sign that says LARGE there is a second sign,
    // further back and taller, and only somebody who kept their head whole
    // notices it - which is what makes the Teller's "nothing" choice the
    // secret best one.
    Town {
        id: "extra-large",
        unlock: Unlock::Hidden,
        actions: &[
            Action::Aisle9,
            Action::ReturnsDesk,
            Action::SampleCounter,
            Action::Manager,
            Action::Pedestal,
            Action::County,
        ],
        after: 13,
        name: "EXTRA LARGE",
        blurb: &[
            "It is one room. It is one room the size of a weather system, all \
             ground floor and no windows, and the far wall is a rumour rather \
             than a thing anybody in here has seen.",
            "The aisles are numbered and the numbers go past four figures. \
             Aisle 9 is close enough to the door to walk to, which is the \
             only reason anybody knows what is in it.",
            "In the entryway, between the doors and the trolleys, there is a \
             stone pedestal with a socket in the top of it. The manager is \
             called Mawes, and nobody who works here will discuss the \
             pedestal, Mawes least of all.",
        ],
    },
    Town {
        id: "the-manse",
        unlock: Unlock::Hidden,
        actions: &[
            Action::CellarDoor,
            Action::Gallery,
            Action::LongTable,
            Action::Library,
            Action::County,
        ],
        after: 24,
        name: "THE MANSE",
        blurb: &[
            "The gate had no road behind it and now there is a house behind \
             it, which is the sort of thing that stops being strange about \
             four minutes after you notice it.",
            "Nobody in the Manse asks who you are. Two of them are eating and \
             one of them is reading and all three of them are doing it in \
             rooms you can hear but not find, because the doors here do not \
             stay where they were put.",
            "There is a cellar, and the plate on the gate said HOLLIS, and \
             nobody inside will answer to it. Everybody in the house knows \
             where the cellar is and nobody in the house will take you to it.",
        ],
    },
    Town {
        id: "the-slagworks",
        unlock: Unlock::Hidden,
        actions: &[
            Action::Crucible,
            Action::MoldLine,
            Action::Tempering,
            Action::Foreman,
            Action::County,
        ],
        // Thirty-three, not thirty-two.
        //
        // The spec says "after rung 32 ... one clear of High Wick at 31, so
        // the two never share a stretch of road", and thirty-two is not one
        // clear of thirty-one, it is next to it: the gates would stand on
        // consecutive rungs and a run would meet two towns back to back. The
        // sentence is right and the number was one out.
        after: 33,
        name: "THE SLAGWORKS",
        blurb: &[
            "The glow over the ridge is a foundry, and the foundry has been \
             here longer than the ridge has. Nothing is smelted here. Things \
             are melted down, which is a different job and is done to things \
             that were already finished once.",
            "Two shifts, no gate, and a yard full of moulds stacked in rows \
             going back further than the light does. Every one of them has \
             been used and every one of them is clean.",
            "The foreman is called Ossery and he keeps looking at the floor. \
             Not at anything on it.",
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

    /// A hidden town that shares a gap with a pinned one would be a town
    /// nobody can reach, because `between` takes the first match.
    #[test]
    fn every_town_has_its_gap_to_itself_whether_or_not_it_is_on_the_map() {
        let mut seen: Vec<usize> = TOWNS.iter().map(|t| t.after).collect();
        seen.sort_unstable();
        let n = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), n, "two towns on one rung, and one of them is unreachable");
    }

    #[test]
    fn every_town_has_at_least_one_door() {
        for t in TOWNS {
            assert!(!t.actions.is_empty(), "{} is a town with nothing in it", t.id);
        }
    }

    #[test]
    fn the_three_shipped_towns_are_still_pinned_and_still_have_their_four() {
        for t in TOWNS.iter().filter(|t| matches!(t.unlock, Unlock::Pinned)) {
            // Their four doors, unchanged. High Wick also has the second
            // pedestal, which is not a door: it costs no visit, and a town's
            // *doors* are the things that do.
            let doors: Vec<Action> =
                t.actions.iter().copied().filter(|a| a.costs_the_visit()).collect();
            assert_eq!(doors, Action::ALL, "{} lost a door", t.id);
        }
    }

    #[test]
    fn every_action_says_what_it_is_for() {
        for a in Action::EVERY {
            assert!(!a.name().is_empty());
            assert!(a.blurb().len() > 30, "{:?} does not explain itself", a);
            assert!(!a.key().is_empty());
        }
    }
}

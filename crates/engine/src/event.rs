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

/// The rungs the two shallow-end doors watch, as indices.
///
/// A fight outside this is not evidence about the early game, and letting one
/// count meant a Grinder knocked back from rung eleven could open a door with
/// a fight it won on the way up.
pub const SHALLOW: std::ops::RangeInclusive<usize> = 1..=9;

/// What has to be true before an event will stand in front of you.
///
/// Most events are pinned to a rung and that is the whole condition. Some are
/// earned: the casino opens because of something you did, not because of where
/// you are, and if you never do it the casino never happens.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Trigger {
    /// Stands on `at`, every run, no questions.
    Rung,
    /// Turns up once the run has won a fight in under `within_ms`, anywhere
    /// from rung `from` up to and including `at`. Miss the window and it never
    /// fires.
    QuickKill { within_ms: u32, from: usize },
    /// The other side of the same coin: a win that took *longer* than
    /// `over_ms`. The shallow end has two doors and they are the same
    /// question asked twice - how is this build actually going?
    SlowKill { over_ms: u32, from: usize },
    /// Stands on `at`, but only for somebody carrying the named rumour *and*
    /// answering whatever it is a rumour about.
    ///
    /// Unlike the others this cannot be decided from a rung and two
    /// stopwatches: the conditions are about the board and about the whole run
    /// so far. `event::at` refuses it and `Run::pending_event` answers it,
    /// because the run is the only thing that knows.
    Whispered { rumour: &'static str },
}

impl Trigger {
    /// The first rung an earned event can appear on. Scheduled ones stand on
    /// exactly one rung, so it is that.
    pub fn from(self) -> usize {
        match self {
            Trigger::Rung | Trigger::Whispered { .. } => 0,
            Trigger::QuickKill { from, .. } | Trigger::SlowKill { from, .. } => from,
        }
    }
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
    /// Ids that shut this one for good once they have been answered.
    ///
    /// The two shallow-end doors are alternatives, not a pair: taking the
    /// casino is a statement about the run, and having made it you do not also
    /// get asked the opposite question.
    pub blocked_by: &'static [&'static str],
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
    // ---- the two the pub sells ----
    //
    // Neither stands here for anybody who did not buy the rumour, and neither
    // stands here for somebody who bought it and then did not do the thing.
    // That is the shape of a rumour: it is a bet on the board you will have.
    LadderEvent {
        id: "the-crownwright",
        at: 19,
        trigger: Trigger::Whispered { rumour: "A Word About the Crownwright" },
        blocked_by: &[],
        expects: "Bone Cantor",
        title: "THE HAT MAN OF KOLOK",
        prose: &[
            "The Kolok Hatter works out of one room over a fish shop and does \
             not turn round when you come in, on the grounds that he can hear \
             how full your head is from where he is sitting.",
            "\"Full,\" he says. \"Good. Most of them come up those stairs \
             empty and want me to put something in it. I make hats. I am not \
             a philanthropist and I am very much not a doctor.\"",
            "He will not sell you a hat. He will take a measurement, for the \
             record. The record is a ledger four inches thick that lives \
             under the bench, and he will not let you look in it.",
        ],
        choices: &[
            Choice {
                label: "Stand still for it",
                blurb: "It takes a minute and he hums the entire way round.",
                requires: Requirement::None,
                outcome: Outcome::Give("Crownwright's Measure"),
                unmet: "",
            },
            Choice {
                label: "Ask what he made last",
                blurb: "It is on the shelf behind him at head height. He has been waiting.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Piety"),
                unmet: "",
            },
        ],
    },
    LadderEvent {
        id: "the-green-ledger",
        at: 22,
        trigger: Trigger::Whispered { rumour: "A Word About the Green Ledger" },
        blocked_by: &[],
        expects: "The Gearwright",
        title: "THE GREEN LEDGER",
        prose: &[
            "The tally man has had the same column open for eleven years. He \
             turns the ledger round so you can read the figure at the bottom. \
             It is a large number and it is in green ink, because everything \
             in this ledger is in green ink, including the corrections.",
            "The figure is roughly what you have put into the ground and \
             pulled back out of it since the Cave Rat. He has been keeping \
             count the whole way up, in fives, four strokes and a bar. He \
             will not say who asked him to.",
            "\"Sign it off,\" he says, \"and it closes, and I go home, and my \
             wife has been asking. Or put a line under it and it stays open, \
             and I do not.\"",
        ],
        choices: &[
            Choice {
                label: "Close the column",
                blurb: "He is out of the door before the ink dries. The drawer under it is yours.",
                requires: Requirement::None,
                outcome: Outcome::Give("The Green Ledger"),
                unmet: "",
            },
            Choice {
                label: "Add your own line",
                blurb: "Eleven years is not so long. He says so himself, twice.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Longhauler"),
                unmet: "",
            },
        ],
    },

    // The pay-off for having asked rather than taken. Always stands here, so a
    // player who took Trundle at the roadside sees what the other answer was
    // worth - and a player who never met the cart at all learns there was one.
    LadderEvent {
        id: "where-it-was-going",
        at: 21,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Slag Warden",
        title: "AHEAD OF SCHEDULE",
        prose: &[
            "Kettleworks, twelve rungs and some weeks on, and there is Gerald \
             in the yard with the harness off, eating.",
            "The four tons went in through the doors nine days ago. The man \
             has been paid, has bought a hat with some of it, and is wearing \
             the hat. He is extremely pleased that you asked.",
            "\"Ahead of schedule,\" he says, for the second time in your \
             acquaintance, and this time he has the docket for it, and makes \
             you look at the docket.",
        ],
        choices: &[
            Choice {
                label: "Ask him again",
                blurb: "Whatever he did, he did by not stopping. Nothing starts fast.",
                requires: Requirement::Took("Ask how he does it"),
                outcome: Outcome::Claim("Longhauler"),
                unmet: "You never asked him anything on the road, so he has nothing for you at Kettleworks",
            },
            Choice {
                label: "Let them eat",
                blurb: "Gerald has earned that yard more than you have earned this road.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
        ],
    },
    // Always stands here, whether or not you can go in. A door you cannot
    // open still tells you there was a door, and a player who skipped the
    // casino learns the casino existed - which is the whole reason the chip
    // is worth carrying thirty rungs.
    LadderEvent {
        id: "the-vip-area",
        at: 29,
        trigger: Trigger::Rung,
        blocked_by: &[],
        expects: "Silence",
        title: "MEMBERS AND GUESTS",
        prose: &[
            "The rope is velvet. The man with the clipboard behind it is \
             called Merrik, his badge says HOST, and Merrik would very much \
             like to see the chip.",
            "Down the corridor behind him, past a door stencilled LINE 3 - \
             AUTHORISED ONLY, there is a noise. It is the noise gear-folk \
             make when they have been at something a very long time and \
             nobody has told them when it stops. You were mined out of a \
             cave. You know the noise.",
            "Merrik says there are five items on a table down there that have \
             never been for sale, and that guests are always welcome, and \
             that he will need your voice down and your hands where he can \
             see them. He says the second half in precisely the tone he said \
             the first.",
        ],
        choices: &[
            Choice {
                label: "Keep your face still",
                blurb: "Look at the table. Do not look down the corridor. Merrik checks.",
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
                unmet: "Merrik does not move the rope. Merrik has not moved the rope in eleven years",
            },
            Choice {
                label: "Get them out",
                blurb: "Two of them are paid to stop exactly this. It costs what that costs.",
                requires: Requirement::Holding("Platinum Chip"),
                outcome: Outcome::Step(&THE_BACK_ROOM),
                unmet: "Merrik does not move the rope. Merrik has not moved the rope in eleven years",
            },
            Choice {
                label: "Walk on",
                blurb: "Merrik thanks you for coming and means it, which is the worst of it.",
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
        // Rung two at the earliest: flattening the Cave Rat is not a
        // demonstration of anything, and the door being open before you have
        // built anything makes the first real decision of the run a coin toss.
        trigger: Trigger::QuickKill { within_ms: 3_500, from: 1 },
        blocked_by: &[],
        expects: "Whisperling",
        title: "THE GALAPAGOS EMPORIUM",
        prose: &[
            "The Galapagos Emporium takes anybody who can walk in, which is \
             how you got in. There is a bowl of complimentary Chromatic Rice \
             by the door and a card over it reading ONE (1) HANDFUL - HONOUR \
             SYSTEM - WE ARE WATCHING YOU TAKE IT.",
            "You are here for Kolok Hold-Em, which is Hold-Em except that one \
             card in the deck is a live gooster and no player may look at it. \
             You have the fnorp. You have taken your one handful.",
            "At the third table along, two players have stopped playing Kolok \
             Hold-Em and started on each other. The room has formed a ring \
             around it. A woman with a clipboard is working through the ring \
             taking side bets in a very neat hand, and the dealer is standing \
             perfectly still with the gooster held out at arm's length.",
        ],
        choices: &[
            Choice {
                label: "Step in",
                blurb: "Both of them at once. The clipboard will want your name first.",
                requires: Requirement::None,
                outcome: Outcome::Step(&TABLE_THREE),
                unmet: "",
            },
            Choice {
                label: "Keep out of it",
                blurb: "Not your table. Cash out, and take whatever the window gives you.",
                requires: Requirement::None,
                outcome: Outcome::Give("Gold Chip"),
                unmet: "",
            },
        ],
    },
    // The other shallow-end door, and the opposite question. Shut for good if
    // you took the casino: that was already an answer about how this run is
    // going, and nobody gets asked both.
    LadderEvent {
        id: "the-long-way",
        at: 8,
        // Fifteen seconds, down from twenty.
        //
        // The number is a statement about the shallow ladder, and the shallow
        // ladder was repacked to a curve: rungs 2 to 9 are four to six themed
        // pieces now where they were hand-authored boards two and three times
        // that. A board blunted until it grinds - the winning build with its
        // weapon taken off, at 27x - takes 18.0s at its slowest down there,
        // and took well over twenty against the boards this threshold was set
        // against. Nothing that can still reach the pay-off twelve rungs later
        // is slower than that.
        //
        // A sharp board's slowest shallow fight is 8.0s, so the two doors stay
        // as far apart as they were; and the prose has always said "that last
        // one took eleven seconds", which is nearer fifteen than twenty.
        trigger: Trigger::SlowKill { over_ms: 15_000, from: 1 },
        blocked_by: &["the-casino"],
        expects: "Whisperling",
        title: "GERALD",
        prose: &[
            "That last one took eleven seconds. You know it took eleven \
             seconds because a man at the roadside was counting out loud, and \
             when you finished he wrote the number in a notebook and said \
             nothing else about it.",
            "His cart is ahead of you on the road, pulled by an animal with a \
             brass plate on its harness. The plate gives the species, which \
             is Slow Trundler, and the name, which is Gerald, and the top \
             speed, which is given in metres per hour.",
            "Gerald is hauling four tons of Deep Chocolate to Kettleworks. \
             They set off in the spring. The man says they are ahead of \
             schedule, and shows you the notebook again at a different page, \
             as though that settles it.",
        ],
        choices: &[
            Choice {
                label: "Ask how he does it",
                blurb: "He will not say on the road. He says catch them up when they get there.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
            Choice {
                label: "Walk with them a while",
                blurb: "Gerald's pace, from here on. Everything slower, every plate worth double.",
                requires: Requirement::None,
                outcome: Outcome::Claim("Trundle"),
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
        blocked_by: &[],
        expects: "The Curator",
        title: "HE IS STILL TALKING",
        prose: &[
            "Lord Drabley Henpeck is on the floor of his own counting house \
             with a broken hip and an excellent view of the ceiling, and he \
             is talking.",
            "He has been talking since he went down. He has names. He has \
             routes. He has the clearance order for the Great Gear Cave, \
             filed correctly, in triplicate, because he is exactly the sort \
             of man who would. All of it is available for the obvious \
             consideration.",
            "He is having a marvellous time. He has asked you twice now \
             whether you are getting all this.",
        ],
        choices: &[
            Choice {
                label: "LET HIM TALK",
                blurb: "He wants a witness and a promise. One more loss before the run ends.",
                requires: Requirement::None,
                outcome: Outcome::Spare,
                unmet: "",
            },
            Choice {
                label: "FINISH IT",
                blurb: "The triplicate burns with him. You walk on angry and arrive angrier.",
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
        blocked_by: &[],
        expects: "Bone Archer",
        title: "TWO BY TWO",
        prose: &[
            "The Bog Toad has been sitting in this road since before you got \
             up this morning and has clearly used the time.",
            "It does not want to fight you. It wants the square thing in your \
             bag. It says square, it says two by two, it will not be moved on \
             the shape and it will not say what it is for.",
            "It counts the fnorp out onto a flat stone while you decide. It \
             counts out twice what the thing is worth. Then it counts the \
             whole pile again, gets the same number, and seems mildly \
             disappointed by that.",
        ],
        choices: &[
            Choice {
                label: "TAKE THE DEAL",
                blurb: "Hand over a 2x2 component. No fight, and twice the bounty on the stone.",
                requires: Requirement::LooseItemOfSize { w: 2, h: 2 },
                outcome: Outcome::BuyOff { times: 2 },
                unmet: "Nothing two by two in the bag. It checks. It counts the bag.",
            },
            Choice {
                label: "FIGHT IT ANYWAY",
                blurb: "It was going to be a fight before it was a negotiation.",
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
        blocked_by: &[],
        expects: "Warded Idol",
        title: "THREE THINGS IN THE SHRINE",
        prose: &[
            "The Warded Idol stands in the shrine the way the Warded Idol \
             always stands in the shrine: plated to the eyeballs, wound to \
             the last click, entirely ready for you.",
            "There is also a hole in the back wall, which was not in the back \
             wall when you came in. Down the hole is a seed facility, and in \
             the seed facility is an old analyst named Boyetano who prays on \
             a floor that cuts his knees. He has been waiting a long while \
             for somebody with shoulders.",
            "And there is a third thing behind the altar, asleep, with a \
             shell on it like a walnut. Nobody who works here will look \
             straight at it. The idol does not look at it either, and the \
             idol has no eyes.",
        ],
        choices: &[
            Choice {
                label: "FIGHT THE IDOL",
                blurb: "The rung as written. It has been ready since before you were born.",
                requires: Requirement::None,
                outcome: Outcome::FightAsWritten,
                unmet: "",
            },
            Choice {
                label: "FOLLOW THE THING YOU SOLD",
                blurb: "Three floors down, and Boyetano at the bottom with something to hand over.",
                requires: Requirement::Took("TAKE THE DEAL"),
                outcome: Outcome::Enter("the-crevice"),
                unmet: "You never sold it, so it never came this way, so there is no hole in the wall.",
            },
            Choice {
                label: "GO ROUND THE BACK",
                blurb: "A boss, this early, and it leaves something behind when it stops.",
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
pub fn at(
    rung: usize,
    best_fight_ms: Option<u32>,
    worst_fight_ms: Option<u32>,
    answered: &[&'static str],
) -> Option<&'static LadderEvent> {
    EVENTS.iter().find(|e| {
        if e.blocked_by.iter().any(|id| answered.contains(id)) {
            return false;
        }
        match e.trigger {
            Trigger::Rung => e.at == rung,
            Trigger::QuickKill { within_ms, from } => {
                (from..=e.at).contains(&rung) && best_fight_ms.is_some_and(|ms| ms < within_ms)
            }
            Trigger::SlowKill { over_ms, from } => {
                (from..=e.at).contains(&rung) && worst_fight_ms.is_some_and(|ms| ms > over_ms)
            }
            // Not answerable from here. See `Trigger::Whispered`.
            Trigger::Whispered { .. } => false,
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

    /// A scheduled event has a rung to itself.
    ///
    /// `event::at` returns the *first* match, so two events that both stand on
    /// one rung means one of them silently never fires. Earned events are the
    /// exception and have to be: they roam a window rather than standing
    /// anywhere, several can be open at once, and which one is asked is
    /// settled by the order they are written in and by `blocked_by`. That is
    /// deliberate - the casino comes first, so a run that earned both doors is
    /// offered the casino and answering it shuts the other.
    #[test]
    fn no_two_scheduled_events_stand_on_the_same_rung() {
        let mut seen = Vec::new();
        for e in EVENTS.iter().filter(|e| matches!(e.trigger, Trigger::Rung)) {
            assert!(!seen.contains(&e.at), "two events on rung {}", e.at + 1);
            seen.push(e.at);
        }
    }

    /// A scheduled event standing inside an earned one's window has to be
    /// written after it, or `find` returns the scheduled one every time and
    /// the earned window is quietly shorter than it says.
    #[test]
    fn nothing_scheduled_shadows_an_earned_window() {
        for (i, earned) in EVENTS.iter().enumerate() {
            if matches!(earned.trigger, Trigger::Rung) {
                continue;
            }
            let window = earned.trigger.from()..=earned.at;
            for (j, sched) in EVENTS.iter().enumerate() {
                if !matches!(sched.trigger, Trigger::Rung) || !window.contains(&sched.at) {
                    continue;
                }
                assert!(
                    j > i,
                    "{} stands on rung {} inside {}'s window and is written first",
                    sched.id,
                    sched.at + 1,
                    earned.id
                );
            }
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

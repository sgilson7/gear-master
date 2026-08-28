//! Rumours: components that are conditions rather than gear.
//!
//! A rumour is a real component. It sits in the tray, it takes up a slot there,
//! it can be handed over. What it does not do is go on a board: it has one cell
//! and nothing on it, so seating it would cost you a cell and gain you nothing.
//!
//! What it is *for* is standing as the condition on an event that will not
//! happen otherwise. Holding "A Word About the Crownwright" is what puts the
//! Crownwright's door on rung twenty-one - and only if the other half of the
//! condition is true when you get there.
//!
//! The pub sells them, and it does not take money. You barter: hand over a
//! loose component of the kind it asks for, or another rumour. That is the
//! point of the pub as a door - it is the one place in the game where what you
//! are carrying is worth more than what you have banked.
//!
//! ## Vagueness is the feature
//!
//! `hint` is what the hover says, and it is deliberately not the condition.
//! "They only see people whose heads are already full" is a rumour; "helmet
//! empty cells < 10" is a quest marker. The two are written side by side here
//! so the gap between them stays deliberate.

use crate::piece::PieceKind;

/// What a rumour wants in trade.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Barter {
    /// A loose component of this kind, handed over.
    Kind(PieceKind),
    /// Another rumour, by name. A rumour you have decided you cannot use is
    /// still worth something, which is what stops a bad draw being dead.
    Rumour(&'static str),
}

impl Barter {
    /// What the price says on the shelf. Short: it goes on a card two inches
    /// wide, under a name that has already taken three lines.
    pub fn label(self) -> String {
        match self {
            Barter::Kind(k) => format!("a loose {}", k.name().to_lowercase()),
            Barter::Rumour(n) => format!("the {}", short_name(n)),
        }
    }

    /// The component this wants, if it wants a named one. The interface needs
    /// it to print the *themed* name rather than the canonical one.
    pub fn named(self) -> Option<&'static str> {
        match self {
            Barter::Rumour(n) => Some(n),
            Barter::Kind(_) => None,
        }
    }
}

/// What has to be true when you arrive for the rumour to be worth anything.
///
/// Checked on the rung, not when the rumour is bought: a rumour is a bet on
/// the board you will have, not the one you have.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Condition {
    /// Fewer than `n` empty cells left in that slot.
    Crowded { slot: crate::piece::SlotKind, under: usize },
    /// At least `n` of a resource banked across the entire run, counting every
    /// fight. The only question anything in the game asks about a whole
    /// playthrough rather than a moment in it.
    BankedAllRun { what: crate::piece::Resource, at_least: i32 },
    /// Carrying it is the whole of it.
    ///
    /// The two the pub sells are bets on the board you will have. A word
    /// somebody told you on the road is not a bet - it is a key, and a key
    /// that also wanted your helmet to be full would be a key with a second
    /// lock on it for no reason.
    Carried,
}

impl Condition {
    /// What the rumour is waiting on, in plain words.
    ///
    /// The hint is vague on purpose - working out what it means is the whole of
    /// it - but vague and *silent* are different things. Two authored events
    /// sat behind four gates with no feedback of any kind, and the result was
    /// that nobody ever saw them. This says what is being asked. It does not
    /// say whether you are meeting it, which wants the run in hand and is a
    /// separate job.
    pub fn describe(self) -> String {
        match self {
            Condition::Crowded { slot, under } => format!(
                "it only matters with fewer than {} empty cells left in the {}",
                under,
                slot.name()
            ),
            Condition::BankedAllRun { what, at_least } => format!(
                "it only matters once you have banked {} {} across the whole run",
                at_least,
                what.name()
            ),
            Condition::Carried => "carrying it is the whole of it".into(),
        }
    }
}

pub struct Rumour {
    pub name: &'static str,
    /// Whether the pub will trade for it.
    ///
    /// The two the bar sells are bets you place; the chain's are things
    /// somebody tells you, and a chain you can barter your way into at the
    /// nearest pub is not a chain. `no_rumour_is_a_key_to_nothing` still
    /// insists every one of them can be got at somehow - see the lint.
    pub on_the_bar: bool,
    /// What the hover says. Vague on purpose - see the module note.
    pub hint: &'static str,
    /// What the pub wants for it.
    pub price: Barter,
    /// The event it opens, by id.
    pub opens: &'static str,
    /// What has to be true on that rung.
    pub needs: Condition,
}

pub static RUMOURS: &[Rumour] = &[
    Rumour {
        name: "A Word About the Crownwright",
        on_the_bar: true,
        hint: "Padgett will not measure a head that has nothing in it. \
               Everybody in the bar nods along at this and not one of them can \
               tell you what it means.",
        price: Barter::Kind(PieceKind::Frame),
        opens: "the-crownwright",
        needs: Condition::Crowded { slot: crate::piece::SlotKind::Helmet, under: 10 },
    },
    Rumour {
        name: "A Word About the Green Ledger",
        on_the_bar: true,
        hint: "There is a man called Creel who works in green ink and has \
               been adding up the same column since before the bar had a roof. \
               What Creel is counting, he is counting about you.",
        price: Barter::Rumour("A Word About the Crownwright"),
        opens: "the-green-ledger",
        needs: Condition::BankedAllRun {
            what: crate::piece::Resource::Nature,
            at_least: 100,
        },
    },
    // ---- the chain -------------------------------------------------------
    //
    // Not on the bar. These are things somebody tells you, and a chain you can
    // barter your way into at the nearest pub is not a chain - it is a
    // shopping list. Every one of them is handed over by a door, which is what
    // `no_rumour_is_a_key_to_nothing` checks.
    Rumour {
        name: "A Word About the Wrong Stars",
        // The one the bar sells, and the chain's on-ramp.
        //
        // The spec puts it in the shop's rare pool or behind the casino's
        // second door. Both are luck, and a chain whose first step is luck is
        // a chain most runs never see the shape of - so it goes where every
        // other word in this game is come by, and the two that follow it are
        // handed over by the chain itself.
        on_the_bar: true,
        hint: "A man called Halloway has been thrown out of every observatory \
               on this road for saying one sentence, and the sentence is about \
               the stars going the wrong way.",
        price: Barter::Kind(PieceKind::Crest),
        opens: "the-astronomer",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Cellar",
        on_the_bar: false,
        hint: "There is a house on this road with a cellar door, and the man \
               behind it is called Corvin, and Corvin is not shouting at \
               anybody in this century.",
        price: Barter::Rumour("A Word About the Wrong Stars"),
        opens: "the-locked-gate",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Glow",
        on_the_bar: false,
        hint: "Over the ridge there is a light that is on all night and every \
               night, and whatever is under it keeps melting down what it \
               keeps sending up.",
        price: Barter::Rumour("A Word About the Cellar"),
        opens: "the-glow-over-the-ridge",
        needs: Condition::Carried,
    },
    // ---- the three standalone pairs --------------------------------------
    //
    // Not chain, not bets. Each one is a piece of local business that is going
    // to happen whether or not you are there, and the word is how you find out
    // in time to be. Two of them are bar talk. The third is told to you by a
    // woman who grades things and has just been refused, which is the only
    // door in the game where declining is what pays.
    Rumour {
        name: "A Word About the Thirsty Wizard",
        on_the_bar: true,
        hint: "There is a wizard up the road, Sam the Wise, who wants what is \
               left when you have finished the thing, and has a room full of \
               what is left, and will not say what for.",
        price: Barter::Kind(PieceKind::Ring),
        opens: "the-wizards-thirst",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Exhibition",
        on_the_bar: true,
        hint: "Dorn and Ilder were the best in the world at a thing and have \
               been retired for six years and are, between them, about as \
               bored as it is possible for two people to be.",
        price: Barter::Kind(PieceKind::Mold),
        opens: "the-exhibition",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Picket",
        on_the_bar: false,
        hint: "The arena has stopped. A woman called Nettle has chalked six \
               demands on the gate and one of them is, oddly, about you.",
        price: Barter::Rumour("A Word About the Exhibition"),
        opens: "the-picket-line",
        needs: Condition::Carried,
    },
    // ---- the Switchyard -------------------------------------------------
    //
    // Neither is on the bar, and not by preference: `SHELVES` is exactly six
    // names and every one of them is spoken for, so the pub is full. (It is
    // `SHELVES` that is six and not `SHOP_SIZE`, which went to seven in
    // 2026-08-27 and does not reach the bar at all - a pub stocks itself
    // through `stock_exactly`.) The first is bought
    // from a woman at the roadside and the second is told to you in a signal
    // box, which is the shape the Unwinding's second and third words already
    // have. Both are `Carried` for the reason the module note gives - a word
    // somebody told you is a key, and a key with a second lock on it is a key
    // with a second lock on it for no reason.
    // THE HUNDRED's one word, and it makes the round trip on its own.
    //
    // **Up**: a charcoal burner who has been down there longer than the roads
    // have tells you one thing, and it opens a door on the road that would not
    // otherwise be there. **Down**: what that door tells you back is what the
    // parish chest's third lock was for, and the chest is a county tile that
    // is inert without it.
    //
    // One word rather than two because `SHELVES` is exactly six and full, so
    // neither direction could have gone on the bar anyway - and because a word
    // that goes up and comes back down as an answer is a better shape than two
    // words passing each other.
    Rumour {
        name: "A Word About the Hundred",
        on_the_bar: false,
        hint: "A man who watches a heap of earth for nine days at a time says \
               the ground here is a subdivision of something, and that \
               somebody once counted it.",
        price: Barter::Kind(crate::piece::PieceKind::Accessory),
        opens: "the-county-surveyed",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Sidings",
        on_the_bar: false,
        hint: "There is a yard under the road where the line used to be \
               sorted, and Hesketh says the times are still being kept, which \
               they would not be if nobody was keeping them.",
        price: Barter::Kind(PieceKind::Mold),
        opens: "the-signal-box",
        needs: Condition::Carried,
    },
    Rumour {
        name: "A Word About the Points",
        on_the_bar: false,
        hint: "Ambrose will throw the points for you the way he throws them \
               for the trains, which is on time and one way only, and he has \
               never once been asked which way.",
        price: Barter::Rumour("A Word About the Sidings"),
        opens: "the-turntable",
        needs: Condition::Carried,
    },
];

/// What the bar will hand over, in shelf order.
///
/// The last of them is not a rumour at all. `TROPHY_SHELF` is the trade that
/// makes a boss trophy worth carrying: the counter pays nothing for one, and
/// this is the only other thing in the game that will take one.
pub fn on_offer() -> &'static [&'static str] {
    SHELVES
}

/// The component that stands for the Recycler trade on the shelves.
pub const TROPHY_SHELF: &str = "Scrap Ticket";

/// The same list as a const, because `stock_exactly` wants a slice of names
/// and building one per visit would allocate for nothing.
const SHELVES: &[&str] = &[
    "A Word About the Crownwright",
    "A Word About the Green Ledger",
    "A Word About the Wrong Stars",
    "A Word About the Thirsty Wizard",
    "A Word About the Exhibition",
    TROPHY_SHELF,

];

pub fn by_name(name: &str) -> Option<&'static Rumour> {
    RUMOURS.iter().find(|r| r.name == name)
}

/// Is this component a rumour rather than gear?
pub fn is_rumour(name: &str) -> bool {
    RUMOURS.iter().any(|r| r.name == name)
}

/// The rumour that opens an event, if one does.
pub fn opens(event_id: &str) -> Option<&'static Rumour> {
    RUMOURS.iter().find(|r| r.opens == event_id)
}

/// What a rumour is for, in one line, for the tray hover.
///
/// Built from the reverse index over `EVENTS` rather than from `Rumour::opens`,
/// which is the same fact written down twice and free to drift. If the event
/// moves, this moves with it.
///
/// Deliberately *not* the hint. The hint is vague because working out what a
/// rumour means is the whole of it; this says which door it is a key to and
/// where that door stands, which is the thing a player cannot work out by
/// staring at their tray. Both are shown, one under the other.
pub fn conditions_line(name: &str) -> Option<String> {
    let events = crate::event::conditioned_by(name);
    if events.is_empty() {
        return None;
    }
    let each: Vec<String> =
        events.iter().map(|e| format!("{} - {}", e.title, e.where_it_stands())).collect();
    Some(format!("Conditions: {}", each.join("; ")))
}

/// "the Crownwright" out of "A Word About the Crownwright", for a price label
/// that would otherwise be half as long as the shelf.
fn short_name(full: &str) -> &str {
    // Both prefixes, longest first. Stripping only "A Word About " leaves the
    // article behind, and the caller adds one of its own: "they want the the
    // Crownwright for it".
    for lead in ["A Word About the ", "A Word About "] {
        if let Some(rest) = full.strip_prefix(lead) {
            return rest;
        }
    }
    full
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rumour_is_a_real_component() {
        for r in RUMOURS {
            assert!(
                crate::piece::CATALOG.iter().any(|d| d.name == r.name),
                "{} is a rumour with nothing to hold",
                r.name
            );
        }
        for name in SHELVES {
            assert!(
                by_name(name).is_some() || *name == TROPHY_SHELF,
                "{name} is on the bar and is neither a rumour nor the trophy trade"
            );
            assert!(
                crate::piece::CATALOG.iter().any(|d| d.name == *name),
                "{name} is on the bar and is not a component"
            );
        }
        assert_eq!(
            SHELVES.len(),
            RUMOURS.iter().filter(|r| r.on_the_bar).count() + 1,
            "a rumour on the bar that the bar does not stock, or the other way round"
        );
        assert!(
            crate::piece::is_event_only(TROPHY_SHELF),
            "the trophy trade could be bought with money"
        );
    }

    /// An orphan rumour is dead content: a component that costs a tray slot,
    /// can be bartered for, and is a key to nothing.
    ///
    /// `every_rumour_opens_a_real_event` reads `Rumour::opens` forwards, which
    /// catches a typo in the id. This reads the events backwards, which catches
    /// the other half - an event that stopped being `Whispered`, or moved to a
    /// different rumour, and left this one holding nothing. One assertion,
    /// because the reverse index makes it one.
    #[test]
    fn no_rumour_is_a_key_to_nothing() {
        for r in RUMOURS {
            let events = crate::event::conditioned_by(r.name);
            assert!(!events.is_empty(), "{} conditions no event at all", r.name);
            assert!(
                conditions_line(r.name).is_some_and(|l| l.contains("Conditions:")),
                "{} cannot say what it is for",
                r.name
            );
        }
        // And nothing waits on a rumour that is not one.
        for e in crate::event::EVENTS {
            if let crate::event::Trigger::Whispered { rumour, .. } = e.trigger {
                assert!(by_name(rumour).is_some(), "{} waits on {}, which is not a rumour", e.id, rumour);
            }
        }
    }

    #[test]
    fn every_rumour_opens_a_real_event() {
        for r in RUMOURS {
            let ev = crate::event::EVENTS.iter().find(|e| e.id == r.opens);
            assert!(ev.is_some(), "{} opens {}, which does not exist", r.name, r.opens);
        }
    }

    #[test]
    fn the_hint_does_not_give_it_away() {
        // A hint that names the number is a quest marker, not a rumour. This
        // cannot check for vagueness, but it can check that the condition's
        // own numbers are not printed in it.
        for r in RUMOURS {
            let numbers: Vec<String> = match r.needs {
                Condition::Crowded { under, .. } => vec![under.to_string()],
                Condition::BankedAllRun { at_least, .. } => vec![at_least.to_string()],
                // Nothing to give away: carrying it is the whole of it.
                Condition::Carried => Vec::new(),
            };
            for n in numbers {
                assert!(
                    !r.hint.contains(&n),
                    "{}'s hint prints {}, which is the whole answer",
                    r.name,
                    n
                );
            }
            assert!(r.hint.len() > 40, "{}: a hint has to be worth reading", r.name);
        }
    }

    /// Every rumour can be got at somehow.
    ///
    /// The bar is one way and a door is the other. A word that neither sells
    /// nor is given is a key nobody can pick up, which is the same dead
    /// content as a key to nothing and needs saying from both ends.
    #[test]
    fn every_rumour_can_be_come_by() {
        for r in RUMOURS {
            if r.on_the_bar {
                assert!(SHELVES.contains(&r.name), "{} says it is on the bar", r.name);
                continue;
            }
            let given = crate::event::EVENTS.iter().any(|e| {
                e.choices
                    .iter()
                    .any(|c| matches!(c.outcome, crate::event::Outcome::Give(n) if n == r.name))
            });
            // A town door is a third way. The Slagworks' foreman has been
            // down there and will say what he heard.
            let told = crate::town::TOWNS.iter().any(|t| {
                t.actions.iter().any(|a| a.gives() == Some(r.name))
            });
            // And a county tile is a fourth. B6's arm, landed in the same
            // milestone as the first word that comes up out of THE HUNDRED -
            // a lint added after the content it is about is a lint that was
            // never going to fail.
            let dug_up = crate::event::COUNTY_EVENTS.iter().any(|e| {
                e.choices.iter().any(|c| {
                    crate::event::every_outcome(&c.outcome)
                        .iter()
                        .any(|o| matches!(o, crate::event::Outcome::Give(n) if *n == r.name))
                })
            });
            assert!(
                given || told || dug_up,
                "{} is on nobody's bar, in nobody's gift and under nobody's field",
                r.name
            );
        }
    }

    #[test]
    fn a_rumour_can_always_be_paid_for() {
        for r in RUMOURS {
            match r.price {
                Barter::Kind(k) => assert!(
                    crate::piece::CATALOG.iter().any(|d| d.kind == k),
                    "{}: nothing in the game is a {:?}",
                    r.name,
                    k
                ),
                Barter::Rumour(n) => assert!(
                    by_name(n).is_some() && n != r.name,
                    "{}: priced in a rumour that is not one, or in itself",
                    r.name
                ),
            }
        }
    }

    #[test]
    fn a_rumour_is_never_on_an_ordinary_shelf() {
        for r in RUMOURS {
            assert!(
                crate::piece::is_event_only(r.name),
                "{} could be bought with money, which is not what a rumour is",
                r.name
            );
        }
    }
}

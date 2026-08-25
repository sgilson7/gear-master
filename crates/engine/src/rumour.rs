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
        }
    }
}

pub struct Rumour {
    pub name: &'static str,
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
        hint: "He will not measure a head that has nothing in it. Everybody \
               in the bar nods along at this and not one of them can tell you \
               what it means.",
        price: Barter::Kind(PieceKind::Frame),
        opens: "the-crownwright",
        needs: Condition::Crowded { slot: crate::piece::SlotKind::Helmet, under: 10 },
    },
    Rumour {
        name: "A Word About the Green Ledger",
        hint: "There is a man in green ink who has been adding up the same \
               column since before the bar had a roof. What he is counting, he \
               is counting about you.",
        price: Barter::Rumour("A Word About the Crownwright"),
        opens: "the-green-ledger",
        needs: Condition::BankedAllRun {
            what: crate::piece::Resource::Nature,
            at_least: 100,
        },
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
const SHELVES: &[&str] =
    &["A Word About the Crownwright", "A Word About the Green Ledger", TROPHY_SHELF];

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
        assert_eq!(SHELVES.len(), RUMOURS.len() + 1, "a rumour nobody can buy");
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
            if let crate::event::Trigger::Whispered { rumour } = e.trigger {
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

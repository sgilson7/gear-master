//! The pedestal, and the four places it goes.
//!
//! Every other door in the game is somewhere you arrive at. This one is
//! somewhere you *bring a key to*: it stands in a shop the size of a weather
//! system and takes an Orb of Travel, which is a real weapon core with a real
//! effect on the spells slotted into it, and which is worth buying by somebody
//! who never finds this thing at all.
//!
//! Three rules and they are all about not wasting a player's time:
//!
//! - **An orb is a piece first.** A duplicate is refused by the pedestal and
//!   stays what it was, which is a weapon. Nothing is bricked by being lucky.
//! - **A destination fires once a run**, and the two pedestals share one
//!   visited-set. The second exists so a run whose orbs arrived late can still
//!   spend them, not so a patient run spends them twice.
//! - **An orbless run sees a dormant pedestal**, never an error. It is
//!   furniture with nothing to say, which is a thing the road already has
//!   plenty of.
//!
//! The table is empty until Phase 2. The plumbing is here so that when the
//! orbs land they are content and nothing else.

/// What a destination turns out to be when you get there.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Where {
    /// An event, pushed onto the road stack from somewhere that is not a rung.
    Event(&'static str),
    /// A mini dungeon, entered the way any other is.
    Dungeon(&'static str),
    /// A floor of a dungeon, entered directly.
    ///
    /// Cleared floors from there are walked through and the first one with a
    /// fight in it is fought. An orb is the only way back into a dungeon whose
    /// door is answered, which is what makes a siding worth more than the
    /// door was: it takes you somewhere you have not been.
    Siding { dungeon: &'static str, floor: usize },
    /// THE HUNDRED, at a mouth of your choosing.
    ///
    /// B1.2. The Ordnance's ticket, and the value it keeps over the draft-two
    /// version - "opens a mouth at your current rung", which needed a
    /// use-from-tray verb nothing else in the game has - is the **choice of
    /// mouth**: found or not, which is the only way into a hidden town's steps
    /// for a run that never found the town.
    County,
}

/// Somewhere an orb goes.
#[derive(Copy, Clone, Debug)]
pub struct Destination {
    pub id: &'static str,
    pub name: &'static str,
    /// The orb that is the key to it, by component name.
    pub via_orb: &'static str,
    pub kind: Where,
}

/// The four.
///
/// Two events and two dungeons, and the split is the point: an orb is a
/// ticket to *somewhere*, and somewhere is sometimes a fight and sometimes a
/// town built at ankle height.
pub const DESTINATIONS: &[Destination] = &[
    Destination {
        // The id is a key and stays put; the name is prose and is the event's
        // own title, which nothing lints against the event. Change one and
        // change the other.
        id: "the-thrumbus-race",
        name: "THE BOLTER RACE",
        via_orb: "Wayfarer's Orb",
        kind: Where::Event("the-thrumbus-race"),
    },
    Destination {
        id: "den-rivals",
        name: "DEN RIVALS",
        via_orb: "Pilgrim's Orb",
        kind: Where::Dungeon("den-rivals"),
    },
    Destination {
        id: "mole-town",
        name: "MOLE TOWN",
        via_orb: "Ferry Orb",
        kind: Where::Event("mole-town"),
    },
    Destination {
        id: "wumpus-world",
        name: "WUMPUS WORLD",
        via_orb: "Stray Orb",
        kind: Where::Dungeon("wumpus-world"),
    },

    // ---- the Switchyard's two sidings -----------------------------------
    //
    // The only destinations that go somewhere a run has already been to the
    // edge of, and the reason the yard's rewards are tickets: having walked
    // one line of a yard with two, the somewhere a run wants most is the
    // other line.
    //
    // Each line's buffer stops pay the orb for the *other* line, which makes
    // "a single run cannot see all of it" a property of the graph rather than
    // a promise. A run that walks Down and feeds the Shunter's enters Up,
    // reaches a buffer stop and is paid the Signalman's - whose destination is
    // the Down line, where two floors are already cleared and walked through.
    // Eight floors. The ninth is the other Up-line buffer stop, and the only
    // orb that goes there has been spent.
    Destination {
        id: "the-up-line",
        name: "THE UP LINE",
        via_orb: "Shunter's Orb",
        kind: Where::Siding { dungeon: "the-switchyard", floor: 5 },
    },
    Destination {
        id: "the-down-line",
        name: "THE DOWN LINE",
        via_orb: "Signalman's Orb",
        kind: Where::Siding { dungeon: "the-switchyard", floor: 1 },
    },
    Destination {
        id: "the-hundred",
        name: "THE HUNDRED",
        via_orb: "Surveyor's Orb",
        kind: Where::County,
    },
];

pub fn by_orb(orb: &str) -> Option<&'static Destination> {
    DESTINATIONS.iter().find(|d| d.via_orb == orb)
}

pub fn by_id(id: &str) -> Option<&'static Destination> {
    DESTINATIONS.iter().find(|d| d.id == id)
}

/// Is this component a key to somewhere?
pub fn is_orb_of_travel(name: &str) -> bool {
    by_orb(name).is_some()
}

/// Does any destination put you down on this floor of this dungeon?
///
/// Half of a lint that lives in two files. A floor carries its own entry
/// cutscene only when something can land a run on it rather than walk it
/// there, and `dungeon.rs`'s `no_floor_offers_a_way_in_that_nothing_uses`
/// asks this. Today nothing lands anywhere but floor 0, so it is false for
/// every floor and the lint is vacuous; `Where::Siding` is what gives it
/// something to say.
pub fn lands_on(dungeon: &str, floor: usize) -> bool {
    DESTINATIONS.iter().any(|d| match d.kind {
        Where::Dungeon(id) => id == dungeon && floor == 0,
        Where::Siding { dungeon: id, floor: f } => id == dungeon && f == floor,
        Where::Event(_) | Where::County => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_destination_is_reachable_and_leads_somewhere_real() {
        for d in DESTINATIONS {
            assert!(
                crate::piece::CATALOG.iter().any(|p| p.name == d.via_orb),
                "{} is opened by {}, which is not a component",
                d.id,
                d.via_orb
            );
            match d.kind {
                Where::Dungeon(id) => assert!(
                    crate::dungeon::by_id(id).is_some(),
                    "{} leads to {}, which is not a dungeon",
                    d.id,
                    id
                ),
                Where::Event(id) => assert!(
                    crate::event::EVENTS.iter().any(|e| e.id == id),
                    "{} leads to {}, which is not an event",
                    d.id,
                    id
                ),
                // THE HUNDRED is not a table with ids in it - it is derived
                // from a seed - so what there is to check is that a mouth
                // exists at all, which `county::MOUTHS` guarantees by being
                // one entry per town and `county` tests from both ends.
                Where::County => assert!(
                    !crate::county::MOUTHS.is_empty(),
                    "{} leads to a county with no way in",
                    d.id
                ),
                Where::Siding { dungeon, floor } => {
                    let x = crate::dungeon::by_id(dungeon).unwrap_or_else(|| {
                        panic!("{} sides into {dungeon}, which is not a dungeon", d.id)
                    });
                    assert!(
                        floor < x.floors.len(),
                        "{} sides onto floor {floor} of {dungeon}, which has {}",
                        d.id,
                        x.floors.len()
                    );
                    // The other half of `dungeon::no_floor_offers_a_way_in_
                    // that_nothing_uses`. A siding drops a run into the middle
                    // of a building it may never have been in, and a floor
                    // that says nothing when you arrive is the walked-into-by-
                    // accident problem the entry cutscene exists to answer.
                    assert!(
                        !x.floors[floor].entry.is_empty(),
                        "{} lands on floor {floor} of {dungeon} and nobody says anything",
                        d.id
                    );
                }
            }
        }
    }

    /// A siding is the only kind of destination that can go somewhere twice.
    ///
    /// Two orbs may point into one dungeon, because the whole design is that
    /// each line's buffer stops pay the ticket to the *other* line. What they
    /// may not do is point at the same floor: that is one destination written
    /// twice, and the second orb would be refused as a duplicate destination
    /// while looking like a fresh one.
    #[test]
    fn no_two_sidings_land_on_the_same_floor() {
        let sidings: Vec<_> = DESTINATIONS
            .iter()
            .filter_map(|d| match d.kind {
                Where::Siding { dungeon, floor } => Some((d.id, dungeon, floor)),
                _ => None,
            })
            .collect();
        for (i, a) in sidings.iter().enumerate() {
            for b in &sidings[i + 1..] {
                assert_ne!((a.1, a.2), (b.1, b.2), "{} and {} are one place", a.0, b.0);
            }
        }
    }

    #[test]
    fn no_two_destinations_share_an_orb_or_an_id() {
        for (i, a) in DESTINATIONS.iter().enumerate() {
            for b in &DESTINATIONS[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.via_orb, b.via_orb, "two destinations, one key");
            }
        }
    }
}

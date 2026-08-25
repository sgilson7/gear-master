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

/// The four, in Phase 2. Empty on purpose until then.
pub const DESTINATIONS: &[Destination] = &[];

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

//! The pedestal, before there is anything to feed it.
//!
//! An orb is a **piece first**: a weapon core with a real effect on the spells
//! slotted into it, worth buying by somebody who never finds the thing that
//! takes it. That ordering is what the tests below are mostly about, because
//! it is the thing that is easy to get backwards - a ticket that is useless
//! once spent, and a duplicate that is useless on arrival, would both be
//! rewards that punish luck.
//!
//! The table is empty until Phase 2, so this file is about the machinery: what
//! a pedestal does with something that is not a key, and what an orbless run
//! sees when it walks past one.

mod common;

use gearmaster_engine::pedestal::{self, Where, DESTINATIONS};
use gearmaster_engine::run::Run;

fn a_run() -> Run {
    let mut run = Run::seeded(0x9E5A);
    common::build_full_loadout(&mut run);
    run
}

#[test]
fn an_orbless_run_meets_a_pedestal_and_nothing_happens() {
    // Never an error. A pedestal with nothing to take is furniture, and the
    // road already has plenty of that.
    let mut run = a_run();
    for id in run.inventory() {
        assert!(run.feed_pedestal(id).is_none(), "something that is not a key opened something");
    }
    assert!(run.destinations_visited.is_empty());
    assert!(run.dungeon.is_none());
    assert!(run.forced_event.is_none());
}

#[test]
fn a_piece_you_do_not_own_is_refused() {
    let mut run = a_run();
    let other = Run::seeded(0x1);
    let id = *other.owned.first().expect("a starter piece");
    assert!(run.feed_pedestal(id).is_none());
}

#[test]
fn the_two_pedestals_share_one_visited_set() {
    // The second exists so a run whose orbs arrived late can still spend them,
    // not so a patient run spends them twice. There is one list, and it is on
    // the run rather than on either pedestal.
    let mut run = a_run();
    run.destinations_visited.push("somewhere");
    assert!(run.destinations_visited.contains(&"somewhere"));
    // Nothing about the list mentions which pedestal was fed.
    assert_eq!(run.destinations_visited.len(), 1);
}

#[test]
fn every_orb_is_a_key_to_exactly_one_place() {
    // Vacuous today and the assertion the four orbs will land against.
    for (i, a) in DESTINATIONS.iter().enumerate() {
        for b in &DESTINATIONS[i + 1..] {
            assert_ne!(a.via_orb, b.via_orb);
        }
        assert!(pedestal::by_id(a.id).is_some());
        assert!(pedestal::is_orb_of_travel(a.via_orb));
        match a.kind {
            Where::Dungeon(id) => assert!(gearmaster_engine::dungeon::by_id(id).is_some()),
            Where::Event(id) => {
                assert!(gearmaster_engine::event::EVENTS.iter().any(|e| e.id == id))
            }
        }
    }
}

#[test]
fn an_event_can_be_asked_from_somewhere_that_is_not_a_rung() {
    // The mechanism a destination needs, and the one THE FORK needs too:
    // an event pushed onto the stack by something other than arriving.
    let mut run = a_run();
    run.rung = 30;
    assert!(run.pending_event().is_none(), "rung 31 is bare in the fixture");
    run.forced_event = Some("the-toads-offer");
    let asked = run.pending_event().expect("a forced event is asked wherever you are");
    assert_eq!(asked.id, "the-toads-offer");
    assert!(run.road_stack().iter().any(|i| i.id() == "the-toads-offer"));

    let walk_on = asked.choices.iter().find(|c| c.label == "FIGHT IT ANYWAY").expect("authored");
    run.take_choice(walk_on);
    assert!(run.forced_event.is_none(), "it was asked and is still being asked");
    assert!(run.pending_event().is_none());
}

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
fn the_four_orbs_are_four_keys_to_four_places() {
    assert_eq!(DESTINATIONS.len(), 4);
    let mut kinds = 0;
    for d in DESTINATIONS {
        assert!(pedestal::is_orb_of_travel(d.via_orb));
        let def = gearmaster_engine::piece::CATALOG
            .iter()
            .find(|p| p.name == d.via_orb)
            .expect("a real component");
        // A piece first, and a ticket second. An orb that is only a ticket is
        // a reward that punishes buying one before you find the pedestal.
        assert_eq!(def.kind, gearmaster_engine::piece::PieceKind::Orb, "{}", d.via_orb);
        assert!(
            !gearmaster_engine::piece::is_event_only(def.name),
            "{} cannot be bought, which is what makes it a ticket and nothing else",
            def.name
        );
        if matches!(d.kind, Where::Dungeon(_)) {
            kinds += 1;
        }
    }
    assert_eq!(kinds, 2, "the four destinations are two fights and two places");
}

#[test]
fn feeding_it_an_orb_spends_the_orb_and_goes_where_the_orb_goes() {
    let mut run = a_run();
    let d = &DESTINATIONS[1];
    let id = run.give(d.via_orb).expect("a real orb");
    let got = run.feed_pedestal(id).expect("it took the key");
    assert_eq!(got.id, d.id);
    assert!(!run.owned.contains(&id), "the orb survived the socket");
    assert!(run.destinations_visited.contains(&d.id));
    match d.kind {
        Where::Dungeon(x) => assert_eq!(run.dungeon.map(|(x2, _)| x2.id), Some(x)),
        Where::Siding { dungeon, floor } => {
            assert_eq!(run.dungeon.map(|(x2, _)| x2.id), Some(dungeon));
            // Where it *lands* is the floor; where it ends up may be further
            // on, because a run that has been here before walks past what it
            // beat. That is the walk-through and it is not this test's.
            assert!(run.dungeon.map(|(_, f)| f >= floor).unwrap_or(false));
        }
        Where::Event(x) => assert_eq!(run.forced_event, Some(x)),
    }
    let receipt = run.take_receipt().expect("a resolution");
    assert!(receipt[0].contains(d.via_orb), "{:?}", receipt);
}

#[test]
fn a_second_copy_of_an_orb_is_a_weapon_and_not_a_second_trip() {
    let mut run = a_run();
    let d = &DESTINATIONS[0];
    let first = run.give(d.via_orb).expect("a real orb");
    let second = run.give(d.via_orb).expect("and another");
    assert!(run.feed_pedestal(first).is_some());
    assert!(run.feed_pedestal(second).is_none(), "it went twice");
    assert!(run.owned.contains(&second), "and the spare was eaten for nothing");
}

#[test]
fn the_pedestal_costs_no_visit_and_is_the_only_thing_that_does_not() {
    use gearmaster_engine::town::{Action, TOWNS};
    let mut with: Vec<&str> = TOWNS
        .iter()
        .filter(|t| t.actions.contains(&Action::Pedestal))
        .map(|t| t.id)
        .collect();
    with.sort_unstable();
    assert_eq!(with, vec!["extra-large", "high-wick"], "there are two of them and only two");
    for a in Action::EVERY {
        assert_eq!(
            a.costs_the_visit(),
            a != Action::Pedestal,
            "{:?} is the wrong side of the one-action rule",
            a
        );
    }

    // And the town survives it, which is the whole of "no door consumed".
    let mut run = a_run();
    run.reveal_town("extra-large");
    run.rung = gearmaster_engine::town::by_id("extra-large").expect("authored").after;
    run.force_win();
    run.settle();
    assert!(run.town.is_some());
    run.visit_town(Action::Pedestal);
    assert!(run.town.is_some(), "walking up to the pedestal spent the visit");
    run.visit_town(Action::SampleCounter);
    assert!(run.town.is_none(), "a door did not spend it");
}

#[test]
fn an_orbless_run_meets_a_pedestal_and_nothing_happens() {
    // Never an error. A pedestal with nothing to take is furniture, and the
    // road already has plenty of that.
    let mut run = a_run();
    for id in run.inventory() {
        if pedestal::is_orb_of_travel(run.registry.def(id).name) {
            continue;
        }
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
    // One list, on the run, and nothing in it says which pedestal was fed.
    let mut run = a_run();
    let d = &DESTINATIONS[3];
    let id = run.give(d.via_orb).expect("a real orb");
    assert!(run.feed_pedestal(id).is_some());
    assert_eq!(run.destinations_visited, vec![d.id]);
    let again = run.give(d.via_orb).expect("another");
    assert!(run.feed_pedestal(again).is_none(), "the other pedestal ran the same trip");
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
            Where::Siding { dungeon, floor } => {
                let d = gearmaster_engine::dungeon::by_id(dungeon).expect("a real dungeon");
                assert!(floor < d.floors.len());
            }
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

/// A siding puts you down inside a dungeon and walks you past what you beat.
///
/// The destination this proves against is built here rather than taken from
/// `DESTINATIONS`, because the two real ones are M6's content and this is the
/// plumbing they will arrive on. Everything it exercises - the arm in
/// `feed_pedestal`, `enter_dungeon_at`, the walk-through - is shipped.
#[test]
fn a_siding_lands_you_on_a_floor_and_walks_past_what_you_cleared() {
    let d = gearmaster_engine::dungeon::by_id("the-crevice").expect("shipped");

    // A run that walked the first floor and lost the second is out of it, and
    // the first floor stays beaten.
    let mut run = a_run();
    run.enter_dungeon_at(d, 0);
    run.pending_scene = None;
    run.force_win();
    run.settle();
    run.back_to_loadout();
    assert!(run.leave_dungeon());
    run.take_receipt();
    assert!(run.has_cleared("the-crevice", 0));

    // Coming back in at the mouth walks past it rather than fighting it again.
    run.enter_dungeon_at(d, 0);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(1), "at the first fight it has not had");
    assert_eq!(
        run.take_receipt(),
        Some(vec!["Walked through: The Reciter - cleared".to_string()])
    );

    // And the banner counts this entry's fights, not the building's rooms.
    assert!(
        run.road_stack()[0].describe().contains("floor 1 of 2"),
        "{}",
        run.road_stack()[0].describe()
    );
}

/// The variant exists and nothing uses it yet, which is the phase discipline.
///
/// M6 adds the two orbs and their sidings. Until then this says out loud that
/// the plumbing is landed and inert, so a green suite is not mistaken for
/// content that shipped.
#[test]
fn no_destination_is_a_siding_yet() {
    assert!(
        !DESTINATIONS.iter().any(|d| matches!(d.kind, Where::Siding { .. })),
        "the sidings are M6's; if this is red, the milestone moved and this test should go"
    );
}

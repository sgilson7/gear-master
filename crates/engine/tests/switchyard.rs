//! The floor graph, proved against a dungeon that is not on the road.
//!
//! A set of points is a decision, and every transition around one - clearing a
//! floor, throwing the lever, leaving, losing, coming back in by a siding - is
//! new machinery that six straight lines cannot exercise. So the fixture is
//! `common::A_YARD`, four rooms with a fork at the top, and the shipped
//! dungeons appear here only where the question is "did this stay the same".
//!
//! Nothing in this file is content. `A_YARD` is not in `DUNGEONS`, its floors
//! are creatures that already exist, and the first `MonsterSpec` this mission
//! writes is M6's.

mod common;

use common::A_YARD;
use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::dungeon::by_id;
use gearmaster_engine::run::{Interrupt, Mode, Run};

fn a_run() -> Run {
    let mut run = Run::seeded(0xB0A7);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Easy;
    run.rung = 20;
    run
}

/// Win the floor you are standing on.
fn clear_a_floor(run: &mut Run) {
    run.pending_scene = None;
    run.force_win();
    run.settle();
    run.back_to_loadout();
}

// ------------------------------------------------------------- at the points

#[test]
fn a_fork_stops_the_road() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    assert!(run.road_is_blocked().is_none(), "a dungeon is where the fighting happens");

    clear_a_floor(&mut run);

    assert!(run.at_points, "floor 0 has two ways on and nobody said which");
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(0), "you are still standing on what you beat");
    assert_eq!(
        run.road_is_blocked(),
        Some("the points"),
        "a lever is not a fight, and which fight it will be is what has not been decided"
    );
    let stack = run.road_stack();
    assert!(matches!(stack[0], Interrupt::Points(..)), "the lever is above the dungeon");
    assert!(matches!(stack[1], Interrupt::Dungeon { .. }));
    assert_eq!(
        stack[0].describe(),
        "A TEST YARD - the points after The Reciter: The long road / The short road"
    );
}

#[test]
fn throwing_the_points_moves_you_and_records_it() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.take_receipt();

    assert!(!run.throw_points(9), "there is no ninth lever position");
    assert!(run.throw_points(1), "the short road");

    assert!(!run.at_points);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(2), "on the short road");
    assert_eq!(run.monster().name, "The Watchers");
    assert_eq!(run.took_exits, vec![("a-test-yard", 0, 1)], "which lever, thrown where");
    assert_eq!(
        run.take_receipt(),
        Some(vec!["The points are thrown: The short road".to_string()])
    );
    assert!(run.road_is_blocked().is_none(), "and the road is open again");
}

#[test]
fn the_points_cannot_be_thrown_from_anywhere_else() {
    let mut run = a_run();
    assert!(!run.throw_points(0), "not in a dungeon at all");
    run.enter_dungeon_at(&A_YARD, 0);
    assert!(!run.throw_points(0), "in one, but standing in front of a fight");
}

// ------------------------------------------------------------ what stays beaten

#[test]
fn a_cleared_floor_is_walked_through_on_re_entry() {
    let mut run = a_run();
    // The short road first, all the way to its buffer stop: floors 0 and 2.
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.throw_points(1);
    clear_a_floor(&mut run);
    assert!(run.dungeon.is_none(), "a buffer stop ends it");

    // Back in. Floor 0 is beaten and the short road is walked out, so the one
    // road with a fight left throws itself and floor 1 is what is in front of
    // you - one floor walked through, not two.
    run.enter_dungeon_at(&A_YARD, 0);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(1));
    assert_eq!(
        run.take_receipt(),
        Some(vec!["Walked through: The Reciter - cleared".to_string()])
    );
    clear_a_floor(&mut run); // floor 1, and on to floor 3
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(3));
    assert!(run.leave_dungeon());
    run.take_receipt();

    // And again, with two floors of the long road behind you: both go past.
    run.enter_dungeon_at(&A_YARD, 0);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(3), "at the first thing not yet beaten");
    assert_eq!(
        run.take_receipt(),
        Some(vec![
            "Walked through: The Reciter - cleared".to_string(),
            "Walked through: The Long Haul - cleared".to_string(),
        ]),
        "the run watches the part it knows go past rather than seeing a banner jump"
    );
    assert!(!run.at_points, "one road with a fight left in it is not a decision");
}

/// A road is open while there is a fight down it, not while its next room is
/// unbeaten.
///
/// The two readings agree everywhere except here, and here the naive one loses
/// a run two rooms it never chose to skip: floor 0's long road has been walked
/// as far as its first room, so "the next room is beaten" says that road is
/// finished. It is not - floor 3 is at the end of it and nobody has fought it.
#[test]
fn a_road_half_walked_is_still_a_road() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.throw_points(0); // the long road
    clear_a_floor(&mut run); // floor 1 beaten; floor 3 is not
    assert!(run.leave_dungeon());

    run.enter_dungeon_at(&A_YARD, 0);
    assert!(
        run.at_points,
        "both roads still have a fight in them, so it is still a decision"
    );
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(0));

    // And throwing the lever down the half-walked road walks past what was
    // walked, which is A1.3's "a thrown lever can land you on a cleared line".
    run.throw_points(0);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(3));
    assert_eq!(
        run.take_receipt(),
        Some(vec![
            "The points are thrown: The long road".to_string(),
            "Walked through: The Long Haul - cleared".to_string(),
        ])
    );
}

#[test]
fn a_fork_with_one_open_exit_throws_itself() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.throw_points(1); // the short road, to floor 2
    clear_a_floor(&mut run); // a buffer stop: out the other side
    assert!(run.dungeon.is_none());

    // Come back to the mouth. Floor 0 is beaten and one of its two roads is
    // too, so there is nothing left to decide and the lever throws itself.
    run.enter_dungeon_at(&A_YARD, 0);
    assert!(!run.at_points, "one road left open is not a set of points");
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(1), "on the road nobody has walked");
}

#[test]
fn a_fork_both_of_whose_roads_are_open_is_still_a_decision() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    assert!(run.at_points);
    assert!(run.leave_dungeon());

    run.enter_dungeon_at(&A_YARD, 0);
    assert!(run.at_points, "floor 0 is beaten and both roads out of it are not");
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(0));
}

#[test]
fn a_siding_puts_you_down_past_what_you_would_have_walked() {
    let mut run = a_run();
    // Straight to floor 1, which carries its own way in.
    run.enter_dungeon_at(&A_YARD, 1);
    assert_eq!(run.dungeon.map(|(_, f)| f), Some(1));
    assert_eq!(
        run.pending_scene,
        Some(A_YARD.floors[1].entry),
        "the floor's own entry, not the dungeon's"
    );
    assert!(!run.has_cleared("a-test-yard", 0), "and floor 0 is still unfought");
}

// ----------------------------------------------------------------- leaving

#[test]
fn leaving_costs_no_life_and_keeps_what_was_cleared() {
    for mode in [Mode::Grinder, Mode::Rogue] {
        let mut run = a_run();
        run.mode = mode;
        let (lives, rung, losses) = (run.lives, run.rung, run.losses);

        run.enter_dungeon_at(&A_YARD, 0);
        clear_a_floor(&mut run);
        assert!(run.leave_dungeon(), "{mode:?}: at the points is a place you may leave from");

        assert!(run.dungeon.is_none());
        assert!(!run.at_points);
        assert_eq!(run.lives, lives, "{mode:?}: leaving is not dying");
        assert_eq!(run.rung, rung, "{mode:?}: leaving is not a knock-back");
        assert_eq!(run.losses, losses, "{mode:?}: leaving is not a loss");
        assert!(run.has_cleared("a-test-yard", 0), "{mode:?}: what you cleared stays cleared");
        assert_eq!(
            run.take_receipt(),
            Some(vec!["Left A TEST YARD. What you cleared stays cleared.".to_string()])
        );
    }
}

#[test]
fn leaving_is_refused_from_anywhere_that_is_not_a_landing_or_the_points() {
    let mut run = a_run();
    assert!(!run.leave_dungeon(), "not in one");
    run.enter_dungeon_at(&A_YARD, 0);
    run.fight_next();
    assert!(!run.leave_dungeon(), "a fight you can stop is a fight the oracle cannot price");
}

/// Leaving is allowed everywhere, which is Part E's E-5 taken as recommended.
#[test]
fn a_shipped_dungeon_can_be_left_as_well() {
    let mut run = a_run();
    run.enter_dungeon("the-threshold");
    clear_a_floor(&mut run);
    assert!(run.leave_dungeon(), "a rule that applies to one dungeon is a rule with a list in it");
    assert!(run.has_cleared("the-threshold", 0));
}

// ------------------------------------------------------------------ losing

#[test]
fn losing_keeps_cleared_floors_and_costs_what_it_costs() {
    for mode in [Mode::Grinder, Mode::Rogue] {
        let mut run = a_run();
        run.mode = mode;
        let (lives, rung) = (run.lives, run.rung);

        run.enter_dungeon_at(&A_YARD, 0);
        clear_a_floor(&mut run);
        run.throw_points(0);
        // Floor 1 with the starting board against an alternate: a real loss.
        run.pending_scene = None;
        run.fight_next();
        run.settle();
        run.back_to_loadout();

        assert!(run.dungeon.is_none(), "{mode:?}: losing puts you out of it");
        assert!(!run.at_points);
        assert!(
            run.has_cleared("a-test-yard", 0),
            "{mode:?}: the floor you beat before the one that beat you stays beaten"
        );
        match mode {
            Mode::Grinder => {
                assert_eq!(run.rung, rung - 1, "a Grinder is knocked back");
                assert_eq!(run.lives, lives);
            }
            Mode::Rogue => {
                assert_eq!(run.lives, lives - 1, "a Rogue pays a life");
                assert_eq!(run.rung, rung);
            }
        }
    }
}

// ------------------------------------------------------------------ the banner

#[test]
fn the_banner_counts_fights_not_floors() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    // Four rooms, and the longest road out of the mouth is three fights.
    assert_eq!(
        run.road_stack()[0].describe(),
        "A TEST YARD - The Reciter - floor 1 of 3",
        "the room count is four and the road out is three"
    );

    clear_a_floor(&mut run);
    run.throw_points(1); // the short road: one fight left, not two
    assert_eq!(run.road_stack()[0].describe(), "A TEST YARD - The Watchers - floor 2 of 2");
}

#[test]
fn the_banner_counts_a_walked_through_floor_as_neither() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.throw_points(1);
    clear_a_floor(&mut run);

    // Back in: floor 0 is walked through, and the fight in front of you is the
    // first of this entry as well as the last of the yard.
    run.enter_dungeon_at(&A_YARD, 0);
    assert_eq!(
        run.road_stack()[0].describe(),
        "A TEST YARD - The Long Haul - floor 1 of 2",
        "a floor walked through was not fought today and is not counted as one"
    );
}

/// The shipped banners read what they read at M0 plus the creature's name.
///
/// Re-pinned here rather than loosened: `floor {n} of {m}` was an index and a
/// room count, and A1.4 replaces both with fights, which for a straight line
/// walked from the top are the same two numbers. What is genuinely new is the
/// creature between them, which acceptance criterion 3 asks for by name.
#[test]
fn the_shipped_banner_did_not_change_except_to_say_who_is_in_front_of_you() {
    let d = by_id("the-threshold").expect("shipped");
    let mut run = a_run();
    run.enter_dungeon("the-threshold");
    assert_eq!(d.fights_ahead(0, &[]), 3);
    assert_eq!(run.road_stack()[0].describe(), "THE THRESHOLD - DOORKEEP - floor 1 of 3");
    for d in gearmaster_engine::dungeon::DUNGEONS {
        let mut run = a_run();
        run.enter_dungeon(d.id);
        let want = format!(
            "{} - {} - floor 1 of {}",
            d.name,
            d.floors[0].creature,
            d.floors.len()
        );
        assert_eq!(run.road_stack()[0].describe(), want, "{}", d.id);
    }
}

// ------------------------------------------------------------------- replay

#[test]
fn a_dungeon_with_points_replays_identically() {
    // Two runs, one script, and the script includes a decision. `throw_points`
    // is player input and nothing here consults the PRNG, so the second walk
    // is the first walk.
    let walk = || {
        let mut run = a_run();
        let mut out: Vec<String> = Vec::new();
        run.enter_dungeon_at(&A_YARD, 0);
        for lever in [0usize, 0] {
            out.extend(run.road_stack().iter().map(|i| i.describe()));
            out.push(format!("fighting {}", run.monster().name));
            clear_a_floor(&mut run);
            if let Some(r) = run.take_receipt() {
                out.extend(r);
            }
            if run.at_points {
                run.throw_points(lever);
                if let Some(r) = run.take_receipt() {
                    out.extend(r);
                }
            }
        }
        out.extend(run.road_stack().iter().map(|i| i.describe()));
        out.push(format!("cleared {:?}", run.cleared_floors));
        out.push(format!("took {:?}", run.took_exits));
        out
    };
    assert_eq!(walk(), walk(), "the same script made a different walk");
}

// -------------------------------------------------------------- a whole yard

#[test]
fn a_buffer_stop_pays_its_own_way_and_the_other_one_stays_where_it_is() {
    let mut run = a_run();
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    run.throw_points(1); // the short road
    clear_a_floor(&mut run);

    assert!(run.dungeon.is_none(), "a buffer stop is the end of the dungeon");
    assert!(run.flags.contains(&"took-the-short-road"), "the leaf paid");
    assert!(
        !run.flags.contains(&"took-the-long-road"),
        "and what is at the other buffer stop is still there"
    );
    assert_eq!(run.cleared_floors, vec![("a-test-yard", 0), ("a-test-yard", 2)]);
}

#[test]
fn wiping_forgets_the_yard() {
    let mut run = a_run();
    run.mode = Mode::Rogue;
    run.enter_dungeon_at(&A_YARD, 0);
    clear_a_floor(&mut run);
    assert!(!run.cleared_floors.is_empty() && run.at_points);
    run.wipe();
    assert!(run.cleared_floors.is_empty(), "a new run has not been anywhere");
    assert!(run.took_exits.is_empty());
    assert!(!run.at_points);
    assert!(run.dungeon.is_none());
}

// ------------------------------------------------------------ the catalogue

/// The two orbs open no new footprint family.
///
/// `stepped_component` groups by kind, slot and cells, and appending a sibling
/// to an existing family re-sorts it - which is how a catalogue addition
/// re-dresses creatures that nobody edited (`the-unwinding.md` #19). Both orbs
/// are event-only, so they would be filtered out of every family anyway; the
/// footprints are chosen so the claim does not have to *depend* on that.
#[test]
fn no_orb_in_the_catalogue_shares_a_footprint_with_these_two() {
    use gearmaster_engine::piece::{PieceKind, CATALOG};

    for name in ["Shunter's Orb", "Signalman's Orb"] {
        let mine = CATALOG.iter().find(|d| d.name == name).expect("appended at M5");
        let sharers: Vec<&str> = CATALOG
            .iter()
            .filter(|d| d.name != name && d.kind == PieceKind::Orb && d.cells == mine.cells)
            .map(|d| d.name)
            .collect();
        assert!(sharers.is_empty(), "{name} shares its shape with {sharers:?}");
    }
    // And the two are not each other's siblings either.
    let shape = |n: &str| CATALOG.iter().find(|d| d.name == n).expect("appended").cells;
    assert_ne!(shape("Shunter's Orb"), shape("Signalman's Orb"));
}

/// An orb is a piece before it is a ticket, and for one milestone it is only
/// a piece.
///
/// The two destinations are M6's. A component with no destination is legal and
/// deliberate - `pedestal.rs`'s own doctrine is that an orb is worth buying by
/// somebody who never finds the pedestal - and saying so here stops a green
/// suite reading as though the sidings had shipped.
#[test]
fn the_two_orbs_are_pieces_before_they_are_tickets() {
    use gearmaster_engine::pedestal::is_orb_of_travel;
    use gearmaster_engine::piece::CATALOG;

    for name in ["Shunter's Orb", "Signalman's Orb"] {
        let d = CATALOG.iter().find(|d| d.name == name).expect("appended at M5");
        assert!(!d.triggers.is_empty(), "{name} does nothing to the spells in it");
        assert!(d.power_bonus > 0, "{name} is not worth building around");
        assert!(
            !is_orb_of_travel(name),
            "{name} became a key at M5; the destinations are M6's"
        );
    }
}

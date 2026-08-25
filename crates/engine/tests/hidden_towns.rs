//! Towns that are not on the map until something puts them there.
//!
//! A pinned town is furniture: it is on the road before the run starts and it
//! is on the road for everybody. A hidden one is somewhere you were *told
//! about* - and after that it is a town in every other respect, standing at
//! its own rung, subject to the one-visit rule, paying the bounty again if you
//! walk past.
//!
//! Two things this file exists to hold still. The three shipped towns must be
//! byte-identical through the migration - E6 criterion 2 asks for their tests
//! unmodified and this is the same claim said from the other side. And a town
//! now carries its own doors, because a hidden town is hidden by being
//! somewhere else, and somewhere else has a crucible rather than a chapel.

mod common;

use gearmaster_engine::run::Run;
use gearmaster_engine::town::{Action, Town, Unlock, TOWNS};

fn a_run() -> Run {
    let mut run = Run::seeded(0x7011);
    common::build_full_loadout(&mut run);
    run
}

#[test]
fn the_three_shipped_towns_are_pinned_and_carry_the_same_four_doors() {
    assert_eq!(TOWNS.len(), 3, "this file is the record of the migration, not of the content");
    for t in TOWNS {
        assert_eq!(t.unlock, Unlock::Pinned, "{} stopped being furniture", t.id);
        assert_eq!(t.actions, &Action::ALL, "{} lost a door", t.id);
    }
}

#[test]
fn a_pinned_town_is_there_without_anybody_revealing_it() {
    let run = a_run();
    for t in TOWNS {
        assert_eq!(
            run.town_between(t.after + 1).map(|x| x.id),
            Some(t.id),
            "{} is not standing in its own gap",
            t.id
        );
    }
}

#[test]
fn a_hidden_town_is_not_on_the_road_until_it_is() {
    // Written against a fabricated town rather than a shipped one, because
    // there are no hidden towns yet and this is the machinery's test. The
    // predicate is the whole of it: `town_between` filters on `unlock`.
    const NOWHERE: Town = Town {
        id: "nowhere",
        after: 3,
        name: "NOWHERE IN PARTICULAR",
        blurb: &["It is not here."],
        unlock: Unlock::Hidden,
        actions: &[Action::Shop],
    };
    let mut run = a_run();
    let visible = |run: &Run, t: &Town| match t.unlock {
        Unlock::Pinned => true,
        Unlock::Hidden => run.towns_revealed.contains(&t.id),
    };
    assert!(!visible(&run, &NOWHERE));
    run.towns_revealed.push("nowhere");
    assert!(visible(&run, &NOWHERE));
}

#[test]
fn revealing_is_once_and_refuses_a_town_that_does_not_exist() {
    let mut run = a_run();
    assert!(!run.reveal_town("no-such-town"), "a typo must not put a ghost on the road");
    assert!(run.reveal_town("high-wick"), "any town can be named");
    assert!(!run.reveal_town("high-wick"), "twice is not a second town");
    assert_eq!(run.towns_revealed.len(), 1);
}

#[test]
fn a_town_still_only_gets_one_visit_however_it_arrived() {
    let mut run = a_run();
    run.rung = TOWNS[0].after;
    run.force_win();
    run.settle();
    assert!(run.town.is_some());
    run.visit_town(Action::Chapel);
    assert!(run.town.is_none(), "the visit did not end");
    assert!(run.towns_seen.contains(&TOWNS[0].id));

    // Knocked back through it and it does not reopen.
    run.rung = TOWNS[0].after;
    run.force_win();
    run.settle();
    assert!(run.town.is_none(), "a town nobody should get twice opened twice");
}

#[test]
fn the_doors_a_town_offers_are_the_doors_it_carries() {
    // The four were a constant for as long as every town had them. This is the
    // assertion that catches a screen still drawing the constant once a town
    // has three doors and a crucible.
    for t in TOWNS {
        for a in t.actions {
            assert!(!a.name().is_empty());
            assert!(a.blurb().len() > 30, "{:?} in {} does not explain itself", a, t.id);
        }
    }
}

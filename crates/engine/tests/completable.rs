//! Every door can be reached, opened, and walked through.
//!
//! Three bugs of one shape got this file written, and all three survived a
//! fully green suite because every test was asking the wrong half of the
//! question.
//!
//! - **THE BIGGER SIGN** stood on rung 41 and revealed a town standing after
//!   rung 14. Something *did* reveal it. Nothing asked whether the reveal
//!   could happen in time.
//! - **THE FOUNDRY REMEMBERS** wanted two crucible melts. A town is one visit
//!   and one action, the only second action in the game is the Second Key, and
//!   the key's only source stands at or after the Slagworks' own gate. Two was
//!   a number no run could reach.
//! - **THE PICKET LINE** opens on a word handed out at rung 20 and advertised
//!   a window from rung 13.
//!
//! So the rule this file enforces: for every door, work out the **earliest
//! rung it can possibly be answered on**, and check that everything it depends
//! on can happen at or before that. A dependency that arrives after the window
//! shuts is content nobody will ever see, and it looks exactly like content
//! that works.

use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::{every_outcome, LadderEvent, Outcome, Requirement, Trigger, EVENTS};
use gearmaster_engine::rumour::RUMOURS;
use gearmaster_engine::town::{Unlock, TOWNS};

/// The first rung a door can be answered on, and the last.
///
/// A `Rung` event stands on exactly one rung. `Trigger::from` returns 0 for it
/// - the earliest a *window* opens, which is not the same question - and
/// reading one for the other is how the first version of this audit came back
/// clean on a table with three broken doors in it.
fn window(e: &LadderEvent) -> (usize, usize) {
    match e.trigger {
        Trigger::Rung => (e.at, e.at),
        _ => (e.trigger.from(), e.at),
    }
}

/// A door pushed onto the stack by a pedestal stands on no rung at all.
fn off_the_road(e: &LadderEvent) -> bool {
    matches!(e.trigger, Trigger::WhenFlagged { flag: "never", .. })
}

/// The first rung a pinned town can be walked into. The bar is in every one.
fn first_town() -> usize {
    TOWNS
        .iter()
        .filter(|t| matches!(t.unlock, Unlock::Pinned))
        .map(|t| t.after + 1)
        .min()
        .expect("the road has a town on it")
}

/// The first rung a word can be in your hands, whoever hands it over.
fn word_by(name: &str) -> Option<usize> {
    let r = RUMOURS.iter().find(|r| r.name == name)?;
    if r.on_the_bar {
        return Some(first_town());
    }
    let by_door = EVENTS
        .iter()
        .filter(|e| gives(e, name))
        .map(|e| window(e).0)
        .min();
    let by_town = TOWNS
        .iter()
        .filter(|t| t.actions.iter().any(|a| a.gives() == Some(name)))
        .map(|t| t.after + 1)
        .min();
    match (by_door, by_town) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

fn gives(e: &LadderEvent, name: &str) -> bool {
    e.choices.iter().any(|c| {
        every_outcome(&c.outcome).iter().any(|o| match o {
            Outcome::Give(n) => *n == name,
            Outcome::Step(b) => b.win == name,
            Outcome::SealedBid { lots } => lots.contains(&name),
            _ => false,
        })
    })
}

/// The first rung a flag can be set on, by a door or by a dungeon floor.
fn flag_by(flag: &str) -> Option<usize> {
    let by_door = EVENTS
        .iter()
        .filter(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
            })
        })
        .map(|e| window(e).0)
        .min();
    // A dungeon stands beside the road rather than on it, so the rung it can
    // first be cleared on is the rung its mouth first stands on.
    let by_floor = DUNGEONS
        .iter()
        .filter(|d| {
            d.also.iter().flat_map(every_outcome).any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
        })
        .filter_map(mouth_of)
        .min();
    match (by_door, by_floor) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// The first rung a dungeon's mouth stands on: a door that opens it, or the
/// town whose action does.
fn mouth_of(d: &gearmaster_engine::dungeon::Dungeon) -> Option<usize> {
    let by_door = EVENTS
        .iter()
        .filter(|e| {
            e.choices.iter().any(|c| {
                every_outcome(&c.outcome).iter().any(
                    |o| matches!(o, Outcome::Enter(id) | Outcome::StartDungeon(id) if *id == d.id),
                )
            })
        })
        .map(|e| window(e).0)
        .min();
    let by_town = TOWNS
        .iter()
        .filter(|t| t.actions.iter().any(|a| a.opens() == Some(d.id)))
        .map(|t| t.after + 1)
        .min();
    match (by_door, by_town) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

// ------------------------------------------------------------ the four checks

#[test]
fn every_door_can_be_reached_before_its_window_shuts() {
    for e in EVENTS.iter().filter(|e| !off_the_road(e)) {
        let (_, last) = window(e);
        match e.trigger {
            Trigger::Whispered { rumour, .. } => {
                let when = word_by(rumour)
                    .unwrap_or_else(|| panic!("{} waits on {}, which nothing hands over", e.id, rumour));
                assert!(
                    when <= last,
                    "{} shuts at rung {} and its word arrives at rung {}",
                    e.id,
                    last + 1,
                    when + 1
                );
            }
            Trigger::WhenFlagged { flag, .. } => {
                let when = flag_by(flag)
                    .unwrap_or_else(|| panic!("{} waits on flag {}, which nothing sets", e.id, flag));
                assert!(
                    when <= last,
                    "{} shuts at rung {} and its flag can first be set at rung {}",
                    e.id,
                    last + 1,
                    when + 1
                );
            }
            _ => {}
        }
    }
}

#[test]
fn every_gated_choice_can_be_opened_before_its_door_shuts() {
    for e in EVENTS.iter().filter(|e| !off_the_road(e)) {
        let (_, last) = window(e);
        for c in e.choices {
            match c.requires {
                Requirement::Took(label) => {
                    let when = EVENTS
                        .iter()
                        .filter(|o| o.id != e.id)
                        .filter(|o| o.choices.iter().any(|k| k.label == label))
                        .map(|o| window(o).0)
                        .min()
                        .unwrap_or_else(|| panic!("{}: nothing offers {:?}", e.id, label));
                    assert!(
                        when <= last,
                        "{} wants {:?}, first offered at rung {}, and shuts at rung {}",
                        e.id,
                        label,
                        when + 1,
                        last + 1
                    );
                }
                Requirement::Holding(name) => {
                    // A shelf can sell it at any time; only a *given* item has
                    // a rung, and then it has to arrive first.
                    if gearmaster_engine::piece::is_event_only(name) {
                        let when = EVENTS
                            .iter()
                            .filter(|o| o.id != e.id)
                            .filter(|o| gives(o, name))
                            .map(|o| window(o).0)
                            .min()
                            .unwrap_or_else(|| panic!("{}: nothing hands over {}", e.id, name));
                        assert!(
                            when <= last,
                            "{} wants {}, first handed over at rung {}, and shuts at rung {}",
                            e.id,
                            name,
                            when + 1,
                            last + 1
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// A counter has to be able to reach the number the door asks for.
///
/// THE FOUNDRY REMEMBERS asked for two crucible melts. A town is one visit and
/// one action; the crucible is one town's door; so the counter tops out at one
/// without the Second Key, whose only source stands at or after the Slagworks'
/// own gate. Two was a number no run could reach, and nothing said so.
#[test]
fn every_counter_can_reach_the_number_it_is_asked_for() {
    for e in EVENTS {
        for c in e.choices {
            let Requirement::Counter { what, at_least } = c.requires else { continue };
            // Every place that can move this counter, once each.
            let by_doors = EVENTS
                .iter()
                .flat_map(|o| o.choices)
                .filter(|k| {
                    every_outcome(&k.outcome).iter().any(|o| matches!(o, Outcome::Count(n) if *n == what))
                })
                .count();
            // Plus the town actions the engine counts directly. One visit,
            // one action - so one apiece.
            let by_towns = TOWNS
                .iter()
                .flat_map(|t| t.actions)
                .filter(|a| a.counts() == Some(what))
                .count();
            let reachable = by_doors + by_towns;
            assert!(
                reachable >= at_least as usize,
                "{} wants {} of {:?} and the road offers {}",
                e.id,
                at_least,
                what,
                reachable
            );
        }
    }
}

/// Every door's window is honest about when it can first stand.
///
/// A window that opens ten rungs before its own key can exist is not wrong so
/// much as misleading - the route map draws it, the strip counts it, and a
/// player reads a door that is not there. THE PICKET LINE advertised rung 13
/// for a word first handed over at rung 20.
#[test]
fn no_window_opens_before_its_own_key_can_exist() {
    for e in EVENTS.iter().filter(|e| !off_the_road(e)) {
        let (first, _) = window(e);
        let needed = match e.trigger {
            Trigger::Whispered { rumour, .. } => word_by(rumour),
            Trigger::WhenFlagged { flag, .. } => flag_by(flag),
            _ => None,
        };
        if let Some(when) = needed {
            assert!(
                first >= when,
                "{} says it can stand from rung {}, and what opens it does not exist until rung {}",
                e.id,
                first + 1,
                when + 1
            );
        }
    }
}

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
            // And the buffer stops. Each floor's `also` fires once per clearing
            // and a floor cleared stays cleared, so a run can bank one apiece -
            // which is exactly the arithmetic that makes "tell him both lines"
            // ask for two and get them from two different buffer stops.
            let by_floors = DUNGEONS
                .iter()
                .flat_map(|d| d.floors)
                .filter(|f| {
                    f.also
                        .iter()
                        .flat_map(every_outcome)
                        .any(|o| matches!(o, Outcome::Count(n) if *n == what))
                })
                .count();
            let reachable = by_doors + by_towns + by_floors;
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

/// A silent counter with no door is a promise nothing keeps.
///
/// `Outcome::Count`'s own doc says what it is for: "Nothing says a word; a door
/// forty rungs later reads the tally and says what it noticed." That is the
/// watcher pattern, and it is the closest this game gets to being haunted -
/// but it only works if somebody is watching. Three of the four counters in
/// the game are written by a choice and read by nothing at all:
///
/// - `shook-the-machine`, set by THE DISPENSER losing its gamble
/// - `moles-paid`, set by paying Tibb in MOLE TOWN
/// - `crossed`, set by crossing THE PICKET LINE
///
/// `crucible-melts` is the one with a door on the other end, and THE FOUNDRY
/// REMEMBERS is what a working one looks like.
///
/// This is the mirror of `no_flag_is_waited_on_forever`, which catches a flag
/// waited on and never set; nothing was catching a counter set and never
/// waited on. Shipped as a budget rather than a fix, because closing it means
/// authoring three doors and that is a content mission, not a prose one. It
/// goes down or it does not move.
const COUNTERS_NOBODY_READS: usize = 3;

fn counters_set() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in EVENTS.iter().flat_map(|e| e.choices) {
        for o in every_outcome(&c.outcome) {
            if let Outcome::Count(what) = o {
                if !out.contains(what) {
                    out.push(what);
                }
            }
        }
    }
    for a in TOWNS.iter().flat_map(|t| t.actions) {
        if let Some(what) = a.counts() {
            if !out.contains(&what) {
                out.push(what);
            }
        }
    }
    // A dungeon floor's own `also`, which is the third place a counter can be
    // written and did not exist until floors became a graph. A buffer stop is
    // where a graph puts its rewards - which buffer stop you reached is the
    // whole of what a graph asks - so a counter written there is a counter the
    // road really can move, and a lint that walked only doors and town actions
    // would call it dead content.
    for f in DUNGEONS.iter().flat_map(|d| d.floors) {
        for o in f.also.iter().flat_map(every_outcome) {
            if let Outcome::Count(what) = o {
                if !out.contains(&what) {
                    out.push(what);
                }
            }
        }
    }
    out
}

fn counters_read() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in EVENTS.iter().flat_map(|e| e.choices) {
        if let Requirement::Counter { what, .. } = c.requires {
            if !out.contains(&what) {
                out.push(what);
            }
        }
    }
    out
}

fn unread_counters() -> Vec<&'static str> {
    let read = counters_read();
    let mut out: Vec<&'static str> = counters_set().into_iter().filter(|w| !read.contains(w)).collect();
    out.sort_unstable();
    out
}

#[test]
fn no_more_counters_go_unread_than_already_do() {
    let found = unread_counters();
    assert!(
        found.len() <= COUNTERS_NOBODY_READS,
        "{} counters are set and read by nothing, over a budget of {}:\n{:#?}",
        found.len(),
        COUNTERS_NOBODY_READS,
        found
    );
}

#[test]
fn the_unread_counter_budget_is_not_slack() {
    let found = unread_counters();
    assert_eq!(
        found.len(),
        COUNTERS_NOBODY_READS,
        "the list shrank to {} - lower COUNTERS_NOBODY_READS in the commit that earned it",
        found.len()
    );
}

/// And every counter that is read can also be written, which is the half that
/// was already covered and is worth stating from this side too.
#[test]
fn every_counter_a_door_reads_is_one_something_writes() {
    let set = counters_set();
    for what in counters_read() {
        assert!(set.contains(&what), "a door reads {what:?}, which nothing anywhere counts");
    }
}

/// The target, for the day the three of them have doors.
#[test]
#[ignore]
fn every_counter_is_read_by_something() {
    assert_eq!(unread_counters(), Vec::<&str>::new());
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

// ------------------------------------------------- the fifth shape
//
// A flag that only one choice sets, where that choice both stands in a
// one-rung window and wants something of its own before it can be taken.
//
// This is the shape that got past every other audit in this file, and it cost
// the game its ending. The road past Francis was gated on
// `looked-through-the-lens`. The only thing that sets that flag is one choice
// in THROUGH THE CRACKED LENS, which stands on exactly one rung and wants
// `Holding("The Cracked Lens")` to take. So a run could do the entire chain,
// hold the mainspring, put Francis down and be told the game was over -
// reported from play, not by the suite.
//
// Nothing here was wrong on its own. The flag *is* set by something, so
// `no_flag_is_waited_on_forever` was happy. The window is not before its key,
// so `no_window_opens_before_its_own_key_can_exist` was happy. It takes both
// facts at once to see it, which is the whole reason this file exists.

/// Flags whose only setter is a gated choice in a one-rung window.
fn flags_behind_a_second_key() -> Vec<&'static str> {
    // Every flag any choice anywhere sets.
    let mut all: Vec<&'static str> = Vec::new();
    for c in EVENTS.iter().flat_map(|e| e.choices) {
        for o in every_outcome(&c.outcome) {
            if let Outcome::Flag(f) = o {
                if !all.contains(f) {
                    all.push(f);
                }
            }
        }
    }
    let mut out: Vec<&'static str> = Vec::new();
    for flag in all {
        // Every choice anywhere that sets this flag.
        let mut setters = Vec::new();
        for e in EVENTS.iter() {
            for c in e.choices {
                if every_outcome(&c.outcome).iter().any(|o| matches!(o, Outcome::Flag(f) if *f == flag))
                {
                    setters.push((e, c));
                }
            }
        }
        // Easy if any one of them is free, or stands in a window wider than a
        // single rung - either gives a run more than one chance at it.
        let easy = setters.iter().any(|(e, c)| {
            let (first, last) = window(e);
            c.requires == Requirement::None || last > first
        });
        if !setters.is_empty() && !easy && !out.contains(&flag) {
            out.push(flag);
        }
    }
    out.sort_unstable();
    out
}

/// One flag in the game is this hard to get, and it is not load-bearing.
///
/// `looked-through-the-lens` is the one, and it is deliberate: THROUGH THE
/// CRACKED LENS stands on rung 48 and wants the lens Halloway sold you thirty
/// rungs back. Missing it should cost you *scouting*, which is what the choice
/// is actually for.
///
/// It cost the ending instead, because `run.rs` gated the road past Francis on
/// it. **That gate is not in any table**, so nothing in this file could see it
/// and nothing here can see the next one either - which is exactly why this is
/// a budget rather than a pass. A flag that is this hard to get is a flag no
/// door should hang on alone, and the number existing at all is what makes
/// somebody check before they hang one.
///
/// The road past Francis now stands for a run holding the mainspring **or** a
/// run that looked, so the flag opens a shortcut and no longer owns anything.
const FLAGS_BEHIND_A_SECOND_KEY: &[&str] = &["looked-through-the-lens"];

#[test]
fn no_new_flag_is_set_only_by_a_gated_choice_in_a_one_rung_window() {
    let found = flags_behind_a_second_key();
    assert_eq!(
        found, FLAGS_BEHIND_A_SECOND_KEY,
        "the set of flags whose only setter is a gated choice on a single rung has \
         moved. If something was added: a run that reaches that rung without the \
         second thing can never set it, so nothing load-bearing may wait on it - \
         and this file cannot check that for you, because the gate that broke the \
         ending lived in `run.rs` where no table could see it."
    );
}

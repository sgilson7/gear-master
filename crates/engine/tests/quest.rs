//! The Manse chain, walked in the order a player meets it.
//!
//! `chain.rs` proves each station of the chain opens the next, and it proves it
//! by standing the run at each door in turn - which means it sets `run.rung`
//! **backwards** between two of them. `the_chain_can_be_finished_in_one_run_in_either_mode`
//! answers THE LOCKED GATE at rung 26 and then puts the run back at rung 25 to
//! meet the Manse. That is a true claim about the doors and it is not a claim
//! about the road: nothing in this suite walked the chain forward.
//!
//! It matters here because the chain is the first thing a goal-conditioned
//! agent is being trained to finish (`design/HANDOFF-two-agents.md` §C7), and
//! an agent trained against a chain that cannot be walked in rung order would
//! be learning a road the game does not have.
//!
//! So: one run, rung one upwards, every fight won by fiat, every chain door
//! answered the moment it stands, and nothing ever set backwards.
//!
//! ## The deadline, which is the thing worth pinning
//!
//! THE MANSE is `Unlock::Hidden` and stands `after: 24`. `town::between` finds
//! the town whose `after + 1` **equals** the rung, and `Run::settle` asks it
//! once, the moment rung index 24 is cleared. So the reveal has one rung to
//! land on and the run walks past the house for ever afterwards.
//!
//! Three bands come out of that and all three are measured below:
//!
//! | first word arrives | what happens |
//! |---|---|
//! | up to rung 25 | both doors answered, the gate stands, the class is won |
//! | rungs 26 to 29 | both doors answered, the town revealed, **no gate** |
//! | rung 30 onward | the astronomer's own window has shut; nothing answered |
//!
//! The middle band is the one to know about. A run there answers every door on
//! the chain, takes the correct choice at both, and puts the town on the map -
//! and cannot reach it. Three of the four tiers a quest spec pays
//! (`HANDOFF-two-agents.md` §3.6) pay in full on a run that cannot finish,
//! which is what makes "the finish is worth more than every step combined" a
//! rule about this game rather than a general worry.
//!
//! `crates/lab/src/bin/qchain.rs` is the printer these numbers were read off.

use gearmaster_engine::combat::Difficulty;
use gearmaster_engine::run::{Mode, Run};
use gearmaster_engine::town::Action;

const WRONG_STARS: &str = "A Word About the Wrong Stars";
const CELLAR: &str = "A Word About the Cellar";
const THRESHOLD: &str = "Threshold-Sighted";

/// The doors the walk answers, and the choice it takes at each.
///
/// Everything else on the road is answered with the first choice that is open,
/// so the walk is never stopped by a door it had no key to - and never *helped*
/// by one either, because none of these choices is on the chain.
const TAKE: &[(&str, &str)] =
    &[("the-astronomer", "Hear him out"), ("the-locked-gate", "Use the word")];

/// What one walk met, by rung index.
#[derive(Default, Debug)]
struct Walk {
    /// Each chain door, and the rung it was answered on.
    answered: Vec<(&'static str, usize)>,
    /// The rung the Manse's gate stood on, if it ever did.
    gate: Option<usize>,
    /// Whether the town went onto the map at all.
    revealed: bool,
    /// The rung the class was won on.
    class: Option<usize>,
    /// The highest rung the walk stood on. A walk that stalls says so here.
    deepest: usize,
}

/// Walk one run from rung one, answering the chain as early as it can be
/// answered, with the first word handed over on `word_at`.
///
/// Bounded twice - by the rung it stops at and by the trips round the loop -
/// because a walk that runs until it runs out is a hang (trap 24), and this one
/// can meet a set of points (trap 23).
fn walk(word_at: usize, mode: Mode) -> Walk {
    let mut run = Run::seeded(0xC4A1);
    run.mode = mode;
    run.difficulty = Difficulty::Easy;
    let mut w = Walk::default();

    for _ in 0..600 {
        w.deepest = w.deepest.max(run.rung);
        if run.rung >= 40 || w.class.is_some() {
            break;
        }
        run.back_to_loadout();
        if run.rung == word_at && !run.holds(WRONG_STARS) {
            run.give(WRONG_STARS);
        }
        // A fountain stands in front of whatever else is on its rung, and
        // `pending_event` will not answer over the top of one.
        if run.at_fountain() || run.at_doubling_fountain() {
            run.drink();
            continue;
        }
        if let Some(e) = run.pending_event() {
            let wanted = TAKE
                .iter()
                .find(|(id, _)| *id == e.id)
                .and_then(|(_, l)| e.choices.iter().find(|c| c.label == *l))
                .filter(|c| run.choice_open(c));
            let Some(c) = wanted.or_else(|| e.choices.iter().find(|c| run.choice_open(c))) else {
                break;
            };
            let at = run.rung;
            let on_the_chain = wanted.is_some();
            run.take_choice(c);
            run.take_receipt();
            // `run.answered`, not the return value: `take_choice` hands back the
            // component it took, and most doors take nothing (trap 21).
            if !run.answered.contains(&e.id) {
                break;
            }
            if on_the_chain {
                w.answered.push((e.id, at));
            }
            w.revealed |= run.towns_revealed.contains(&"the-manse");
            continue;
        }
        if let Some(t) = run.pending_town() {
            if t.id == "the-manse" {
                w.gate = Some(run.rung);
                run.visit_town(Action::CellarDoor);
                run.take_receipt();
                continue;
            }
            run.skip_town();
            continue;
        }
        if run.at_points {
            run.throw_points(0);
            continue;
        }
        if run.dungeon.is_some() {
            run.pending_scene = None;
            run.force_win();
            if run.classes.iter().any(|c| c.name == THRESHOLD) {
                w.class = Some(run.rung);
            }
            continue;
        }
        run.force_win();
    }
    w
}

// -------------------------------------------------------------- the walk

#[test]
fn the_manse_chain_can_be_walked_in_rung_order() {
    for mode in [Mode::Grinder, Mode::Rogue] {
        let w = walk(7, mode);
        assert_eq!(
            w.answered.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec!["the-astronomer", "the-locked-gate"],
            "{mode:?}: the chain was not answered in order: {w:?}"
        );
        assert!(w.gate.is_some(), "{mode:?}: the Manse's gate never stood: {w:?}");
        assert!(w.class.is_some(), "{mode:?}: the stair was never walked: {w:?}");
    }
}

/// Each station is answered strictly after the one before it, and inside its
/// own window.
///
/// This is the half `chain.rs` cannot state, because it puts the run where each
/// door is rather than letting the run arrive.
#[test]
fn every_station_is_answered_after_the_one_that_opened_it() {
    let w = walk(7, Mode::Grinder);
    let by = |id: &str| w.answered.iter().find(|(k, _)| *k == id).map(|(_, r)| *r);
    let astronomer = by("the-astronomer").expect("the first door stood");
    let gate = by("the-locked-gate").expect("the second door stood");
    assert!(astronomer < gate, "the gate was answered before the word for it existed");
    let manse = gearmaster_engine::town::by_id("the-manse").expect("authored");
    assert!(
        gate <= manse.after,
        "the gate was answered on rung {} and the house stands after rung {}",
        gate + 1,
        manse.after + 1
    );
    assert_eq!(
        w.gate,
        Some(manse.after + 1),
        "the gate stood somewhere other than the rung the table gives it"
    );
}

// ---------------------------------------------------------- the deadline

/// The last rung the first word can arrive on and still buy the class.
///
/// Rung index 24, which is displayed rung 25 - the rung immediately before the
/// Manse's gate. Both chain doors can be answered on one rung, because
/// `Run::standing_events` is asked again after every answer and puts whispered
/// doors first, so hearing the astronomer out on rung 25 puts THE LOCKED GATE
/// on rung 25 as well.
const LAST_RUNG_THE_WORD_CAN_ARRIVE: usize = 24;

#[test]
fn the_word_has_to_be_in_hand_before_the_manse_gate() {
    let manse = gearmaster_engine::town::by_id("the-manse").expect("authored");
    assert_eq!(
        LAST_RUNG_THE_WORD_CAN_ARRIVE, manse.after,
        "the house moved; the deadline is the rung before its gate and this constant is not it"
    );
    for at in 0..=LAST_RUNG_THE_WORD_CAN_ARRIVE {
        let w = walk(at, Mode::Grinder);
        assert!(
            w.class.is_some(),
            "a word handed over on rung {} did not reach the stair: {w:?}",
            at + 1
        );
    }
    let late = walk(LAST_RUNG_THE_WORD_CAN_ARRIVE + 1, Mode::Grinder);
    assert!(
        late.class.is_none() && late.gate.is_none(),
        "a word one rung past the deadline still reached the house: {late:?}"
    );
}

/// The four rungs where every tier pays and nothing can be finished.
///
/// A run that comes by the word here answers both doors, takes the correct
/// choice at both, and puts THE MANSE on the map - and the gate it opened
/// stands twenty-five rungs behind. The chain is over and the run does not
/// know it.
///
/// This is the trajectory `crates/trades/tests/quest.rs` scores against a
/// finishing one. It is not a hypothetical: it is four rungs of this road.
#[test]
fn a_run_past_the_deadline_still_answers_every_door_on_the_chain() {
    let astronomer = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-astronomer")
        .expect("authored");
    // The band runs from the rung after the deadline to the last rung the
    // astronomer will still stand on, which is what shuts it.
    for at in LAST_RUNG_THE_WORD_CAN_ARRIVE + 1..=astronomer.at {
        let w = walk(at, Mode::Grinder);
        assert_eq!(
            w.answered.len(),
            2,
            "rung {}: the doors stopped standing before the window said they would: {w:?}",
            at + 1
        );
        assert!(w.revealed, "rung {}: the gate did not open onto the town: {w:?}", at + 1);
        assert!(w.gate.is_none(), "rung {}: the house stood twice: {w:?}", at + 1);
        assert!(w.class.is_none(), "rung {}: the stair was walked after the house: {w:?}", at + 1);
    }
    // And past the astronomer's own window there is no chain at all, which is
    // the honest end of the band rather than more of it.
    let never = walk(astronomer.at + 1, Mode::Grinder);
    assert!(never.answered.is_empty(), "a door stood past its own window: {never:?}");
}

// ------------------------------------------------- what the walk relies on

/// The cellar door is the only way into the mind lane, and it costs the visit.
///
/// Stated here rather than left implicit because the whole deadline rests on
/// it: if a pedestal reached THE THRESHOLD, missing the house would cost a
/// detour rather than the chain.
#[test]
fn nothing_but_the_manse_reaches_the_threshold() {
    let by_town: Vec<&str> = gearmaster_engine::town::TOWNS
        .iter()
        .filter(|t| t.actions.iter().any(|a| a.opens() == Some("the-threshold")))
        .map(|t| t.id)
        .collect();
    assert_eq!(by_town, vec!["the-manse"], "another town grew a staircase");
    assert!(
        !gearmaster_engine::pedestal::DESTINATIONS.iter().any(|p| matches!(
            p.kind,
            gearmaster_engine::pedestal::Where::Dungeon(id) | gearmaster_engine::pedestal::Where::Siding { dungeon: id, .. }
            if id == "the-threshold"
        )),
        "an orb reaches the stair, so the Manse is no longer the only way in"
    );
    assert!(
        !gearmaster_engine::event::EVENTS.iter().any(|e| e.choices.iter().any(|c| {
            gearmaster_engine::event::every_outcome(&c.outcome).iter().any(|o| matches!(
                o,
                gearmaster_engine::event::Outcome::Enter(id)
                    | gearmaster_engine::event::Outcome::StartDungeon(id) if *id == "the-threshold"
            ))
        })),
        "a door on the road reaches the stair"
    );
    assert!(
        Action::CellarDoor.costs_the_visit(),
        "the cellar door stopped costing the town's one action, which is a different game"
    );
}

/// Exactly one door reveals the house, and exactly one choice at it.
#[test]
fn one_choice_in_the_game_puts_the_manse_on_the_map() {
    let mut setters: Vec<(&str, &str)> = Vec::new();
    for e in gearmaster_engine::event::EVENTS {
        for c in e.choices {
            if gearmaster_engine::event::every_outcome(&c.outcome).iter().any(|o| {
                matches!(o, gearmaster_engine::event::Outcome::RevealTown(id) if *id == "the-manse")
            }) {
                setters.push((e.id, c.label));
            }
        }
    }
    assert_eq!(
        setters,
        vec![("the-locked-gate", "Use the word")],
        "the set of ways into the house moved, and the deadline above is about the old one"
    );
}

/// The word the chain starts on can be come by before the astronomer stands.
#[test]
fn the_first_word_is_on_the_bar_before_the_first_door() {
    let r = gearmaster_engine::rumour::RUMOURS
        .iter()
        .find(|r| r.name == WRONG_STARS)
        .expect("authored");
    assert!(r.on_the_bar, "the chain's on-ramp left the bar");
    let first_pub = gearmaster_engine::town::TOWNS
        .iter()
        .filter(|t| matches!(t.unlock, gearmaster_engine::town::Unlock::Pinned))
        .filter(|t| t.actions.contains(&Action::Pub))
        .map(|t| t.after + 1)
        .min()
        .expect("a pinned town with a bar in it");
    let astronomer = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-astronomer")
        .expect("authored");
    assert!(
        first_pub <= astronomer.trigger.from(),
        "the first bar is at rung {} and the door it opens stands from rung {}",
        first_pub + 1,
        astronomer.trigger.from() + 1
    );
}

/// And the second word is handed over by the door the first one opens.
#[test]
fn the_astronomer_hands_over_what_the_gate_asks_for() {
    let e = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-astronomer")
        .expect("authored");
    assert!(
        e.choices.iter().any(|c| {
            c.label == "Hear him out"
                && gearmaster_engine::event::every_outcome(&c.outcome).iter().any(|o| {
                    matches!(o, gearmaster_engine::event::Outcome::Give(n) if *n == CELLAR)
                })
        }),
        "the astronomer stopped handing over the cellar word"
    );
    let gate = gearmaster_engine::event::EVENTS
        .iter()
        .find(|e| e.id == "the-locked-gate")
        .expect("authored");
    assert!(
        matches!(gate.trigger, gearmaster_engine::event::Trigger::Whispered { rumour, .. } if rumour == CELLAR),
        "the gate stopped waiting on the cellar word"
    );
}

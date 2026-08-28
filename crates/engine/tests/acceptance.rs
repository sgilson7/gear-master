//! E6, in one file, criterion by criterion.
//!
//! Thirteen claims the mission is finished when it can make. Most of them are
//! already proven somewhere else - that is what the rest of the suite is - and
//! this file exists so the answer to "is it done" is one command rather than a
//! reading of forty-six others. Where a criterion is proven elsewhere, the
//! assertion here names the file that proves it and checks the same fact from
//! its own side; where it is not, it is proven here.
//!
//! The one thing this file must not become is a summary. A test that asserts
//! `true` beside a comment saying somebody checked is worse than no test, so
//! every criterion below either measures something or names the mechanism it
//! is standing on.

mod common;

use gearmaster_engine::combat::{Difficulty, LADDER};
use gearmaster_engine::event::{every_outcome, Outcome, Requirement, EVENTS};
use gearmaster_engine::run::{Mode, Run};

const MAINSPRING: &str = "An Unwound Mainspring";

fn a_run(seed: u64) -> Run {
    let mut r = Run::seeded(seed);
    r.mode = Mode::Grinder;
    r.difficulty = Difficulty::Medium;
    common::build_full_loadout(&mut r);
    r
}

// ------------------------------------------------------------ 1. determinism

#[test]
fn e6_1_two_replays_of_a_seed_agree_about_everything_that_rolls() {
    // The three things in the mission that draw from the run's own PRNG: the
    // crucible's melt, the sealed bid's reserve, and the dispenser's gamble.
    // Combat has no RNG at all, which is the doctrine this rests on.
    let play = |seed: u64| -> Vec<String> {
        let mut out = Vec::new();
        let mut run = a_run(seed);
        run.gold = 100_000;

        // The dispenser.
        let d = EVENTS.iter().find(|e| e.id == "the-dispenser").unwrap();
        run.rung = d.at;
        if let Some(c) = d.choices.iter().find(|c| c.label == "Shake it") {
            run.take_choice(c);
            out.extend(run.take_receipt().unwrap_or_default());
        }

        // The sealed bid.
        let b = EVENTS.iter().find(|e| e.id == "the-sealed-bid").unwrap();
        run.rung = b.at;
        run.flags.push("slagworks-known");
        if let Some(c) = b.choices.iter().find(|c| c.label == "Name a figure") {
            run.take_choice_with(c, 3_000);
            out.extend(run.take_receipt().unwrap_or_default());
        }

        // The crucible.
        if let Some(id) = run.inventory().first().copied() {
            let melted = run.melt(id);
            out.push(format!("melt {:?}", melted.map(|m| run.registry.def(m).name)));
        }
        out
    };
    assert_eq!(play(0x51_51), play(0x51_51), "a seed did not replay");
    assert_ne!(play(0x51_51), play(0x99_99), "every seed rolls the same way");
}

// ----------------------------------------------------------- 2. no regression

#[test]
fn e6_2_the_shallow_ladder_did_not_move() {
    // Rungs 1-14 are where the casino corridor lives and where A1 could most
    // easily have done damage. The claim is about the *shape* of the shallow
    // end: a finished board still walks it.
    //
    // A *finished* board, not the preset. The preset is the deliberately blunt
    // reference build and it clears nine rungs of fifty by design - `two_runs`
    // walks it up the ladder precisely to prove the slow door opens for a build
    // that cannot earn the casino. Asking it to win fourteen fights is asking
    // it to stop being what it is for.
    let mut run = common::run_from(gearmaster_engine::share::A_WINNING_RUN);
    run.mode = Mode::Grinder;
    run.difficulty = Difficulty::Medium;
    for rung in 0..14usize {
        run.rung = rung;
        run.fight(&LADDER[rung]);
        let log = run.log.as_ref().expect("a fight");
        assert!(log.outcome == gearmaster_engine::combat::Outcome::Victory, "rung {} lost", rung + 1);
        run.settle();
        run.back_to_loadout();
    }
}

// --------------------------------------------------------- 4. the chain walks

#[test]
fn e6_4_both_roads_to_the_mainspring_are_open() {
    // Proven in full by `chain.rs` and `phase_two.rs`; checked here from the
    // other end - that the two payers exist and pay the same thing, which is
    // what makes a refused Herald survivable.
    let by_fight = EVENTS.iter().any(|e| {
        e.choices.iter().any(|c| {
            every_outcome(&c.outcome)
                .iter()
                .any(|o| matches!(o, Outcome::Step(b) if b.win == MAINSPRING))
        })
    });
    let by_courier = EVENTS.iter().any(|e| {
        e.choices.iter().any(|c| {
            every_outcome(&c.outcome)
                .iter()
                .any(|o| matches!(o, Outcome::Passenger { pays, .. } if *pays == MAINSPRING))
        })
    });
    assert!(by_fight && by_courier, "the chain has one road again");
}

// ------------------------------------------------------- 7. number anchoring

#[test]
fn e6_7_every_figure_in_the_mission_is_a_multiple_of_a_bounty() {
    // The gold rule as a lint over the whole table rather than one part of it.
    // A constant means one thing at rung four and something else at rung forty.
    fn times(o: &Outcome, out: &mut Vec<i32>) {
        match o {
            Outcome::Pay { times } | Outcome::BuyOff { times } => out.push(*times),
            _ => {}
        }
    }
    let mut seen = 0;
    for e in EVENTS {
        for c in e.choices {
            let mut v = Vec::new();
            for o in every_outcome(&c.outcome) {
                times(o, &mut v);
            }
            for t in v {
                seen += 1;
                assert!((0..=20).contains(&t), "{}: {} pays {} bounties", e.id, c.label, t);
            }
            if let Requirement::Purse { times } = c.requires {
                seen += 1;
                assert!((1..=20).contains(&times), "{}: {} costs {}", e.id, c.label, times);
            }
        }
    }
    // Nineteen today. The bar is that the road deals in bounties at all and in
    // more than a handful of places - not a pinned count, which would fail the
    // day somebody writes a door that pays in something else.
    assert!(seen >= 15, "only {} figures in the whole table deal in bounties", seen);
}

// --------------------------------------------------- 8. phase discipline held

/// Creatures the Switchyard has landed as frames and not yet dressed.
///
/// Re-pinned at the Switchyard's M6, and it empties at that mission's M9. It
/// is a *list* rather than a count because a list says which nine, and because
/// packing one creature without striking its name fails just as loudly as
/// adding a tenth.
///
/// The Unwinding left this at zero and the phase discipline is what put nine
/// back: Phase 2 ships a creature as a name, a band, a theme and the stats of
/// the ladder creature standing at that band, and Phase 4 packs the boards.
/// **Empty since M9.** It held the yard's nine for three milestones.
const UNDRESSED_UNTIL_THE_YARD_IS_PACKED: &[&str] = &[];

#[test]
fn e6_8_every_creature_in_the_game_is_dressed() {
    // The frame lint's own target, asserted from outside it. Phase 4's whole
    // job: red before, green after, and no scaffold board left anywhere.
    let naked: Vec<&str> =
        gearmaster_engine::bestiary::unpacked().iter().map(|f| f.name).collect();
    assert_eq!(
        naked, UNDRESSED_UNTIL_THE_YARD_IS_PACKED,
        "the undressed creatures are not the ones this test is waiting for"
    );
}

// ---------------------------------------------- 9, 10, 11, 12: the four rules

#[test]
fn e6_9_only_the_second_key_breaks_the_one_action_rule() {
    use gearmaster_engine::town::Action;
    let free: Vec<Action> =
        Action::EVERY.iter().copied().filter(|a| !a.costs_the_visit()).collect();
    assert_eq!(
        free,
        vec![Action::Pedestal, Action::County],
        "something other than the two things that are not doors stopped costing the visit. \
         THE HUNDRED's way down is the second, and it is the pedestal's exception rather \
         than a new one: the county is under the town, one trip per town for the whole run, \
         and charging a visit for it would make six towns six decisions the county always \
         loses. Anything else in this list is a bug"
    );
    // And the key itself, which is a thing rather than a door.
    let key = gearmaster_engine::relic::CRUSHABLES
        .iter()
        .find(|c| c.name == "the Second Key")
        .expect("the key exists");
    assert!(matches!(key.what, gearmaster_engine::relic::Crush::SecondKey));
}

#[test]
fn e6_10_a_granted_row_moves_nothing_that_was_already_placed() {
    let mut run = a_run(0x60_60);
    let before: Vec<(gearmaster_engine::piece::SlotKind, Vec<gearmaster_engine::piece::PieceId>)> =
        gearmaster_engine::piece::SlotKind::ALL
            .iter()
            .map(|&k| (k, run.loadout.slot(k).pieces()))
            .collect();
    run.owed_rows = 1;
    run.grow_slot(gearmaster_engine::piece::SlotKind::Chest);
    for (k, was) in before {
        assert_eq!(run.loadout.slot(k).pieces(), was, "growing the chest moved the {:?}", k);
    }
}

#[test]
fn e6_11_the_underwriter_eats_one_loss_and_only_one() {
    let mut run = a_run(0x11_11);
    run.rung = 20;
    run.apply_outcome(&Outcome::Underwrite, Requirement::None);
    assert!(run.underwritten_until.is_some(), "nobody underwrote anything");
    // It covers a window rather than for ever.
    let until = run.underwritten_until.unwrap();
    assert!(until > run.rung, "the cover expired before it started");
    assert!(
        until - run.rung <= gearmaster_engine::run::UNDERWRITTEN_FOR,
        "the cover outlasts the promise"
    );
}

#[test]
fn e6_12_scouting_is_knowledge_and_knowledge_is_not_a_stat() {
    let mut run = a_run(0x12_12);
    let before = run.player_stats();
    run.apply_outcome(&Outcome::Scout, Requirement::None);
    assert!(run.scouting, "the lens did nothing");
    assert_eq!(run.player_stats(), before, "scouting moved a number");
}

// ------------------------------------------------------------ the road itself

#[test]
fn e6_the_road_holds_everything_the_mission_promised() {
    // A census rather than a claim. If any of these numbers falls, something
    // was deleted rather than finished.
    // The Switchyard adds four doors, one dungeon, two words and two
    // destinations. Every one of these is an equality rather than a bound, so
    // the census fails on a deletion *and* on an arrival nobody wrote down.
    // Thirty-eight: the Switchyard's four, and the road past Francis - which
    // was a creature, a route-map label and a `past_the_top()` nothing called,
    // and had no door for four missions.
    //
    // **Thirty-nine since THE HUNDRED's F7.** One door, and it is the one the
    // county opens: a word carried up out of a field, and what the road tells
    // you back is what opens a box down there. The county's own eight are
    // `COUNTY_EVENTS` and are counted separately below, because a tile is not
    // a rung and a census that added them together would say the road had
    // grown by nine.
    assert_eq!(EVENTS.len(), 39, "the road lost a door");
    assert_eq!(gearmaster_engine::event::COUNTY_EVENTS.len(), 8, "the county lost a tile");
    assert_eq!(gearmaster_engine::town::TOWNS.len(), 6, "the road lost a town");
    assert_eq!(gearmaster_engine::dungeon::DUNGEONS.len(), 7, "the road lost a dungeon");
    assert_eq!(gearmaster_engine::rumour::RUMOURS.len(), 11, "the road lost a word");
    assert_eq!(gearmaster_engine::pedestal::DESTINATIONS.len(), 6, "an orb lost its place");
    assert_eq!(gearmaster_engine::bestiary::FRAMES.len(), 24, "a creature went missing");
}

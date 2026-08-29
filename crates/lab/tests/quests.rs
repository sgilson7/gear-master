//! The three named chains, dressed for an agent.
//!
//! `crates/engine/tests/quest.rs` checks the derivation against the tables and
//! `crates/trades/tests/quest.rs` checks the payment rule against two
//! trajectories. Neither can check the **translation**, because the derivation
//! is on one side of the boundary and the agent is on the other and only this
//! crate can see both. That is what these are.
//!
//! Run explicitly: `cargo test -p gearmaster-lab`.

use gearmaster_lab::quests::{self, Undressable};
use gearmaster_trades::quest::{Mark, Tier};

/// Every stop that crosses is one the console can recognise, and the finish
/// crosses or the quest is refused.
#[test]
fn the_named_chains_dress_or_say_why_they_cannot() {
    for (name, goal) in quests::NAMED {
        match quests::quest(name, *goal) {
            Ok(q) => {
                assert!(!q.stops.is_empty(), "{name} dressed to nothing");
                assert_eq!(
                    q.stops.last().map(|s| s.tier),
                    Some(Tier::Finish),
                    "{name} lost its finish and was handed over anyway"
                );
                assert_eq!(
                    q.stops.iter().filter(|s| s.tier == Tier::Finish).count(),
                    1,
                    "{name} has more than one finish"
                );
                // Windows survive the crossing, and none of them is impossible.
                for s in &q.stops {
                    assert!(s.window.0 <= s.window.1, "{name}: {:?} cannot be passed", s.mark);
                }
            }
            Err(Undressable::FinishIsNotOnTheScreen { mark, .. }) => {
                // One of the three, today. The county's chains are flags and
                // `View` carries no flags, deliberately - see the test below,
                // which is the one that will fail if that changes.
                assert_eq!(name, &"pathfinder_drover", "{name} lost its finish: {mark}");
            }
            Err(other) => panic!("{name}: {other:?}"),
        }
    }
}

/// **The one that cannot be trained as written, and why.**
///
/// §3.5 names three models. Two of them dress. The third is aimed at a chain of
/// THE HUNDRED being finished, which the engine records as a flag - and `View`
/// carries no flags, because a flag is bookkeeping and a player is shown no
/// list of them.
///
/// So this is a **console** question rather than a training one: either the
/// county tab grows a line saying which of its three chains are done, or
/// `pathfinder_drover` is aimed at something else that is on the screen. It is
/// the owner's call and it is not made here. What is done here is refusing to
/// hand out a chain with its head missing, which would train an agent on the
/// cheap tiers and never pay it for finishing.
#[test]
fn the_drover_chain_is_the_one_the_screen_cannot_show() {
    let e = quests::by_name("pathfinder_drover").expect_err(
        "the county's chains became visible on the screen. Good - now aim the \
         drover model at the finish rather than at the way down, and delete this.",
    );
    assert!(matches!(e, Undressable::FinishIsNotOnTheScreen { .. }), "{e:?}");
}

/// The road past Francis is the stair plus four more stops.
///
/// Derived rather than arranged: both routes to the mainspring wait on
/// `threshold-cleared`, so the longest chain in the game contains the shortest
/// one. That is why §C7 trains the threshold first, and it is also the sharpest
/// evidence that the three models are three chains rather than one copied.
#[test]
fn the_unwound_chain_contains_the_threshold_chain() {
    let t = quests::by_name("pathfinder_threshold").expect("dresses");
    let u = quests::by_name("pathfinder_unwound").expect("dresses");
    let theirs: Vec<&Mark> = u.stops.iter().map(|s| &s.mark).collect();
    for s in t.stops.iter().filter(|s| s.tier != Tier::Finish) {
        assert!(
            theirs.contains(&&s.mark),
            "the road past Francis stopped needing {:?}",
            s.mark
        );
    }
    assert!(u.stops.len() > t.stops.len(), "the two chains are the same length");
}

/// Both dressed chains carry the same deadline, and it is rung 25.
///
/// Not a coincidence and not typed anywhere: the Manse's gate stands on one
/// rung, everything the stair needs is due before it, and the road past Francis
/// needs the stair. So the rung by which a run has either bought a word at a bar
/// or lost the end of the game is **twenty-five**, and the derivation is what
/// says so.
#[test]
fn the_deadline_is_the_same_rung_for_both_chains_that_dress() {
    for name in ["pathfinder_threshold", "pathfinder_unwound"] {
        let q = quests::by_name(name).expect("dresses");
        let first = q.stops.first().expect("a chain has a first stop");
        assert_eq!(
            first.mark,
            Mark::Holding("A Word About the Wrong Stars".into()),
            "{name} no longer starts at the bar"
        );
        assert_eq!(
            first.window.1,
            24,
            "{name}: the word is due by rung {} and the walk in \
             crates/engine/tests/quest.rs measured rung 25",
            first.window.1 + 1
        );
    }
}

// --------------------------------------------- what a walked run can see
//
// `crates/trades/tests/quest.rs` checks the road description against a written
// list of verbs, which is what that crate can do: a walk from a fresh console
// meets `Fight` and nothing else, because a starter board loses rung one and a
// Grinder cannot slide below it.
//
// Here a packer is available, so the run gets as far as doors, gates and
// fountains and the question becomes what a *run* offers rather than what a
// list contains.

/// How many distinct road-step vectors a real run produces.
///
/// **It was one.** Every verb the pathfinder owns described to the same vector,
/// because the road agent had been handed the quartermaster's move description
/// and every road verb fell into its one leftover bucket. `--bin qmoves` is the
/// wider measurement - 1,341 verbs across four runs - and this is the cheaper
/// guard beside it.
///
/// **It goes up or it does not move.** Down means something narrowed
/// `pathfinder::describe` and the agent can see less than it could.
const ROAD_STEPS_TOLD_APART: usize = 18;

#[test]
fn a_walked_run_offers_road_steps_the_network_can_tell_apart() {
    use gearmaster_console::{Console, Difficulty, Mode};
    use gearmaster_lab::packers::Packer;
    use gearmaster_trades::env::{Step as RoadStep, Walking};
    use gearmaster_trades::pathfinder;

    let mut c = Console::start(0x1212, Mode::Grinder, Difficulty::Medium);
    let packer = Packer::named("control");
    let mut w = Walking::new(None, 120);
    let mut seen: Vec<[f32; pathfinder::MOVE]> = Vec::new();
    let mut packed_at = None;
    // Bounded twice: by the walking budget and by the trips round the loop.
    for _ in 0..120 {
        let ms = w.moves(&c);
        if ms.is_empty() {
            break;
        }
        let v = c.view();
        for s in &ms {
            let d = pathfinder::describe(&v, s);
            if !seen.iter().any(|x| x == &d) {
                seen.push(d);
            }
        }
        // Pack once a rung, then act - or the run loses rung one for ever and
        // the sample is one verb repeated.
        let at = if packed_at != Some(v.rung_shown)
            && ms.iter().any(|s| matches!(s, RoadStep::Pack))
        {
            packed_at = Some(v.rung_shown);
            ms.iter().position(|s| matches!(s, RoadStep::Pack)).expect("just checked")
        } else {
            ms.iter().position(|s| matches!(s, RoadStep::Press(_))).unwrap_or(0)
        };
        match &ms[at] {
            RoadStep::Pack => packer.pack(&mut c, 40),
            RoadStep::Press(verb) => {
                if !c.apply(*verb).ok {
                    break;
                }
            }
        }
        w.steps += 1;
    }
    assert_eq!(
        seen.len(),
        ROAD_STEPS_TOLD_APART,
        "the number of road steps a run offers that the network can tell apart \
         has moved. Up is the description getting richer and this constant owns \
         the re-measurement - run `--bin qmoves` for the wider figure. Down \
         means something narrowed `pathfinder::describe`."
    );
}

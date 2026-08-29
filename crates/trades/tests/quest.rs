//! What a quest pays, and what it refuses to pay.
//!
//! The gate this file exists for is `design/HANDOFF-two-agents.md` §C6's
//! second: *an agent that farms a cheap tier scores less than one that
//! finishes*, written as two hand-made trajectories rather than hoped for in
//! training.
//!
//! The trajectories are not invented. `crates/engine/tests/quest.rs` measured
//! four rungs of the Manse chain - displayed 26 to 29 - where a run is offered
//! both doors, takes the correct choice at both, puts the town on the map, and
//! cannot reach it because the gate stands on one rung and that rung is behind.
//! The farmer below is that run. It earns three of the four tiers honestly.
//!
//! The answer is not a weight. `Φ` is potential-based and is zeroed at the end
//! of every episode however it ended, so the tiers **telescope to nothing** and
//! there is no sum for a finish to have to beat. That is the claim, and these
//! are the sums.

use gearmaster_trades::quest::{End, Mark, Progress, Quest, Stop, Tier};

const GAMMA: f32 = 0.997;
const FINISH: f32 = 50.0;

/// The threshold chain as the derivation produces it.
///
/// Written out here rather than derived, because `gearmaster-trades` cannot
/// see the engine and must not - the derivation is checked against the tables
/// in `crates/engine/tests/quest.rs` and against this shape in
/// `crates/lab/src/bin/qquest.rs`. What this file is testing is the payment
/// rule, and a payment rule does not care where the chain came from.
fn threshold() -> Quest {
    let stop = |tier, mark, window| Stop { tier, mark, window };
    Quest {
        name: "pathfinder_threshold".into(),
        stops: vec![
            stop(Tier::Prerequisite, Mark::Holding("A Word About the Wrong Stars".into()), (7, 24)),
            stop(Tier::Offered, Mark::Offered("the-astronomer".into()), (17, 24)),
            stop(Tier::Chose, Mark::Holding("A Word About the Cellar".into()), (17, 24)),
            stop(Tier::Offered, Mark::Offered("the-locked-gate".into()), (22, 24)),
            stop(Tier::Chose, Mark::Gate("THE MANSE".into()), (25, 25)),
            stop(Tier::Chose, Mark::Entered("the-threshold".into()), (25, 49)),
            stop(Tier::Finish, Mark::Wearing("Threshold-Sighted".into()), (25, 49)),
        ],
    }
}

/// Play a trajectory: at each step, the set of stop indices the screen shows.
///
/// Returns the discounted return, which is the only number that matters -
/// `Σ γᵗ r_t`, the thing a Q value is an estimate of.
fn play(q: &Quest, steps: &[&[usize]], end: End) -> f32 {
    let mut p = Progress::new(q);
    let mut total = 0.0;
    let mut discount = 1.0;
    for (t, showing) in steps.iter().enumerate() {
        let last = t + 1 == steps.len();
        let e = if last { end } else { End::Running };
        let seen = |m: &Mark| {
            q.stops.iter().enumerate().any(|(i, s)| &s.mark == m && showing.contains(&i))
        };
        let paid = q.pay_by(&mut p, seen, GAMMA, e, FINISH);
        total += discount * paid.total();
        discount *= GAMMA;
    }
    total
}

/// Every stop up to and including `n`, which is how a run walks a chain.
fn upto(n: usize) -> Vec<usize> {
    (0..=n).collect()
}

// ------------------------------------------------------------- the two runs

/// The run that farms, and the run that finishes.
///
/// The farmer takes every cheap tier the road will give it and runs out of
/// road. The finisher takes the same ones and then finishes. The gate is that
/// the second scores more, and it does - by exactly the finish, because the
/// tiers are worth nothing at all once an episode is over.
#[test]
fn a_run_that_farms_scores_less_than_a_run_that_finishes() {
    let q = threshold();
    let farmer: Vec<Vec<usize>> = (0..=3).map(upto).collect();
    let farmer: Vec<&[usize]> = farmer.iter().map(|v| v.as_slice()).collect();
    let finisher: Vec<Vec<usize>> = (0..=6).map(upto).collect();
    let finisher: Vec<&[usize]> = finisher.iter().map(|v| v.as_slice()).collect();

    let farmed = play(&q, &farmer, End::Truncated);
    let finished = play(&q, &finisher, End::Terminated);
    assert!(
        finished > farmed,
        "farming four tiers scored {farmed} and finishing scored {finished}"
    );
    // And not merely less: nothing at all. The tiers are a hint about where to
    // look and they are not income.
    assert!(farmed.abs() < 1e-3, "the farmer banked {farmed}");
}

/// The tiers telescope to nothing over any complete episode.
///
/// This is the claim the whole design rests on and it is one line to check, so
/// it is checked for every prefix of the chain and for both ways of ending.
/// A prefix that banks anything is a prefix an agent will learn to stop at.
#[test]
fn the_tiers_sum_to_nothing_however_far_a_run_gets() {
    let q = threshold();
    for reached in 0..q.stops.len() {
        for end in [End::Terminated, End::Truncated] {
            let steps: Vec<Vec<usize>> = (0..=reached).map(upto).collect();
            let steps: Vec<&[usize]> = steps.iter().map(|v| v.as_slice()).collect();
            let banked = play(&q, &steps, end);
            // The finish arrives on the last step and is discounted like
            // anything else. What the tiers add is nothing, exactly, which is
            // why the expected figure has no term for them.
            let expected = if reached + 1 == q.stops.len() {
                FINISH * GAMMA.powi(reached as i32)
            } else {
                0.0
            };
            assert!(
                (banked - expected).abs() < 1e-2,
                "{end:?} after {reached} stops banked {banked}, wanted {expected}"
            );
        }
    }
}

/// **Truncation is where this leaks if it leaks.**
///
/// `qpack.rs:409` zeroes its potential when the agent says it is done and not
/// when the press budget ends the episode, so a truncated packing keeps
/// `γΦ(s_T)`. For items assembled that is nearly harmless. For a chain it is
/// exactly the farm, because a farming episode is precisely one that ends by
/// running out of road with the cheap tiers ticked.
///
/// So the two endings pay the same, and this is the assertion that says so.
#[test]
fn ending_by_running_out_of_road_pays_the_same_as_ending_properly() {
    let q = threshold();
    let steps: Vec<Vec<usize>> = (0..=4).map(upto).collect();
    let steps: Vec<&[usize]> = steps.iter().map(|v| v.as_slice()).collect();
    assert!(
        (play(&q, &steps, End::Truncated) - play(&q, &steps, End::Terminated)).abs() < 1e-4,
        "a run that stopped short was paid differently for how it stopped"
    );
}

/// A stop passed twice pays once.
///
/// The repeatable doors are the reason (§3.6): the county door is at six towns
/// and costs no visit, the chapel is at three and costs one. A run that stands
/// in front of the same door for twenty decisions has not done anything twenty
/// times.
#[test]
fn a_stop_the_road_offers_again_is_not_worth_anything_again() {
    let q = threshold();
    let mut p = Progress::new(&q);
    let seen = |m: &Mark| matches!(m, Mark::Offered(id) if id == "the-astronomer");
    let first = q.pay_by(&mut p, seen, GAMMA, End::Running, FINISH);
    assert_eq!(first.passed, 1);
    // Standing still is worth `γΦ − Φ`, which is a small negative and is the
    // discount doing its job rather than a payment. What it must never be is
    // positive: a door that paid again for being stood in front of again is
    // the farm the one-shot rule exists to stop.
    for _ in 0..20 {
        let again = q.pay_by(&mut p, seen, GAMMA, End::Running, FINISH);
        assert_eq!(again.passed, 0, "the same door paid twice");
        assert!(again.shaped <= 0.0, "standing still paid {}", again.shaped);
        assert!(again.finish == 0.0, "standing still finished the chain");
    }
    assert_eq!(p.passed(), 1);
}

/// The finish is paid once, and only for finishing.
#[test]
fn the_finish_is_paid_once_and_only_for_the_last_stop() {
    let q = threshold();
    let mut p = Progress::new(&q);
    let everything = |_: &Mark| true;
    let first = q.pay_by(&mut p, everything, GAMMA, End::Terminated, FINISH);
    assert_eq!(first.finish, FINISH, "finishing the chain paid nothing");
    let again = q.pay_by(&mut p, everything, GAMMA, End::Terminated, FINISH);
    assert_eq!(again.finish, 0.0, "the finish was paid twice");
}

/// Progress is two numbers a network can read, and it is monotone.
///
/// Without them the potential telescopes over something the agent cannot see,
/// and a shaped reward it cannot predict is noise rather than a hint.
#[test]
fn how_far_along_a_run_is_is_something_it_can_see() {
    let q = threshold();
    let mut p = Progress::new(&q);
    assert_eq!(q.features(&p), [0.0, 0.0]);
    let mut last = 0.0;
    for reached in 0..q.stops.len() {
        let showing = upto(reached);
        let seen = |m: &Mark| {
            q.stops.iter().enumerate().any(|(i, s)| &s.mark == m && showing.contains(&i))
        };
        q.observe_by(&mut p, seen);
        let f = q.features(&p);
        assert!(f[0] >= last, "progress went backwards");
        last = f[0];
    }
    assert_eq!(q.features(&p), [1.0, 1.0], "a finished chain does not read as finished");
}

/// The ordering of the tiers is the design, and it is not a matter of taste.
#[test]
fn a_dearer_tier_is_worth_more_than_a_cheaper_one() {
    assert!(Tier::Offered.weight() < Tier::Prerequisite.weight());
    assert!(Tier::Prerequisite.weight() < Tier::Chose.weight());
    // And the finish weighs nothing *in the potential*, because it is not a
    // hint about where to look - it is the thing being looked for, and it is
    // paid outside `Φ` so that ending an episode cannot cancel it.
    assert_eq!(Tier::Finish.weight(), 0.0);
}

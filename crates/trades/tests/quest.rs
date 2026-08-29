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
    let stop = |tier, mark, window| Stop { tier, mark, by: Vec::new(), doors: Vec::new(), window };
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

/// And it is paid on the **transition**, so a stop passed out of order after
/// the finish does not pay for the finish again.
///
/// Not reachable on the Manse chain, whose last stop needs every other one. It
/// is reachable on any chain where a run might already be holding something the
/// chain asks for later, and "the finish is only worth this much once" is the
/// load-bearing half of the whole design.
#[test]
fn a_stop_passed_after_the_finish_does_not_pay_for_the_finish_again() {
    let q = threshold();
    let mut p = Progress::new(&q);
    let last = q.stops.len() - 1;
    let only_the_finish = |m: &Mark| *m == q.stops[last].mark;
    let paid = q.pay_by(&mut p, only_the_finish, GAMMA, End::Running, FINISH);
    assert_eq!(paid.finish, FINISH, "reaching the last stop paid nothing");
    // Now an earlier stop turns up. Something was newly passed, and the chain
    // is still finished - which is exactly the shape that paid twice.
    let an_earlier_one = |m: &Mark| *m == q.stops[0].mark;
    let after = q.pay_by(&mut p, an_earlier_one, GAMMA, End::Running, FINISH);
    assert_eq!(after.passed, 1, "the earlier stop was not passed");
    assert_eq!(after.finish, 0.0, "the finish was paid a second time");
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

// ------------------------------------------- what the road agent can see
//
// Not about quests, and here because this is the file that found it: a chain
// the pathfinder is paid along is worth nothing if the pathfinder cannot tell
// which verb it is pressing.

use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_trades::env::Step as RoadStep;
use gearmaster_trades::pathfinder;

/// Every kind of road step describes to a vector of its own.
///
/// **They all described to one.** The road agent had been handed
/// `feature::mv`, which is the quartermaster's move description: a one-hot over
/// eight board-and-shop shapes, `_ => 8` for everything else, and every field
/// after it about a *piece*, which no road verb has. So the network's action
/// space was "pack" against "a road verb", and which road verb was the order
/// the console listed them in.
///
/// That is why every road policy this repo ever called learned reached about
/// the rung the one that presses the first legal thing reaches.
/// `analysis/the-two-trades.md` Q5.1 said "the Q network is not what is
/// deciding" and put it down to the packer; the packer was not the reason.
///
/// Checked against a written list rather than a walk. A walk from a fresh
/// console meets one verb - `Fight`, over and over, because a starter board
/// loses rung one - so counting what a walk *happens* to meet measures the walk.
/// `crates/lab/tests/quests.rs` does the walked version, where a packer can get
/// the run as far as a door.
#[test]
fn every_kind_of_road_step_looks_different_to_the_network() {
    let c = Console::start(0x1212, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    let steps = [
        RoadStep::Pack,
        RoadStep::Press(Verb::Fight),
        RoadStep::Press(Verb::FightParty),
        RoadStep::Press(Verb::Answer { choice: 0 }),
        RoadStep::Press(Verb::AnswerWith { choice: 0, figure: 1 }),
        RoadStep::Press(Verb::Town { door: gearmaster_console::Door::Pub }),
        RoadStep::Press(Verb::WalkOn),
        RoadStep::Press(Verb::ThrowPoints { exit: 0 }),
        RoadStep::Press(Verb::Leave),
        RoadStep::Press(Verb::Out),
        RoadStep::Press(Verb::Drink),
        RoadStep::Press(Verb::DrinkChoosing { class: 0 }),
        RoadStep::Press(Verb::Double { class: 0 }),
    ];
    let mut seen: Vec<[f32; pathfinder::MOVE]> = Vec::new();
    for s in &steps {
        let d = pathfinder::describe(&v, s);
        assert!(
            !seen.iter().any(|x| x == &d),
            "{s:?} describes identically to something before it"
        );
        seen.push(d);
    }
    assert_eq!(seen.len(), steps.len());
}

/// And two of one kind differ, which is the half a one-hot alone would miss.
///
/// A chain is made of *which* answer was taken at a door, and
/// `HANDOFF-two-agents.md` §3.6 is about nothing else - so an agent that can
/// tell answering from fighting and not the first answer from the second has
/// been given half a representation.
#[test]
fn two_answers_and_two_town_doors_do_not_look_the_same() {
    let c = Console::start(0x1212, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    let d = |s| pathfinder::describe(&v, &s);
    assert_ne!(
        d(RoadStep::Press(Verb::Answer { choice: 0 })),
        d(RoadStep::Press(Verb::Answer { choice: 1 })),
        "two answers at one door describe identically"
    );
    assert_ne!(
        d(RoadStep::Press(Verb::Town { door: gearmaster_console::Door::Pub })),
        d(RoadStep::Press(Verb::Town { door: gearmaster_console::Door::Chapel })),
        "two doors at one gate describe identically"
    );
    // The county is free and every other door spends the town's one action,
    // which is the fact `HANDOFF-two-agents.md` §3.6 turns on.
    assert_ne!(
        d(RoadStep::Press(Verb::Town { door: gearmaster_console::Door::County })),
        d(RoadStep::Press(Verb::Town { door: gearmaster_console::Door::Chapel })),
        "a free door and one that costs the visit describe identically"
    );
}

/// And packing is not the only action with a shape of its own any more.
///
/// It was: `Pack` described as all zeros, borrowed from the packing side's
/// `Done`, while every road verb shared one vector. So the only choice the
/// network could really express was pack-or-not, and a trained one pressed
/// `Pack` 320 times out of 320.
#[test]
fn packing_is_a_kind_like_any_other() {
    let c = Console::start(0x1212, Mode::Grinder, Difficulty::Medium);
    let v = c.view();
    let packing = pathfinder::describe(&v, &RoadStep::Pack);
    assert_ne!(packing, [0.0; pathfinder::MOVE], "`Pack` is the all-zero action again");
    let pressing = pathfinder::describe(&v, &RoadStep::Press(Verb::Fight));
    assert_ne!(packing, pressing, "packing and fighting became the same vector");
}

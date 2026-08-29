//! Both episodes run, and both end.
//!
//! Sub-second, and neither fights anything: what is checked is that the two
//! environments are well-formed - every move offered is accepted, an episode
//! terminates, and a seed replays. The horizons Q0 measured are asserted here
//! so a change that makes an episode ten times longer fails rather than
//! quietly costs a training run.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_trades::env::{Goal, Move, Packing, Step, Walking};

fn fresh() -> Console {
    Console::start(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium)
}

/// A deterministic pick, so an episode replays without an RNG dependency.
fn pick<T>(v: &[T], n: usize) -> &T {
    &v[n % v.len()]
}

#[test]
fn a_packing_episode_runs_and_ends() {
    let mut c = fresh();
    let mut e = Packing::new(60);
    let mut taken = 0;
    while let Some(&m) = e.moves(&c).first() {
        assert!(e.step(&mut c, m), "the environment offered {:?} and it was refused", m);
        taken += 1;
        if e.finished {
            break;
        }
        assert!(taken <= 60, "it did not end");
    }
    assert!(taken > 0, "it offered nothing at all");
}

#[test]
fn done_is_always_on_offer_and_always_ends_it() {
    // Without it a packer dithers: a step cost alone teaches it to press the
    // cheapest key rather than to stop.
    let mut c = fresh();
    let mut e = Packing::new(60);
    assert!(e.moves(&c).contains(&Move::Done));
    e.step(&mut c, Move::Done);
    assert!(e.finished);
    assert!(e.moves(&c).is_empty(), "an episode that has ended offers nothing");
}

#[test]
fn a_packing_episode_stays_inside_the_horizon_q0_measured() {
    // Q0: the control makes thirteen decisions a typical episode and
    // forty-seven at the worst. Sixty is the budget; a walk that spends it all
    // has gone wrong in a way the reward should be able to see.
    let mut c = fresh();
    let mut e = Packing::new(60);
    let mut n = 0;
    loop {
        let ms = e.moves(&c);
        if ms.is_empty() {
            break;
        }
        // Take a placement where there is one, else stop - the shortest
        // sensible walk, which is what bounds the horizon.
        let m = ms
            .iter()
            .find(|m| matches!(m, Move::Press(gearmaster_console::Verb::Place { .. })))
            .copied()
            .unwrap_or(Move::Done);
        e.step(&mut c, m);
        n += 1;
        if e.finished {
            break;
        }
    }
    assert!(n <= 60, "{} decisions, and Q0 measured thirteen", n);
}

#[test]
fn a_walking_episode_runs_and_offers_pack() {
    let c = fresh();
    let e = Walking::new(None, 600);
    let ms = e.moves(&c);
    assert!(!ms.is_empty());
    assert!(ms.contains(&Step::Pack), "there is a tray, so packing is on offer");
}

#[test]
fn a_goal_is_something_the_screen_can_answer() {
    // A goal the pathfinder cannot recognise is a goal it cannot aim at, so
    // `met` reads the view and nothing else.
    let c = fresh();
    assert!(!Walking::new(Some(Goal::Rung(10)), 600).met(&c));
    assert!(Walking::new(Some(Goal::Rung(0)), 600).met(&c), "rung one is past rung zero");
    assert!(!Walking::new(Some(Goal::Dungeon("the-crevice".into())), 600).met(&c));
}

#[test]
fn two_walks_of_one_seed_are_the_same_walk() {
    let mut a = fresh();
    let mut b = fresh();
    for (ca, cb) in [(&mut a, &mut b)] {
        let mut ea = Walking::new(None, 40);
        let mut eb = Walking::new(None, 40);
        for i in 0..40 {
            let (ma, mb) = (ea.moves(ca), eb.moves(cb));
            assert_eq!(ma, mb);
            if ma.is_empty() {
                break;
            }
            let sa = pick(&ma, i).clone();
            if let Step::Press(v) = sa {
                ca.apply(v);
                cb.apply(v);
            }
            ea.steps += 1;
            eb.steps += 1;
        }
    }
    assert_eq!(a.screen(), b.screen());
}

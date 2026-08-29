//! Losing is worth nothing, and in Rogue it is worth less than that.
//!
//! The owner's rule for the Rogue pair: **a loss must provide negative or no
//! value.** That is not a weight to tune, it is a property of the reward, and
//! this file is where it is stated as one.
//!
//! It was not true. `Reward` paid `+1` every time the rung went up, and
//! `--bin qrogue` measured that over six seeds at **6.37 payments per rung
//! actually reached** in Grinder and 5.04 in Rogue - the agent being paid over
//! and over for ground it had already been paid for. In Grinder that is a run
//! oscillating against a wall it cannot pass. In Rogue it is a run being wiped
//! to the bottom and climbing the first ten rungs again, which is to say it is
//! a reward for dying.
//!
//! The road pays for **new ground** now and for nothing else, and `best`
//! survives a wipe on purpose: the run that replaces a dead one has not been
//! anywhere, and re-walking the dead one's road is not progress.
//!
//! Everything here drives `Reward::value` rather than a played game, so a
//! trajectory is something written down rather than something a console has to
//! be steered into.

use gearmaster_trades::pathfinder::Reward;

/// One step: the rung the screen shows, whether the run was wiped, whether the
/// fight was lost.
type Step = (usize, bool, bool);

fn run(rogue: bool, steps: &[Step]) -> f32 {
    let mut r = Reward::new(rogue);
    steps.iter().map(|&(rung, wiped, lost)| r.value(rung, wiped, lost, false)).sum()
}

/// Climb to `n` a rung at a time, winning every fight.
fn climb(to: usize) -> Vec<Step> {
    (2..=to).map(|r| (r, false, false)).collect()
}

// ------------------------------------------------------- the owner's rule

/// **A loss is never worth anything.** Neither mode, no trajectory.
#[test]
fn a_lost_fight_is_never_worth_more_than_not_losing_it() {
    for rogue in [false, true] {
        for to in [2usize, 5, 12, 30] {
            let clean = run(rogue, &climb(to));
            // The same climb with a defeat inserted at every rung on the way.
            let mut with_losses: Vec<Step> = Vec::new();
            for &(rung, _, _) in &climb(to) {
                with_losses.push((rung.saturating_sub(1), false, true));
                with_losses.push((rung, false, false));
            }
            let lossy = run(rogue, &with_losses);
            assert!(
                lossy < clean,
                "rogue={rogue} to rung {to}: losing on the way scored {lossy}, \
                 clean scored {clean}"
            );
        }
    }
}

/// **And it costs more in Rogue**, because it takes more off you.
#[test]
fn a_loss_costs_more_in_rogue_than_in_grinder() {
    let one_loss = [(1usize, false, true)];
    assert!(
        run(true, &one_loss) < run(false, &one_loss),
        "a Rogue loss is not dearer than a Grinder one"
    );
}

/// **Re-climbing pays nothing.** The measured farm, closed.
///
/// A Grinder knocked off rung ten and climbing back to ten has gone nowhere,
/// and the reward says so. Before this it said `+1` a rung, every time round.
#[test]
fn climbing_a_rung_this_run_has_already_stood_on_is_worth_nothing() {
    for rogue in [false, true] {
        let once = run(rogue, &climb(10));
        // Up to ten, knocked back to five, up to ten again - four more rungs
        // of climbing and not one of them new.
        let mut oscillating = climb(10);
        oscillating.push((5, false, true));
        oscillating.extend((6..=10).map(|r| (r, false, false)));
        let twice = run(rogue, &oscillating);
        assert!(
            twice < once,
            "rogue={rogue}: falling and re-climbing scored {twice} against {once} for \
             climbing once"
        );
    }
}

// ------------------------------------------------------------- the wipe

/// **A wipe is the worst thing that can happen, and it is not a bigger loss.**
///
/// A Rogue run out of lives is *replaced*: rung one, gold back to what it
/// started with, board gone. `Console::over` never sees it because the
/// replacement already has its lives back, so nothing else in the loop can tell.
#[test]
fn a_wipe_costs_more_than_the_loss_that_caused_it() {
    let lost = run(true, &[(4, false, true)]);
    let wiped = run(true, &[(1, true, false)]);
    assert!(wiped < lost, "a wipe scored {wiped} against a plain loss at {lost}");
}

/// **And the replacement run is not paid for the dead one's road.**
///
/// This is the whole of "losing provides no value". A run that climbs to ten,
/// dies, and climbs to ten again has been to rung ten once as far as the
/// reward is concerned - so dying on purpose to re-farm the early rungs earns
/// strictly less than never dying, rather than the same or more.
#[test]
fn a_run_that_dies_and_re_climbs_earns_nothing_for_the_second_climb() {
    let once = run(true, &climb(10));
    let mut died_and_again = climb(10);
    died_and_again.push((1, true, false));
    died_and_again.extend(climb(10));
    let twice = run(true, &died_and_again);
    assert!(
        twice < once,
        "dying and re-climbing scored {twice}, climbing once scored {once}"
    );
    // And the second climb itself is worth nothing at all beyond its step
    // costs - which is the sharper statement and the one that closes the farm.
    let mut r = Reward::new(true);
    for &(rung, wiped, lost) in &climb(10) {
        r.value(rung, wiped, lost, false);
    }
    r.value(1, true, false, false);
    let second: f32 = climb(10).iter().map(|&(g, w, l)| r.value(g, w, l, false)).sum();
    assert!(
        second < 0.0,
        "the second climb to rung ten paid {second}, which is not 'no value'"
    );
}

/// Going somewhere genuinely new after a wipe still pays.
///
/// The rule is that losing is worth nothing, not that a run which has died is
/// worth nothing for ever. A replacement that gets *past* where the dead one
/// stopped is doing something no run in this episode has done.
#[test]
fn a_replacement_run_is_paid_for_ground_the_dead_one_never_reached() {
    let mut r = Reward::new(true);
    for &(rung, w, l) in &climb(10) {
        r.value(rung, w, l, false);
    }
    r.value(1, true, false, false);
    for &(rung, w, l) in &climb(10) {
        r.value(rung, w, l, false);
    }
    let past = r.value(11, false, false, false);
    assert!(past > 0.0, "rung eleven, which no run this episode had reached, paid {past}");
}

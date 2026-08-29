//! The two judges rank boards differently, or there is no Rogue packer.
//!
//! R3's gate. **Passing `Mode::Rogue` where `qpack` passes `Mode::Grinder`
//! produces the same packer**, because the packer's reward is a fight and
//! `crates/engine/src/combat.rs` never names `Mode`. A fight is a pure function
//! of two boards, so the mode cannot reach the reward and training against a
//! Rogue run changes nothing the network learns.
//!
//! `lab::scoring` is the reward that can tell them apart: a board is judged
//! against a window of the rungs ahead, Grinder averages the window and Rogue
//! averages it and then pays again for the worst thing in it.
//!
//! The claim this file has to make is not that the numbers differ - adding a
//! term to one of two rewards will always do that - but that they **order**
//! boards differently. If every board is ranked the same way by both, the Rogue
//! judge is the Grinder judge with extra steps and R3 has not happened.
//!
//! ## How often, and the number is small
//!
//! `--bin qjudge` over six seeds and six rungs: **4 of 406 pairs** invert. The
//! mechanism is why. `Judge::Rogue` adds `worst.min(0.0)`, so on a board that
//! wins every rung of its window the two judges return the **same number** -
//! and a board the pilot built usually does win its whole window.
//!
//! That is not a weak result, it is a concentrated one: the two differ exactly
//! on the boards where a run would die, and agree everywhere a run is safe.
//! But it does mean an inversion has to be looked for rather than stumbled
//! into, and the first version of this file looked at four boards, found none
//! of them losing anywhere, and concluded the judges were the same.
//!
//! So there are two tests. The **property** is the strong claim and holds on
//! any board. The **inversion** is the gate, and it names the pair that
//! reproduces it rather than hoping a sample contains one.

use gearmaster_console::{Difficulty, Mode};
use gearmaster_lab::curriculum;
use gearmaster_lab::scoring::{self, Judge};

/// A board a run actually had at a rung, with the window it faces.
struct Situation {
    what: String,
    window: Vec<f32>,
    grinder: f32,
    rogue: f32,
}

/// Walked once. Each walk is a whole control run of shops and fights.
fn situations() -> &'static Vec<Situation> {
    static ONCE: std::sync::OnceLock<Vec<Situation>> = std::sync::OnceLock::new();
    ONCE.get_or_init(walk)
}

/// The pair `--bin qjudge` finds the inversion in, plus one seed either side.
///
/// Seed `0xf141...` at rung five has a window of `+1.70 +1.70 -1.40` - it beats
/// the next two and falls over on the third - and at rung twenty a window of
/// `-1.22 +1.56 +1.54`, which loses where it stands and improves. Grinder
/// prefers the first (0.67 against 0.63) and Rogue prefers the second (-0.59
/// against -0.73). Widen the grid in `qjudge` rather than here: this is a gate,
/// and a gate that walks thirty-six runs is a gate nobody runs.
fn walk() -> Vec<Situation> {
    let mut out = Vec::new();
    for seed in [0xF1418AF3EDF965FDu64, 0x5EED1234] {
        for rung in [4usize, 19] {
            let (c, walked) = curriculum::walk_to(seed, Mode::Grinder, Difficulty::Medium, rung);
            if !walked.arrived {
                continue;
            }
            let (stats, items) = c.board_for_scoring();
            if items.is_empty() {
                continue;
            }
            out.push(Situation {
                what: format!("{seed:#x} at rung {}", rung + 1),
                window: scoring::window(stats, &items, rung),
                grinder: scoring::score(stats, &items, rung, Judge::Grinder),
                rogue: scoring::score(stats, &items, rung, Judge::Rogue),
            });
        }
    }
    out
}

/// **The property, and it holds on every board.**
///
/// A board that wins its whole window is priced the same by both judges - there
/// is nothing to be afraid of, so being afraid costs nothing. A board that
/// loses anywhere in it is worth strictly less to a Rogue, because in Rogue the
/// losing rung is the one that ends the run.
///
/// This is what `Judge::Rogue` *is*, stated from the outside, and it is the
/// half that does not depend on which boards a sample happened to contain.
#[test]
fn the_rogue_judge_charges_for_a_losing_rung_and_the_grinder_one_does_not() {
    let all = situations();
    assert!(all.len() >= 3, "only {} boards were walked, so this proves little", all.len());
    let mut safe = 0;
    let mut risky = 0;
    for s in all {
        let worst = s.window.iter().cloned().fold(f32::INFINITY, f32::min);
        if worst < 0.0 {
            risky += 1;
            assert!(
                s.rogue < s.grinder,
                "{}: window {:?} has a loss in it and the two judges agree ({} vs {})",
                s.what,
                s.window,
                s.grinder,
                s.rogue
            );
        } else {
            safe += 1;
            assert!(
                (s.rogue - s.grinder).abs() < 1e-4,
                "{}: window {:?} wins throughout and the judges differ ({} vs {})",
                s.what,
                s.window,
                s.grinder,
                s.rogue
            );
        }
    }
    assert!(risky > 0, "no walked board loses anywhere in its window, so nothing was tested");
    assert!(safe > 0, "every walked board loses somewhere, so the safe half was not tested");
}

/// **The gate.** Some pair of real boards is ranked in opposite orders.
#[test]
fn the_two_judges_disagree_about_which_board_is_better() {
    let all = situations();
    assert!(all.len() >= 3, "only {} boards were walked, so this proves little", all.len());
    let mut inverted: Vec<String> = Vec::new();
    for (i, a) in all.iter().enumerate() {
        for b in all.iter().skip(i + 1) {
            let g = a.grinder.partial_cmp(&b.grinder).expect("real numbers");
            let r = a.rogue.partial_cmp(&b.rogue).expect("real numbers");
            if g.is_eq() || r.is_eq() || g == r {
                continue;
            }
            inverted.push(format!(
                "{} ({:+.2}/{:+.2}) against {} ({:+.2}/{:+.2})",
                a.what, a.grinder, a.rogue, b.what, b.grinder, b.rogue
            ));
        }
    }
    assert!(
        !inverted.is_empty(),
        "no pair of {} real boards was ranked differently by the two judges. Either \
         the Rogue judge has stopped charging for a losing rung - which the \
         property test above would also catch - or these particular boards all win \
         their whole windows, in which case widen the grid in `walk` rather than \
         concluding the judges are the same. `--bin qjudge` over six seeds and six \
         rungs finds four inversions in 406 pairs.\n\nBoards: {:#?}",
        all.len(),
        all.iter().map(|s| (&s.what, &s.window, s.grinder, s.rogue)).collect::<Vec<_>>()
    );
}

/// A board that wins now and dies next rung is priced as what it is.
///
/// The shape the whole thing turns on: combat is deterministic, so the risk in
/// Rogue is not a dice roll - it is that the screen shows the *next* creature
/// and nothing beyond it. A board tuned to what it can see is a board betting
/// four lives on the next card.
#[test]
fn a_board_that_falls_over_one_rung_later_is_worth_less_to_a_rogue() {
    let all = situations();
    let spiky = all
        .iter()
        .filter(|s| s.grinder > 0.0 && s.rogue < 0.0)
        .min_by(|a, b| a.rogue.partial_cmp(&b.rogue).expect("real numbers"));
    let s = spiky.unwrap_or_else(|| {
        panic!(
            "no walked board wins on average and loses somewhere in its window, so \
             the window is not doing anything. Boards: {:?}",
            all.iter().map(|s| (&s.what, &s.window, s.grinder, s.rogue)).collect::<Vec<_>>()
        )
    });
    assert!(
        s.rogue < s.grinder,
        "{}: the Rogue judge did not price the worst rung in the window",
        s.what
    );
}

/// An empty board is the one thing both judges agree about without asking.
#[test]
fn no_board_at_all_is_not_a_bad_board_but_no_answer() {
    use gearmaster_engine::stats::Stats;
    for judge in [Judge::Grinder, Judge::Rogue] {
        assert_eq!(scoring::score(Stats::ZERO, &[], 4, judge), -1.5);
    }
}

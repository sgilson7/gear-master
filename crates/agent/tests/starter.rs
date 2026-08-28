//! The control, played through the console.
//!
//! `starter` is the baseline printer's `starter` row put through the economy
//! for the first time in this repo's life: seat what will sit, fight, repeat.
//! Its clear rate is meant to be dismal - it is the number every later
//! milestone is measured against, and A0's §1 table says it is zero today.

use gearmaster_agent::starter;
use gearmaster_console::{Difficulty, Mode};

#[test]
fn the_control_plays_a_run_and_stops_somewhere_honest() {
    let e = starter(0x5EED_1234_ABCD_0001, Mode::Grinder, Difficulty::Medium, 4_000);
    println!(
        "starter: rung {}, {} board clears, {} game clears, {} presses - {}",
        e.rung_reached, e.board_clears, e.game_clears, e.presses, e.why
    );
    assert!(e.presses > 0, "it pressed nothing at all");
    assert!(e.rung_reached >= 1);
}

#[test]
fn the_control_is_the_same_run_every_time() {
    let a = starter(0x6060, Mode::Grinder, Difficulty::Medium, 2_000);
    let b = starter(0x6060, Mode::Grinder, Difficulty::Medium, 2_000);
    assert_eq!(a, b, "the environment has noise in it");
}


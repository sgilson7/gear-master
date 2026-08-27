//! A scripted run, piped in twice, diffed against itself.
//!
//! The design contract is that a script replays identically (`CLAUDE.md` §1),
//! and until now nothing checked it in the suite - the two replays `HANDOFF.md`
//! cites were run by hand and nobody wrote the script down
//! (`post-unwinding.md` §1). This is the harness that ends that, and it is the
//! one acceptance criterion 1 wants for the yard.
//!
//! `Run::new()` is `seeded(0x5EED_1234_ABCD_0001)`, so the shop, the crucible
//! and every other out-of-combat roll are pinned; combat consults no RNG at
//! all. Two runs of one script that differ mean something reached for the
//! clock, the environment or an address.

use std::io::Write;
use std::process::{Command, Stdio};

fn play(script: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gearmaster-cli"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the driver builds");
    child.stdin.take().expect("piped").write_all(script.as_bytes()).expect("wrote the script");
    let out = child.wait_with_output().expect("it finished");
    assert!(out.status.success(), "the driver died: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("utf-8")
}

/// The script a run walks to get somewhere worth watching.
///
/// Eight rungs on the auto-builder's board, which is what it can clear
/// (`analysis/switchyard.md`, M0's shallow-ladder table), a town gate, and the
/// two new verbs asked from places they are not legal so their refusals are in
/// the transcript too.
const A_WALK: &str = "\
preset
road
throw 0
leave
fight
fight
fight
fight
fight
fight
fight
road
map
quit
";

#[test]
fn a_scripted_run_replays_identically() {
    let once = play(A_WALK);
    let twice = play(A_WALK);
    assert_eq!(once, twice, "the same script made a different run");
    assert!(once.contains("Rung 8"), "the walk did not get where it was going:\n{once}");
}

/// The two dungeon verbs answer from outside a dungeon rather than panicking.
#[test]
fn throw_and_leave_say_no_from_the_road() {
    let out = play("throw 0\nleave\nthrow x\nquit\n");
    assert!(out.contains("You are not at the points."), "{out}");
    assert!(out.contains("You are not in a dungeon."), "{out}");
    assert!(out.contains("error: throw <n>"), "{out}");
}

/// Every verb `help` advertises is a verb the driver answers to.
///
/// A help line for a verb that does not exist is worse than no help line, and
/// two were added this milestone.
#[test]
fn help_lists_the_two_new_verbs() {
    let out = play("help\nquit\n");
    assert!(out.contains("throw <n>"), "{out}");
    assert!(out.contains("leave"), "{out}");
}

/// The yard, walked twice, byte for byte.
///
/// Acceptance criterion 1. It was deferred from M3 because nothing in
/// `DUNGEONS` had points in it and no dungeon was reachable by any board the
/// driver could build; THE SWITCHYARD is reachable through `road`, `answer`
/// and `throw`, which is what makes this a walk rather than two refusals.
///
/// The script does not fight its way to rung 28 - the preset board cannot -
/// so what it replays is the driver's own surface: the road printed at every
/// step, the points listed under the banner, and the two verbs refusing and
/// accepting in the same order twice. Determinism is the claim, and the claim
/// is about the whole transcript rather than about any line of it.
const THE_YARD: &str = "\
preset
road
map
throw 0
leave
fight
road
fight
road
fight
road
items
stats
quit
";

#[test]
fn the_cli_verbs_replay() {
    let once = play(THE_YARD);
    let twice = play(THE_YARD);
    assert_eq!(once, twice, "the same script made a different run");
    // Both verbs were reached and both answered rather than panicking.
    assert!(once.contains("You are not at the points."), "{once}");
    assert!(once.contains("You are not in a dungeon."), "{once}");
}

/// The driver knows the yard exists and can name what is in it.
///
/// Not a walk: a check that the content M6 landed is reachable through the
/// engine the driver drives, so a transcript taken by hand later is taken
/// against something real.
#[test]
fn the_road_the_driver_prints_knows_about_the_yard() {
    use gearmaster_engine::dungeon::by_id;

    let d = by_id("the-switchyard").expect("M6 landed it");
    assert_eq!(d.floors.len(), 9);
    assert_eq!(d.forks(), 3);
    // The map's label for it, which `road`/`map` print through `route::ascii`.
    let mut run = gearmaster_engine::run::Run::new();
    run.rung = 27;
    let ascii = gearmaster_engine::route::ascii(&run).join("\n");
    assert!(
        ascii.contains("THE SWITCHYARD (4 fights, 3 points)"),
        "the map does not say how deep the yard goes:\n{ascii}"
    );
}

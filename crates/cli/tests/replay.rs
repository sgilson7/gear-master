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
///
/// The whole of what can be replayed through the CLI today: no dungeon in
/// `DUNGEONS` has points in it, and none of them is reachable by any board the
/// driver can build from its own verbs - THE CREVICE's door is the shrine fork
/// on rung 10 and the preset board loses rung 9. The yard is what makes this
/// script a walk instead of two refusals, and the yard is M6.
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

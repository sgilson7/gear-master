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
    // Two since A7: the yard is islands and the throat's fork is gone.
    assert_eq!(d.forks(), 2);
    // The map's label for it, which `road`/`map` print through `route::ascii`.
    let mut run = gearmaster_engine::run::Run::new();
    run.rung = 27;
    let ascii = gearmaster_engine::route::ascii(&run).join("\n");
    assert!(
        ascii.contains("THE SWITCHYARD (4 fights, 2 points)"),
        "the map does not say how deep the yard goes:\n{ascii}"
    );
}

// --------------------------------------------------------- THE HUNDRED, F2

/// A trip down and back, from the one town a scripted board can reach.
///
/// **The exit criterion asked for all three pinned towns and this is one.**
/// The wall is the Switchyard's M3 wall and it has not moved: no board the
/// driver can build from its own verbs clears rung 9, and Kettleworks' gate is
/// after rung 17. `sandbox` grants every component and `preset` is still the
/// auto-builder, which wins eight fights and then oscillates on the
/// Whisperling. The three-town walk that criterion is about is
/// `county::every_town_lets_you_down_at_its_own_mouth`, which does all
/// **six**; what belongs here is the half that only a driver can prove - that
/// a script of these verbs, piped in twice, comes out the same.
///
/// Seven fights to Sump Bottom's gate, down the steps, and every way a move
/// can end in the transcript: an edge, fresh ground, a tile walked over twice,
/// the last move, and one asked after the trip is spent.
///
/// `answer 0` after each move because a tile that asks something is answered
/// **there** - nothing walks away from an open question, which is the bug
/// playing it found. An `answer` with nothing asking says so and costs
/// nothing, so the script is the same script whichever tiles come up.
const A_TRIP: &str = "\
sandbox
preset
fight
fight
fight
fight
fight
fight
fight
road
go
answer 0
walk w
walk e
answer 0
walk n
answer 0
walk w
answer 0
walk s
answer 0
walk e
answer 0
walk n
road
town on
road
quit
";

#[test]
fn a_county_trip_replays_identically() {
    let once = play(A_TRIP);
    let twice = play(A_TRIP);
    assert_eq!(once, twice, "the same trip made a different county");
    assert!(once.contains("Down into THE HUNDRED at A6"), "never got down there:\n{once}");
    assert!(once.contains("THE HUNDRED - A6 - 5 MOVES LEFT"), "no banner:\n{once}");
    assert!(once.contains("the edge of the county"), "the edge was not walked into");
    assert!(once.contains("walked over, and already yours"), "nothing was walked over twice");
    assert!(once.contains("no moves left"), "the trip never ran out");
    assert!(once.contains("You are not in THE HUNDRED."), "a spent trip took another move");
}

/// The three county verbs answer from the road rather than panicking.
#[test]
fn the_county_verbs_say_no_from_the_road() {
    let out = play("walk n\nout\ngo\nwalk sideways\nquit\n");
    assert!(out.contains("You are not in THE HUNDRED."), "{out}");
    assert!(out.contains("The way down is in a town, and you are not at one."), "{out}");
    assert!(out.contains("error: walk n|s|e|w"), "{out}");
}

#[test]
fn help_lists_the_county_verbs() {
    let out = play("help\nquit\n");
    for verb in ["go ", "walk n|s|e|w", "out "] {
        assert!(out.contains(verb), "help does not advertise {verb:?}:\n{out}");
    }
}

/// The driver prints the county half of the map, and the same one twice.
///
/// F9's exit criterion is that the CLI and the GUI draw the same county, and
/// what a driver can prove of that is that it draws one at all and that two
/// runs of one script draw it identically. The GUI's half is
/// `the_second_tab_and_the_grid_it_opens_both_fit`, and both read the same
/// drawing rules off the same run.
#[test]
fn the_map_draws_the_county_and_draws_it_the_same_way_twice() {
    let script = "\
sandbox
preset
fight
fight
fight
fight
fight
fight
fight
map
go
walk e
map
out
map
quit
";
    let once = play(script);
    let twice = play(script);
    assert_eq!(once, twice, "the same script drew a different county");
    assert!(once.contains("THE HUNDRED"), "no county on the map:\n{once}");
    // Greyed before the first visit, and a grid after it.
    assert!(
        once.contains("A county, under the road")
            || once.contains("a county, under the road"),
        "an unvisited county did not say what it was"
    );
    assert!(once.contains("gates:"), "the county drew no gates");
    assert!(once.contains("of 49 cleared"), "the county drew no tally");
}

/// The driver draws the question the tile just asked.
///
/// **The other half of the bug playing it found.** `show_county` printed the
/// compass and never the question, so a walk that landed on a scene printed
/// four directions and the scene appeared much later. A screen that can hold
/// a question has to draw it.
#[test]
fn a_county_tile_asks_its_question_where_you_are_standing() {
    let out = play("\
sandbox
preset
fight
fight
fight
fight
fight
fight
fight
go
walk e
answer 0
walk e
quit
");
    // The mouth's own tile asks something, and the driver shows it: a title, a
    // scene, numbered answers, and how to give one.
    assert!(out.contains("`answer <n>`."), "the driver never offered an answer:\n{out}");
    // And nothing walks until it is answered - the second `walk e` is the one
    // that moves, and the first is refused with the question still up.
    // The question comes up before anything moves: the banner that says where
    // you are standing does not appear until the tile has been answered.
    let asked = out.find("`answer <n>`.").expect("a question");
    let banner = out.find("THE HUNDRED - ").expect("a banner, once the tile is done with you");
    assert!(
        asked < banner,
        "the walking screen was drawn before the question on the tile under it"
    );
}

/// The twelve verbs A1 gave this driver, walked in one script.
///
/// Written because a verb nobody scripts is a verb nobody replays, and this
/// milestone's whole claim is that a transcript is something a person can
/// type. Four of these - `clear <slot>`, `grow`, `crush` and `perambulate` -
/// had no spelling in **either** interface before now
/// (`console/tests/parity.rs`), so this is the first time a script has been
/// able to ask for them at all.
const THE_NEW_VERBS: &str = "\
preset
shop
reroll
pin 0
shop
barter 0 Oak Handle
lock Iron Blade
lift Iron Blade
turn Iron Blade
drop Iron Blade weapon 0 0
undo
clear weapon
show weapon
grow weapon
crush Oak Handle
mouths
perambulate 0 5
drink 0
brawl
quit
";

#[test]
fn the_new_verbs_replay_identically_too() {
    let once = play(THE_NEW_VERBS);
    let twice = play(THE_NEW_VERBS);
    assert_eq!(once, twice, "the same script made a different run");
    assert!(!once.contains("unknown command"), "a verb went missing:\n{once}");
}

/// The four that had no interface answer, rather than being unknown words.
///
/// A refusal in the transcript is the point: it says the verb exists and the
/// run is not in a place to use it, which is what a player would be told.
#[test]
fn the_four_that_had_no_interface_answer_from_the_road() {
    let out = play("clear weapon\ngrow weapon\ncrush Oak Handle\nperambulate 0 5\nquit\n");
    assert!(!out.contains("unknown command"), "{out}");
    assert!(out.contains("No row owed."), "{out}");
    assert!(out.contains("does not crush"), "{out}");
    assert!(out.contains("Not granted, or not from a mouth."), "{out}");
}

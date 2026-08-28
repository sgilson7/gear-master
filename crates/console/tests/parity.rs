//! Does this console offer what the game offers, and nothing more?
//!
//! Two directions, because half a lint is not a lint (`CLAUDE.md` §6 trap 19):
//!
//! 1. **Nothing more.** Every `Run` method a `Verb` presses is either one a
//!    shipped interface also calls, or one of the four this milestone found
//!    had no interface at all. A verb that reaches something no interface
//!    reaches is an agent playing a game nobody can play.
//! 2. **Nothing less.** Every state-changing `Run` method a shipped interface
//!    calls is either a `Verb` or a named cheat. A missing verb is content an
//!    agent cannot reach and a person can.
//!
//! Both directions read the two interfaces' source at test time. That is how
//! `assembly_bonuses::which_pools_a_board_can_actually_make` works and it is
//! the right shape here for the same reason: walk what exists, collect what it
//! reaches, and assert the difference is a list somebody wrote down.

const GUI_WHOLE: &str = include_str!("../../gui/src/main.rs");
const CLI: &str = include_str!("../../cli/src/main.rs");
const RUN: &str = include_str!("../../engine/src/run.rs");
const CONSOLE: &str = include_str!("../src/lib.rs");

/// The window's source with its own tests cut off.
///
/// The GUI's `#[cfg(test)]` modules drive the run directly - one of them calls
/// `enter_county` to check what a refused step says - and a test is not a
/// button. Scanning the whole file would let any cheat a GUI test happens to
/// use pass for a player action, which is exactly the loophole trap 29 is
/// about: ask what the cheapest way to satisfy a lint is before shipping it.
fn gui() -> &'static str {
    GUI_WHOLE.split("\n#[cfg(test)]").next().unwrap()
}

/// `Run` methods that change something.
fn mutators() -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = RUN.lines().peekable();
    while let Some(l) = lines.next() {
        let Some(rest) = l.trim_start().strip_prefix("pub fn ") else { continue };
        if !l.starts_with("    pub fn ") {
            continue;
        }
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        // The receiver is on this line or the next one.
        let head = format!("{}{}", l, lines.peek().copied().unwrap_or(""));
        if head.contains("&mut self") {
            out.push(name);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn calls(src: &str) -> Vec<String> {
    calls_on(src, "run.")
}

/// Method names called on a given receiver.
fn calls_on(src: &str, recv: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in src.match_indices(recv) {
        let rest = &src[i + recv.len()..];
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && rest[name.len()..].starts_with('(') {
            out.push(name);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Reached by a `Verb`, in `Console::press`.
fn pressed() -> Vec<String> {
    let body = CONSOLE.split("fn press(").nth(1).expect("press exists");
    let body = body.split("\n    fn answer(").next().unwrap();
    // `press` binds the run as `r`, so that is the receiver to walk.
    let mut out = calls_on(body, "r.");
    // `answer` is `press`'s continuation and reaches three more.
    out.extend(["take_choice".into(), "take_choice_with".into(), "pending_event".into()]);
    out.sort();
    out.dedup();
    out
}

/// Not verbs, on purpose. A pilot must not be able to spell any of these.
///
/// The first five are the evaluation's forbidden list (§10); the rest are
/// bookkeeping an interface does for itself, or a test helper.
const CHEATS: &[&str] = &[
    "force_win",
    "skip_to",
    "skip_fight",
    "with_all_pieces",
    "apply_preset",
    "wipe",
    "give",
    "grant_life",
    "grant_quest",
    "begin_fight",
    "begin_parish",
    "begin_county_fight",
    "set_theme",
    "clear_all_pieces",
    "refresh_class_effects",
    "forget_undo",
    "grow_boards",
    "unlock_insight",
    "reveal_town",
    "arrested_into_the_county",
    "apply_outcome",
    "count",
    "take_passenger",
    "deliver_passenger",
    "melt",
    "fight",
    // Reached only from the window's scene-jump menu, which sets `run.rung`
    // by hand and calls `force_win` two lines later. A developer's ladder into
    // the middle of the game is not a button.
    "enter_dungeon_at",
];

/// Not actions at all: the screen keeping its own house.
///
/// `take_receipt` is the interface reading a line it has already drawn and
/// dismissing it. `Console::apply` does the same thing in the same place, and
/// it is not a verb because pressing it is not a thing a player does.
const BOOKKEEPING: &[&str] = &["take_receipt", "back_to_loadout", "settle"];

/// Player-facing mutators that **no shipped interface reaches**, found by this
/// test when it was first written, at `020bc7c`.
///
/// Each is a thing the engine can do, that a person cannot ask it to do. The
/// console gives all four a verb - which is the fix - and the CLI gives all
/// four a spelling, so the transcript stays something a person can type.
///
/// `walk_the_perambulation` is the one that matters: THE PARISH is a chain
/// whose final journey is a route rather than a destination, it shipped with
/// THE HUNDRED, and until this milestone the only thing that had ever walked
/// it was `tests/hundred.rs`.
const NO_INTERFACE_HAD_THESE: &[&str] =
    &["clear_slot", "crush", "grow_slot", "walk_the_perambulation"];

#[test]
fn every_verb_presses_something_an_interface_also_presses() {
    let gui = calls(gui());
    let cli = calls(CLI);
    for m in pressed() {
        let known = gui.contains(&m)
            || cli.contains(&m)
            || NO_INTERFACE_HAD_THESE.contains(&m.as_str());
        assert!(
            known,
            "a verb presses `{}` and neither interface does. Either it is a \
             player action - in which case say which button - or it is not, \
             and it is not a verb.",
            m
        );
    }
}

#[test]
fn every_button_an_interface_has_is_a_verb_or_a_named_cheat() {
    let muts = mutators();
    let gui = calls(gui());
    let cli = calls(CLI);
    let pressed = pressed();
    let mut missing = Vec::new();
    for m in &muts {
        if !gui.contains(m) && !cli.contains(m) {
            continue;
        }
        if pressed.contains(m)
            || CHEATS.contains(&m.as_str())
            || BOOKKEEPING.contains(&m.as_str())
        {
            continue;
        }
        missing.push(m.clone());
    }
    assert!(
        missing.is_empty(),
        "an interface can do these and a pilot cannot: {:?}",
        missing
    );
}

#[test]
fn the_four_that_had_no_interface_have_one_now() {
    // The fix, held as a ratchet. The CLI reaches all four as of this
    // milestone; if a later commit takes one away, a person loses a button an
    // agent still has, and the whole claim of this mission is that the two
    // sets are the same.
    let cli = calls(CLI);
    let missing: Vec<&&str> =
        NO_INTERFACE_HAD_THESE.iter().filter(|m| !cli.contains(&m.to_string())).collect();
    assert!(missing.is_empty(), "the CLI stopped reaching {:?}", missing);
}

#[test]
fn the_window_still_has_no_button_for_the_four() {
    // Recorded rather than fixed. Giving the GUI four buttons is interface
    // work with a layout question attached to each, and this mission is not
    // the place for it - but a run played in the window still cannot walk a
    // perambulation, and that is worth failing a test about the day somebody
    // thinks it can.
    let gui = calls(gui());
    let reached: Vec<&&str> =
        NO_INTERFACE_HAD_THESE.iter().filter(|m| gui.contains(&m.to_string())).collect();
    assert!(
        reached.is_empty(),
        "the window reaches {:?} now - take them off NO_INTERFACE_HAD_THESE",
        reached
    );
}

#[test]
fn no_verb_can_spell_a_cheat() {
    let pressed = pressed();
    for c in CHEATS {
        assert!(
            !pressed.contains(&c.to_string()),
            "`{}` is reachable from a Verb. The forbidden list is only as good \
             as this assertion (`HANDOFF-solver.md` §7).",
            c
        );
    }
}

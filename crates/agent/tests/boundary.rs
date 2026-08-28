//! The mission's central claim, as a test.
//!
//! An agent that "has no access to the code base" is a promise unless
//! something enforces it. This is what enforces it: the pilot's manifest names
//! one dependency, so Rust's own name resolution refuses every path into the
//! engine's tables. There is no discipline involved and nothing to remember.
//!
//! If this fails, the numbers this mission produces stop meaning what they
//! say, because a pilot that can simulate a fight it is not standing in front
//! of is not playing the game.

const MANIFEST: &str = include_str!("../Cargo.toml");
const LIB: &str = include_str!("../src/lib.rs");

#[test]
fn the_pilot_depends_on_the_console_and_on_nothing_else() {
    let deps = MANIFEST.split("[dependencies]").nth(1).expect("a dependencies section");
    let named: Vec<&str> = deps
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('['))
        .collect();
    assert_eq!(
        named.len(),
        1,
        "the pilot has grown a second dependency: {:?}",
        named
    );
    assert!(named[0].starts_with("gearmaster-console"), "{:?}", named[0]);
}

#[test]
fn the_pilot_cannot_name_the_engine_or_the_oracle() {
    // Comments may name them - this crate's manifest explains at length why
    // they are absent. Code may not.
    let code = |src: &str| -> String {
        src.lines()
            .map(str::trim)
            .filter(|l| !l.starts_with('#') && !l.starts_with("//") && !l.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let manifest = code(MANIFEST);
    let lib = code(LIB);
    for forbidden in ["gearmaster_engine", "gearmaster-engine", "gearmaster_oracle"] {
        assert!(
            !manifest.contains(forbidden) && !lib.contains(forbidden),
            "`{}` appears in the pilot. It must not: an agent that can read the \
             tables is not playing the game, it is looking up the answer.",
            forbidden
        );
    }
}

#[test]
fn the_pilot_cannot_spell_the_forbidden_list() {
    // §10's list, checked against the source rather than against a habit.
    // Every one of these is `pub` on `Run` and none of them is reachable from
    // here, because `Run` is not reachable from here.
    let lib = LIB
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    for cheat in [
        "skip_to",
        "force_win",
        "with_all_pieces",
        "apply_preset",
        "skip_fight",
        "simulate_party",
        "CATALOG",
        "LADDER",
        "MonsterSpec",
    ] {
        assert!(!lib.contains(cheat), "the pilot mentions `{}`", cheat);
    }
}

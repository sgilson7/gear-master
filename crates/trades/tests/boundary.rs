//! Neither agent can name the engine or the oracle.
//!
//! The same claim `crates/agent/tests/boundary.rs` makes, for the same reason,
//! and it has to be made again because it is a property of *this* crate's
//! manifest rather than of the workspace. Training is privileged and lives in
//! `gearmaster-lab`; what is learned comes back here as plain weights.

const MANIFEST: &str = include_str!("../Cargo.toml");

fn code(src: &str) -> String {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_trades_depend_on_the_console_and_nothing_else() {
    let deps = MANIFEST.split("[dependencies]").nth(1).expect("a dependencies section");
    let named: Vec<&str> = deps
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('['))
        .collect();
    assert_eq!(named.len(), 1, "a second dependency appeared: {:?}", named);
    assert!(named[0].starts_with("gearmaster-console"), "{:?}", named[0]);
}

#[test]
fn neither_agent_can_spell_a_privileged_name() {
    let manifest = code(MANIFEST);
    for forbidden in ["gearmaster-engine", "gearmaster_engine", "gearmaster-oracle", "burn"] {
        assert!(
            !manifest.contains(forbidden),
            "`{}` reached the trades' manifest. An agent that can simulate a \
             fight it is not standing in is not playing the game.",
            forbidden
        );
    }
}

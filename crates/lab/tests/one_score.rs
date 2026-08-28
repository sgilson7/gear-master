//! Two readings of one idea, held to the same answer.
//!
//! S0 exists twice on purpose: `gearmaster-oracle` computes it from a
//! `Loadout`, and `gearmaster-agent` computes it from the `View` - because the
//! pilot may not name the oracle, and a board's worth has to be something it
//! can work out from its own screen. Two implementations of one idea is a
//! thing that drifts, and this is the only place in the workspace that can see
//! both.

use gearmaster_agent::sense::Sense;
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::run::Run;
use gearmaster_engine::share;
use gearmaster_oracle::{Board, Surrogate};

fn board_from_code(code: &str) -> (Board, Console) {
    let shared = share::import(code).expect("a code the repo ships");
    let board = Board {
        gear: shared.placed.iter().map(|&(d, s, x, y, r)| (d, s, x, y, r)).collect(),
        chunks: Vec::new(),
        rows: shared.slot_rows.map(|r| r + shared.extra_rows),
    };
    // The same board, standing in a run, so the console can draw it.
    let mut run = Run::new();
    run.clear_all();
    run.owned.clear();
    let (reg, lo) = board.rebuild();
    run.registry = reg;
    run.loadout = lo;
    run.owned = gearmaster_engine::piece::SlotKind::ALL
        .into_iter()
        .flat_map(|k| run.loadout.slot(k).pieces())
        .collect();
    (board, Console::standing_in(run, 0))
}

#[test]
fn the_screen_and_the_loadout_agree_about_what_a_board_does() {
    for (label, code) in [
        ("owner", share::A_WINNING_RUN),
        ("friend", share::A_FRIENDS_RUN),
        ("perfect", share::A_PERFECT_RUN),
    ] {
        let (board, console) = board_from_code(code);
        let privileged = Surrogate::of_board(&board);
        let blind = Sense::of(&console.view());

        assert_eq!(privileged.items, blind.items, "{}: item count", label);
        assert_eq!(privileged.flow, blind.flow, "{}: mana a second", label);
        assert_eq!(privileged.armour_ps, blind.armour_ps, "{}: armour a second", label);
        assert_eq!(
            privileged.physical_dps + privileged.magic_dps,
            blind.damage_ps,
            "{}: damage a second",
            label
        );
        assert_eq!(privileged.fastest_ms, blind.fastest_ms, "{}: the fastest item", label);
        assert_eq!(privileged.curse_resist, blind.curse_resist, "{}: curse resistance", label);
        assert_eq!(privileged.health, blind.health, "{}: health", label);
        assert_eq!(privileged.strength, blind.strength, "{}: strength", label);
    }
}

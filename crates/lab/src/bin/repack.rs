//! Can the pilot recover what a person did with the same pieces?
//!
//! The benchmark `design/the-apprentice.md` §8 gives A3, and it is the one
//! that decides whether anything above it means anything: **a packer that
//! cannot recover what a person did with the same pieces cannot be trusted to
//! do better with different ones.** The human packed the owner's seventy-five
//! pieces to 48/50.
//!
//!     cargo run --release -p gearmaster-oracle --bin repack
//!
//! ## Blind hands, privileged eyes
//!
//! The harness is privileged: it stands a run in front of the pilot with a
//! particular tray in it (`Console::standing_in`, which takes a `Run` and is
//! therefore unreachable from the pilot's own crate), and afterwards it walks
//! the whole ladder with the oracle to say how far the board got.
//!
//! **The pilot builds blind.** It sees a tray, five grids and a character
//! sheet, and it presses keys. It never learns what any of it is worth in a
//! fight, and the number this prints is not available to it.

use gearmaster_agent::hands;
use gearmaster_agent::sense::Sense;
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::combat::{simulate_at, Outcome, LADDER, SUDDEN_DEATH_MS};
use gearmaster_engine::piece::{PieceId, CATALOG};
use gearmaster_engine::run::Run;
use gearmaster_engine::share;
use std::time::Instant;

/// A run holding **exactly** these pieces and no others.
///
/// `Run::new` deals the starter kit - an oak handle and an iron blade
/// (`run.rs:10`) - so a tray built on top of one is two pieces bigger than it
/// says it is. The first run of this benchmark reported the owner's tray as
/// seventy-seven, which is a comparison against a board the owner did not
/// build.
fn tray_of(defs: &[usize]) -> Run {
    let mut run = Run::new();
    run.mode = Mode::Grinder;
    run.clear_all();
    run.owned.clear();
    for &d in defs {
        let id = run.registry.alloc(d);
        run.owned.push(id);
    }
    run
}

/// Every piece a share code holds, stripped of where it sat.
fn pieces_of(code: &str) -> Vec<usize> {
    share::import(code).expect("a code the repo ships").placed.iter().map(|&(d, ..)| d).collect()
}

/// The preset's twenty-two, taken off the board the auto-builder makes.
fn preset_pieces() -> Vec<usize> {
    let mut run = Run::new();
    run.apply_preset();
    run.owned.iter().map(|&id| run.registry.def_index(id)).collect()
}

/// Walk the ladder with the board as it stands, and say how far it got.
fn ladder(run: &Run) -> (usize, usize, u32) {
    let (stats, items) = (run.player_stats(), run.combat_items());
    let mut cleared = 0;
    let mut board_cleared = 0;
    let mut ttk: Vec<u32> = Vec::new();
    for spec in LADDER {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        if log.outcome == Outcome::Victory {
            cleared += 1;
            ttk.push(log.duration_ms);
            if log.duration_ms < SUDDEN_DEATH_MS {
                board_cleared += 1;
            }
        }
    }
    ttk.sort_unstable();
    (cleared, board_cleared, ttk.get(ttk.len() / 2).copied().unwrap_or(0))
}

fn main() {
    let budget: usize = std::env::var("REPACK_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400_000);

    println!(
        "Repack-from-tray. The pilot sees a tray and five grids and presses keys;\n\
         the ladder walk afterwards is the harness's, not its own.\n"
    );
    println!(
        "{:<10} {:>6} {:>7} {:>7} {:>6} {:>7} {:>8} {:>9}   {}",
        "tray", "pieces", "seated", "items", "cells", "presses", "cleared", "median", "wall-clock"
    );
    println!("{}", "-".repeat(96));

    let trays: Vec<(&str, Vec<usize>)> = vec![
        ("starter", vec![
            CATALOG.iter().position(|d| d.name == "Oak Handle").unwrap(),
            CATALOG.iter().position(|d| d.name == "Iron Blade").unwrap(),
        ]),
        ("preset", preset_pieces()),
        ("owner", pieces_of(share::A_WINNING_RUN)),
        ("friend", pieces_of(share::A_FRIENDS_RUN)),
        ("perfect", pieces_of(share::A_PERFECT_RUN)),
    ];

    for (label, defs) in trays {
        let n = defs.len();
        let t = Instant::now();
        // The tray cap is twelve, and these trays are far bigger - so the
        // board is built in loads, which is also how a run acquires them.
        let mut run = tray_of(&[]);
        assert!(run.owned.is_empty(), "the tray starts empty or the count is a lie");
        let mut seated = 0;
        let mut presses = 0;
        for load in defs.chunks(gearmaster_engine::run::INVENTORY_CAP) {
            for &d in load {
                let id = run.registry.alloc(d);
                run.owned.push(id);
            }
            let mut c = Console::standing_in(run, 0);
            let p = hands::pack(&mut c, budget);
            seated += p.seated;
            presses += p.presses;
            run = c.into_run();
            // Anything the hands would not seat is dropped rather than
            // carried: the tray has to be empty before the next load, and a
            // piece nothing wanted is a piece a player would have sold.
            let left: Vec<PieceId> = run.inventory();
            for id in left {
                run.owned.retain(|&o| o != id);
            }
        }
        let elapsed = t.elapsed().as_secs_f64();
        let (cleared, board, median) = ladder(&run);
        let view_cells = Sense::of(&Console::standing_in(run, 0).view());
        println!(
            "{:<10} {:>6} {:>7} {:>7} {:>6} {:>7} {:>4}/{:<3} {:>8.2}s   {:.1}s",
            label,
            n,
            seated,
            view_cells.items,
            view_cells.filled,
            presses,
            cleared,
            board,
            median as f64 / 1000.0,
            elapsed
        );
    }

    println!(
        "\ncleared is victories out of fifty; the second figure is how many were\n\
         board-decided rather than the clock's. The human packed the owner's\n\
         seventy-five to 48/50."
    );
}

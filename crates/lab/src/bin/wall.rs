//! What stops a run, and whether it is the board or the road.
//!
//! Twelve of sixteen seeds stopped at exactly rung 13 in A4's first pass. A
//! wall that lands on the same rung from twelve different economies is a
//! statement about the rung rather than about the seeds.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::sense::Sense;
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::combat::{simulate_at, Outcome, LADDER};
use gearmaster_engine::rng::Rng;

fn seeds(n: usize) -> Vec<u64> {
    let mut out = vec![0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

fn main() {
    let n: usize = std::env::var("WALL_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
    println!("Where a run stops, and what its board could do when it stopped.\n");
    println!(
        "{:<20} {:>5} {:>6} {:>7} {:>6} {:>6} {:>7}  {}",
        "seed", "rung", "items", "cells", "hp", "gold", "beats", "the creature it could not pass"
    );
    println!("{}", "-".repeat(108));
    let mut stopped: std::collections::BTreeMap<usize, usize> = Default::default();

    for seed in seeds(n) {
        // Play, then stand a fresh console in the same run to read the board.
        let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };
        let e = pilot::play(seed, Mode::Grinder, Difficulty::Medium, d);
        let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        for line in &e.transcript {
            if let Some(v) = gearmaster_console::Verb::parse(line) {
                c.apply(v);
            }
        }
        let v = c.view();
        let s = Sense::of(&v);
        // How far that board gets against the ladder, judged by the harness.
        let (stats, items) = c.board_for_scoring();
        let mut beats = 0;
        for spec in LADDER {
            if simulate_at(stats, &items, spec, Difficulty::Medium).outcome == Outcome::Victory {
                beats += 1;
            }
        }
        let wall = LADDER.get(e.best_rung.saturating_sub(1)).map(|s| s.name).unwrap_or("-");
        println!(
            "{:<20} {:>5} {:>6} {:>7} {:>6} {:>6} {:>7}  {}",
            format!("{:#010X}", seed),
            e.best_rung,
            s.items,
            s.filled,
            s.health,
            v.gold,
            beats,
            wall
        );
        *stopped.entry(e.best_rung).or_default() += 1;
    }

    println!("\nrungs runs stopped on:");
    for (rung, count) in stopped {
        println!(
            "  {:<3} {:<24} {}",
            rung,
            LADDER.get(rung.saturating_sub(1)).map(|s| s.name).unwrap_or("-"),
            "#".repeat(count)
        );
    }
}

//! Play the seed set, and say how far the pilot got.
//!
//!     cargo run --release -p gearmaster-lab --bin play
//!     PLAY_SEEDS=16 PLAY_MODE=rogue cargo run --release -p gearmaster-lab --bin play
//!
//! The harness is privileged only in that it *starts* runs and times them.
//! Everything inside a run is the pilot pressing keys, and the pilot has never
//! seen this file.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::prior::Learned;
use gearmaster_agent::seen::Seen;
use gearmaster_console::{Difficulty, Mode};
use gearmaster_engine::rng::Rng;
use std::time::Instant;

/// The seed set, as A0 wrote it down: the four the repo already uses, then
/// draws from `Rng::new(0x501_7E5)` in order. The first sixty-four are the
/// training half.
fn seeds(n: usize) -> Vec<u64> {
    let mut out =
        vec![0x5EED_1234_ABCD_0001u64, 0x0000_0000_0000_6060, 0x0000_0000_0000_1111, 0x0000_0000_0000_1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

fn main() {
    let n: usize =
        std::env::var("PLAY_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
    let mode = match std::env::var("PLAY_MODE").as_deref() {
        Ok("rogue") => Mode::Rogue,
        _ => Mode::Grinder,
    };
    let budget: usize =
        std::env::var("PLAY_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let patience: usize =
        std::env::var("PLAY_PATIENCE").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
    let d = Doctrine { budget, patience, ..Doctrine::default() };

    let keep: usize = std::env::var("PLAY_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(16);
    let learned = std::env::var("PLAY_PRIOR").ok().and_then(|p| Learned::load(&p, keep));
    println!(
        "{:?}, Medium, {} seeds, {} presses each, patience {}. {}\n",
        mode,
        n,
        budget,
        patience,
        match &learned {
            Some(_) => format!("Learned prior, keeping the top {} seats.", keep),
            None => "No prior.".into(),
        }
    );
    println!(
        "{:<20} {:>5} {:>7} {:>7} {:>7} {:>6} {:>6} {:>6} {:>8}  {}",
        "seed", "rung", "board", "game", "losses", "buys", "doors", "towns", "presses", "why"
    );
    println!("{}", "-".repeat(112));

    let mut rungs: Vec<usize> = Vec::new();
    let mut total_wall = 0.0;
    let mut ends = Vec::new();
    for seed in seeds(n) {
        let t = Instant::now();
        let mut seen = Seen::default();
        let e = match &learned {
            Some(l) => pilot::play_guided(seed, mode, Difficulty::Medium, d, &mut seen, l),
            None => pilot::play(seed, mode, Difficulty::Medium, d),
        };
        total_wall += t.elapsed().as_secs_f64();
        println!(
            "{:<20} {:>5} {:>7} {:>7} {:>7} {:>6} {:>6} {:>6} {:>8}  {}",
            format!("{:#018X}", seed),
            e.best_rung,
            e.board_clears,
            e.game_clears,
            e.losses,
            e.bought,
            e.doors,
            e.towns,
            e.presses,
            e.why
        );
        rungs.push(e.best_rung);
        ends.push(e);
    }

    rungs.sort_unstable();
    let scr = |at: usize| ends.iter().filter(|e| pilot::reached(e, at)).count();
    println!("\n{}", "-".repeat(112));
    println!(
        "rungs reached: min {}, median {}, max {}   |   {:.1}s a seed",
        rungs[0],
        rungs[rungs.len() / 2],
        rungs[rungs.len() - 1],
        total_wall / n as f64
    );
    for target in [5usize, 10, 15, 25, 50] {
        println!(
            "  SCR(R{:<2})  {:>3}/{:<3}  {:>5.1}%",
            target,
            scr(target),
            n,
            100.0 * scr(target) as f64 / n as f64
        );
    }
    let why = |w: &str| ends.iter().filter(|e| e.why == w).count();
    println!("\nwhere they stopped:");
    for w in [
        "stuck below its ceiling",
        "out of presses",
        "the run ended",
        "a door with no open choice",
        "nothing left to press",
        "nothing worth pressing",
    ] {
        if why(w) > 0 {
            println!("  {:<28} {}", w, why(w));
        }
    }
}

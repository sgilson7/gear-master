//! Does more of the same buy anything, and if not, why not?
//!
//! The spec's A6 opens only if the failure is **exploration** - the tray never
//! holds the family the fight wants - rather than **evaluation** - the board
//! is nearly right and scored wrongly. A net is the answer to the first and a
//! better objective is the answer to the second, and building the wrong one is
//! the most expensive mistake available here.
//!
//!     cargo run --release -p gearmaster-lab --bin plateau

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Difficulty, Mode};
use gearmaster_engine::rng::Rng;
use std::time::Instant;

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
    let n: usize = std::env::var("PLATEAU_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(64);
    println!(
        "Grinder, Medium, {} seeds. Patience is how many fights a run spends on\n\
         a rung it is not passing; the press budget was never the binding one.\n",
        n
    );
    println!(
        "{:<10} {:>9} {:>8} {:>8} {:>8} {:>8} {:>9} {:>9}",
        "patience", "budget", "R10", "R15", "R25", "R50", "median", "wall/seed"
    );
    println!("{}", "-".repeat(80));

    let mut runs_at: Vec<(usize, Vec<pilot::Ended>)> = Vec::new();
    for (patience, budget) in [(8usize, 200_000usize), (24, 600_000), (80, 2_000_000)] {
        let d = Doctrine { patience, budget, coverage: 0.0 };
        let t = Instant::now();
        let ends: Vec<pilot::Ended> = seeds(n)
            .into_iter()
            .map(|s| pilot::play(s, Mode::Grinder, Difficulty::Medium, d))
            .collect();
        let el = t.elapsed().as_secs_f64();
        let scr = |at: usize| ends.iter().filter(|e| pilot::reached(e, at)).count();
        let mut rungs: Vec<usize> = ends.iter().map(|e| e.best_rung).collect();
        rungs.sort_unstable();
        println!(
            "{:<10} {:>9} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>9} {:>8.2}s",
            patience,
            budget,
            100.0 * scr(10) as f64 / n as f64,
            100.0 * scr(15) as f64 / n as f64,
            100.0 * scr(25) as f64 / n as f64,
            100.0 * scr(50) as f64 / n as f64,
            rungs[rungs.len() / 2],
            el / n as f64
        );
        runs_at.push((patience, ends));
    }

    // ---- why the failures failed ---------------------------------------
    let (_, ref deepest) = runs_at[runs_at.len() - 1];
    let failed: Vec<&pilot::Ended> = deepest.iter().filter(|e| !pilot::reached(e, 15)).collect();
    println!(
        "\nOf {} runs that never passed rung 15, how close was the closest loss?\n",
        failed.len()
    );
    let mut narrow = 0;
    let mut wide = 0;
    let mut buckets = [0usize; 5];
    for e in &failed {
        let Some(l) = e.narrowest_loss else { continue };
        let b = ((l * 5.0) as usize).min(4);
        buckets[b] += 1;
        if l <= 0.10 {
            narrow += 1;
        } else {
            wide += 1;
        }
    }
    for (i, count) in buckets.iter().enumerate() {
        println!(
            "  {:>3}-{:>3}% of the creature left standing  {:<3} {}",
            i * 20,
            (i + 1) * 20,
            count,
            "#".repeat(*count)
        );
    }
    println!(
        "\n  within 10% - the board was nearly right:  {}\n  \
         wider than that - the tray never held it:   {}",
        narrow, wide
    );
    println!(
        "\n{}",
        if narrow > wide {
            "EVALUATION: the boards are close and something is scoring them wrongly.\n\
             The answer is a better objective, not a bigger search."
        } else {
            "EXPLORATION: the boards are not close. The tray never holds the family\n\
             the fight wants, which is what a learned prior is for."
        }
    );
}

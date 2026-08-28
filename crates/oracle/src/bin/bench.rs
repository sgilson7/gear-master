//! What the three tiers cost, through this crate's own path.
//!
//! A0 timed the engine's primitives. This times what a search actually pays,
//! which is not the same thing: `Surrogate::of_board` rebuilds the board
//! before it reads it, and a rebuild is not free.

use gearmaster_engine::combat::{Difficulty, LADDER};
use gearmaster_oracle::gate::{Gate, References};
use gearmaster_oracle::{Oracle, Surrogate};
use std::time::Instant;

fn med(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn timed(n: u32, mut f: impl FnMut()) -> f64 {
    let mut v = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let t = Instant::now();
        f();
        v.push(t.elapsed().as_nanos() as f64);
    }
    med(v)
}

fn main() {
    let refs = References::standard();
    let (_, stats, items, board) = &refs.boards[2];
    let spec = &LADDER[29];

    println!("One performance core, release, medians.\n");

    let ns = timed(2_000, || {
        let _ = Surrogate::of(*stats, items);
    });
    println!("  S0, board already built      {:>9.1} ns   ({:>10} /s)", ns, (1e9 / ns) as u64);

    let ns = timed(300, || {
        let _ = Surrogate::of_board(board);
    });
    println!("  S0, rebuilding first         {:>9.1} ns   ({:>10} /s)", ns, (1e9 / ns) as u64);

    let ns = timed(500, || {
        let _ = Oracle::fight_uncached(*stats, items, spec, Difficulty::Medium);
    });
    println!("  S1, one fight                {:>9.1} ns   ({:>10} /s)", ns, (1e9 / ns) as u64);

    let oracle = Oracle::new();
    oracle.fight(board, *stats, items, spec, Difficulty::Medium);
    let ns = timed(20_000, || {
        let _ = oracle.fight(board, *stats, items, spec, Difficulty::Medium);
    });
    println!("  S1, cached                   {:>9.1} ns   ({:>10} /s)", ns, (1e9 / ns) as u64);

    let g = Gate { refs: &refs, rung: 29, rank: spec.rank };
    let cold = Oracle::new();
    let ns = timed(60, || {
        cold.clear();
        let _ = g.rows(&cold, spec);
    });
    println!("  S2, the sixteen-fight gate   {:>9.1} ns   ({:>10} /s)", ns, (1e9 / ns) as u64);

    // Many threads. Each gets its own oracle: a shared cache would need a lock
    // and the fights are the cost, not the lookups.
    for threads in [8usize, 12] {
        let t = Instant::now();
        let per = 40usize;
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    let refs = References::standard();
                    let g = Gate { refs: &refs, rung: 29, rank: spec.rank };
                    let o = Oracle::new();
                    for _ in 0..per {
                        o.clear();
                        let _ = g.rows(&o, spec);
                    }
                });
            }
        });
        let el = t.elapsed().as_secs_f64();
        println!(
            "  S2 on {:>2} threads             {:>9.1} gates/s  ({} fights/s)",
            threads,
            (threads * per) as f64 / el,
            ((threads * per * 16) as f64 / el) as u64
        );
    }
}

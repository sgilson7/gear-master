//! How many decisions each trade actually makes.
//!
//! `design/the-two-trades.md` §3 estimates 30-60 for the quartermaster and
//! 200-500 for the pathfinder. Those are estimates until this is run, and the
//! whole architecture rests on them: if a pathfinder episode is five thousand
//! decisions, the macro-action did not buy what it was supposed to.
//!
//!     cargo run --release -p gearmaster-lab --bin horizons

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Difficulty, Mode, Verb};
use gearmaster_engine::rng::Rng;
use gearmaster_trades::partition::{name_of, owner, Trade};
use std::collections::BTreeMap;

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
    let n: usize = std::env::var("HORIZON_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };

    let mut per_trade: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_verb: BTreeMap<&str, usize> = BTreeMap::new();
    // A quartermaster episode is a run of its verbs between two of the
    // pathfinder's. That is exactly what `pack` will replace.
    let mut episodes: Vec<usize> = Vec::new();
    // The same episodes counted in **decisions that changed something** - a
    // placement that stuck, a purchase, a sale. A learned packer emits those
    // and makes no trials at all, so this is its horizon and the one above is
    // the control's search cost.
    let mut kept_episodes: Vec<usize> = Vec::new();
    let mut pathfinder_decisions: Vec<usize> = Vec::new();
    let mut trials = 0usize;

    for seed in seeds(n) {
        let e = pilot::play(seed, Mode::Grinder, Difficulty::Medium, d);
        let mut here = 0usize;
        let mut kept = 0usize;
        let mut pf = 0usize;
        let mut prev: Option<Verb> = None;
        for line in &e.transcript {
            let Some(v) = Verb::parse(line) else { continue };
            *by_verb.entry(name_of(v)).or_default() += 1;
            // A trial seat is a place followed by an undo. Counted apart,
            // because a learned packer does not make them at all.
            if v == Verb::Undo && matches!(prev, Some(Verb::Place { .. })) {
                trials += 1;
                // The placement before it did not stick, so it was never a
                // decision - and neither was the undo.
                kept = kept.saturating_sub(1);
            } else if !matches!(v, Verb::Undo | Verb::Rotate { .. }) {
                kept += 1;
            }
            prev = Some(v);
            match owner(v) {
                Trade::Quartermaster => {
                    *per_trade.entry("quartermaster").or_default() += 1;
                    here += 1;
                }
                Trade::Pathfinder => {
                    *per_trade.entry("pathfinder").or_default() += 1;
                    pf += 1;
                    if here > 0 {
                        episodes.push(here);
                        kept_episodes.push(kept);
                        here = 0;
                        kept = 0;
                    }
                }
            }
        }
        if here > 0 {
            episodes.push(here);
            kept_episodes.push(kept);
        }
        // Plus one `pack` per episode: the macro-action is a decision too.
        pathfinder_decisions.push(pf + episodes.len().min(pf));
    }

    let total: usize = per_trade.values().sum();
    println!("{} runs of the hand-written control, Grinder, Medium.\n", n);
    println!("presses, by trade:");
    for (who, count) in &per_trade {
        println!("  {:<16} {:>9}  {:>5.1}%", who, count, 100.0 * *count as f64 / total as f64);
    }
    println!(
        "  {:<16} {:>9}  {:>5.1}%   <- place-then-undo pairs, which a learned packer does not make",
        "of which trials", trials * 2, 200.0 * trials as f64 / total as f64
    );

    episodes.sort_unstable();
    kept_episodes.sort_unstable();
    pathfinder_decisions.sort_unstable();
    let pct = |v: &Vec<usize>, p: f64| v.get(((v.len() as f64 - 1.0) * p) as usize).copied().unwrap_or(0);
    println!(
        "\nquartermaster episodes: {}\n  \
         the control's presses:  min {}, median {}, 90th {}, max {}\n  \
         decisions that stuck:   min {}, median {}, 90th {}, max {}",
        episodes.len(),
        episodes.first().copied().unwrap_or(0),
        pct(&episodes, 0.5),
        pct(&episodes, 0.9),
        episodes.last().copied().unwrap_or(0),
        kept_episodes.first().copied().unwrap_or(0),
        pct(&kept_episodes, 0.5),
        pct(&kept_episodes, 0.9),
        kept_episodes.last().copied().unwrap_or(0)
    );
    println!(
        "\npathfinder decisions a run (its verbs, plus one `pack` an episode):\n  \
         min {}, median {}, max {}",
        pathfinder_decisions.first().copied().unwrap_or(0),
        pct(&pathfinder_decisions, 0.5),
        pathfinder_decisions.last().copied().unwrap_or(0)
    );

    println!("\nthe ten commonest verbs:");
    let mut v: Vec<(&&str, &usize)> = by_verb.iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (name, count) in v.into_iter().take(10) {
        println!("  {:<16} {:>9}  {:?}", name, count, owner_of(name));
    }

    println!(
        "\nWhat this says about the spec's estimates:\n  \
         quartermaster 30-60 a episode: {}\n  pathfinder 200-500 a run:      {}",
        verdict(pct(&kept_episodes, 0.5), 30, 60),
        verdict(pct(&pathfinder_decisions, 0.5), 200, 500)
    );
}

fn owner_of(name: &str) -> Trade {
    if gearmaster_trades::QUARTERMASTER.contains(&name) {
        Trade::Quartermaster
    } else {
        Trade::Pathfinder
    }
}

fn verdict(got: usize, lo: usize, hi: usize) -> String {
    if got >= lo && got <= hi {
        format!("median {} - inside", got)
    } else {
        format!("median {} - OUTSIDE, and the estimate moves here", got)
    }
}

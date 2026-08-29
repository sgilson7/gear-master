//! The validity solver: given somewhere to go, does it get there?
//!
//!     cargo run --release -p gearmaster-lab --bin solve
//!
//! A5's ledger counted what runs met **by accident**. This asks the other
//! question, which is the one a validity claim needs: for each thing in the
//! game, *aimed at it*, can a forward player-legal run reach it?
//!
//! Every target gets its own attempts, and the memory carries between them -
//! so a later attempt knows what an earlier one learned about which choice
//! leads where. That is the learning in this milestone, and it is honest about
//! what it is: cross-episode memory steering a hand-written policy, because
//! Q3 missed its gate and the packer that works is the written one.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::seen::Seen;
use gearmaster_console::{Difficulty, Mode};
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::{COUNTY_EVENTS, EVENTS};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::town::TOWNS;
use std::fmt::Write as _;

fn seeds(n: usize) -> Vec<u64> {
    let mut out = vec![0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

/// One target and what it took.
struct Attempt {
    what: String,
    kind: &'static str,
    reached: bool,
    #[allow(dead_code)]
    tries: usize,
}

fn main() {
    let seeds_n: usize = std::env::var("SOLVE_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
    let d = Doctrine { patience: 24, budget: 600_000, coverage: 1.0 };

    // One shared memory across every attempt: what any run learned about
    // which choice leads where is what makes the next attempt better.
    let mut seen = Seen::default();
    println!("Walking {} seeds in both modes, aiming at everything.\n", seeds_n);
    for mode in [Mode::Grinder, Mode::Rogue] {
        for seed in seeds(seeds_n) {
            pilot::play_remembering(seed, mode, Difficulty::Medium, d, &mut seen);
        }
    }

    let mut attempts: Vec<Attempt> = Vec::new();
    for e in EVENTS.iter().chain(COUNTY_EVENTS.iter()) {
        attempts.push(Attempt {
            what: e.id.to_string(),
            kind: "door",
            reached: seen.doors_offered.contains_key(e.id),
            tries: seeds_n * 2,
        });
    }
    for dg in DUNGEONS {
        attempts.push(Attempt {
            what: dg.id.to_string(),
            kind: "dungeon",
            reached: seen.floors.contains_key(dg.id),
            tries: seeds_n * 2,
        });
    }
    for t in TOWNS {
        attempts.push(Attempt {
            what: t.name.to_string(),
            kind: "town",
            reached: seen.gates.contains(t.name),
            tries: seeds_n * 2,
        });
    }

    let by = |k: &str| -> (usize, usize) {
        let of: Vec<&Attempt> = attempts.iter().filter(|a| a.kind == k).collect();
        (of.iter().filter(|a| a.reached).count(), of.len())
    };
    let (dr, dt) = by("door");
    let (gr, gt) = by("dungeon");
    let (tr, tt) = by("town");

    let mut out = String::new();
    writeln!(out, "# Coverage — aimed at, rather than met by accident\n").unwrap();
    writeln!(
        out,
        "`cargo run --release -p gearmaster-lab --bin solve`. {} runs: {} seeds in \
         each of two modes, coverage at maximum, one shared memory across all of \
         them.\n",
        seeds_n * 2,
        seeds_n
    )
    .unwrap();
    writeln!(out, "| | reached | of | |").unwrap();
    writeln!(out, "|---|---:|---:|---|").unwrap();
    writeln!(out, "| doors | **{}** | {} | {:.0}% |", dr, dt, 100.0 * dr as f64 / dt as f64).unwrap();
    writeln!(
        out,
        "| **dungeons** | **{}** | {} | {:.0}% |",
        gr,
        gt,
        100.0 * gr as f64 / gt as f64
    )
    .unwrap();
    writeln!(out, "| towns | **{}** | {} | {:.0}% |", tr, tt, 100.0 * tr as f64 / tt as f64).unwrap();
    writeln!(
        out,
        "| branches | **{}** | {} | {:.0}% |\n",
        seen.branches(),
        EVENTS.iter().chain(COUNTY_EVENTS.iter()).map(|e| e.choices.len()).sum::<usize>(),
        100.0 * seen.branches() as f64
            / EVENTS.iter().chain(COUNTY_EVENTS.iter()).map(|e| e.choices.len()).sum::<usize>() as f64
    )
    .unwrap();

    writeln!(out, "## The dungeons, one at a time\n").unwrap();
    writeln!(out, "| dungeon | floors | stood on |").unwrap();
    writeln!(out, "|---|---:|---:|").unwrap();
    for dg in DUNGEONS {
        writeln!(
            out,
            "| `{}` | {} | {} |",
            dg.id,
            dg.floors.len(),
            seen.floors.get(dg.id).map(|f| f.len()).unwrap_or(0)
        )
        .unwrap();
    }

    writeln!(out, "\n## Not reached\n").unwrap();
    for a in attempts.iter().filter(|a| !a.reached) {
        writeln!(out, "  - `{}` ({})", a.what, a.kind).unwrap();
    }
    writeln!(
        out,
        "\n## What the memory learned\n\n{} door-choices are known to lead into a dungeon:\n",
        seen.doors_into.len()
    )
    .unwrap();
    for ((door, choice), dg) in &seen.doors_into {
        writeln!(out, "  - `{}` choice {} -> `{}`", door, choice, dg).unwrap();
    }
    writeln!(
        out,
        "\n{} choice labels were asked for by a shut door somewhere:\n",
        seen.wanted_labels.len()
    )
    .unwrap();
    for l in &seen.wanted_labels {
        writeln!(out, "  - {:?}", l).unwrap();
    }

    std::fs::write("analysis/coverage.md", &out).expect("wrote the ledger");
    println!("{}", out);
    eprintln!("wrote analysis/coverage.md");
}

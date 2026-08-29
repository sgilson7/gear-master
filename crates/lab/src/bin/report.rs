//! How the pilot is actually doing, in the places the ledger only counts.
//!
//!     cargo run --release -p gearmaster-lab --bin report

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::seen::Seen;
use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::rng::Rng;
use gearmaster_engine::rumour::RUMOURS;

fn seeds(n: usize) -> Vec<u64> {
    let mut out = vec![0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

/// How a run gets into each dungeon, and the door it hangs off.
///
/// Read off `event.rs` and `run.rs` by hand, because the pilot cannot: it sees
/// a choice's words and not its outcome. A report may look.
fn ways_in() -> std::collections::BTreeMap<&'static str, (&'static str, &'static str)> {
    [
        ("the-crevice", ("a door: the-shrine-fork", "the-shrine-fork")),
        ("the-under-mine", ("a door: the-fork", "the-fork")),
        ("the-switchyard", ("a door: the-turntable", "the-turntable")),
        ("the-threshold", ("a town cellar (THE MANSE)", "")),
        ("the-undertow", ("an orb, at a pedestal", "")),
        ("den-rivals", ("an orb, at a pedestal", "")),
        ("wumpus-world", ("an orb, at a pedestal", "")),
    ]
    .into_iter()
    .collect()
}

fn main() {
    let n: usize = std::env::var("REPORT_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    let cover: f32 =
        std::env::var("REPORT_COVERAGE").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let d = Doctrine { patience: 24, budget: 600_000, coverage: cover };

    let mut seen = Seen::default();
    // What each run was actually holding at the end - rumours are components,
    // so they are in the tray or on a board like anything else.
    let mut held: std::collections::BTreeMap<String, usize> = Default::default();
    let mut bartered = 0usize;
    let mut ends = Vec::new();

    for seed in seeds(n) {
        let e = pilot::play_remembering(seed, Mode::Grinder, Difficulty::Medium, d, &mut seen);
        bartered += e.bartered;
        // Replay to read the final state.
        let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        for line in &e.transcript {
            if let Some(v) = Verb::parse(line) {
                c.apply(v);
            }
        }
        let v = c.view();
        for p in &v.tray {
            if RUMOURS.iter().any(|r| r.name == p.name) {
                *held.entry(p.name.clone()).or_default() += 1;
            }
        }
        ends.push(e);
    }

    println!("=== RUMOURS ===\n");
    println!("{} rumours exist. A rumour is bartered for at a pub - never bought.\n", RUMOURS.len());
    println!("  pub doors gone through   {}", seen.town_doors.values().filter_map(|m| m.get("pub")).sum::<usize>());
    println!("  barters made             {}", bartered);
    println!("  rumours still held at the end of a run:");
    if held.is_empty() {
        println!("    none, in any of {} runs", n);
    }
    for (name, count) in &held {
        println!("    {:<34} {} runs", name, count);
    }
    println!("\n  every rumour, and what it is for:");
    for r in RUMOURS {
        println!(
            "    {:<34} {:<5} {}",
            r.name,
            if r.on_the_bar { "bar" } else { "told" },
            if seen.doors_offered.contains_key(r.opens) {
                format!("opens `{}` - which HAS been offered", r.opens)
            } else {
                format!("opens `{}` - never offered", r.opens)
            }
        );
    }

    println!("\n=== MINI DUNGEONS ===\n");
    // Every way into every dungeon, read off the tables - which the *report*
    // may do and the pilot may not. Three roads in, one town cellar, and three
    // that only an orb reaches.
    let ways = ways_in();
    println!(
        "{:<18} {:>6} {:>9}  {:<22} {}",
        "dungeon", "floors", "stood on", "the way in", "was that offered?"
    );
    println!("{}", "-".repeat(84));
    for dg in DUNGEONS {
        let floors = seen.floors.get(dg.id);
        let (how, gate) = ways.get(dg.id).copied().unwrap_or(("unknown", ""));
        let reached = if gate.is_empty() {
            "-".to_string()
        } else if seen.doors_offered.contains_key(gate) {
            "yes".to_string()
        } else {
            "**no**".to_string()
        };
        println!(
            "{:<18} {:>6} {:>9}  {:<22} {}",
            dg.id,
            dg.floors.len(),
            floors.map(|f| f.len()).unwrap_or(0),
            how,
            reached
        );
    }
    println!(
        "\n  {} of {} dungeons entered across {} runs.",
        DUNGEONS.iter().filter(|d| seen.floors.contains_key(d.id)).count(),
        DUNGEONS.len(),
        n
    );

    println!("\n=== WHAT SHUT DOORS ASKED FOR ===\n");
    if seen.wanted_labels.is_empty() {
        println!("  nothing - no shut choice named an earlier one");
    }
    for l in &seen.wanted_labels {
        println!("  a shut choice wanted: {:?}", l);
    }
    println!("\n  doors into dungeons this agent has learned:");
    if seen.doors_into.is_empty() {
        println!("    none");
    }
    for ((door, choice), dg) in &seen.doors_into {
        println!("    {} choice {} -> {}", door, choice, dg);
    }
    println!("\n  the-toads-offer answered: {}", seen.choices_taken.contains_key("the-toads-offer"));
    println!("  the-shrine-fork answered: {:?}", seen.choices_taken.get("the-shrine-fork"));

    println!("\n=== THE RUN ITSELF ===\n");
    let mut rungs: Vec<usize> = ends.iter().map(|e| e.best_rung).collect();
    rungs.sort_unstable();
    println!(
        "  rungs: min {}, median {}, max {}",
        rungs[0],
        rungs[rungs.len() / 2],
        rungs[rungs.len() - 1]
    );
    println!(
        "  doors answered per run: {:.1}   towns: {:.1}   buys: {:.0}   sells: {:.0}   rerolls: {:.0}",
        ends.iter().map(|e| e.doors).sum::<usize>() as f64 / n as f64,
        ends.iter().map(|e| e.towns).sum::<usize>() as f64 / n as f64,
        ends.iter().map(|e| e.bought).sum::<usize>() as f64 / n as f64,
        ends.iter().map(|e| e.sold).sum::<usize>() as f64 / n as f64,
        ends.iter().map(|e| e.rerolled).sum::<usize>() as f64 / n as f64,
    );
}

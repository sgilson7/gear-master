//! The validity ledger: what a forward, player-legal play has actually reached.
//!
//!     cargo run --release -p gearmaster-lab --bin cover
//!
//! Three columns, because the cheapest way to satisfy a coverage metric is to
//! stand in front of things (`CLAUDE.md` §6 trap 29):
//!
//! * **offered** - the door reached the road stack while a run was there;
//! * **answered** - one of its choices was taken;
//! * **branched** - *every* choice has been taken, by some run somewhere.
//!
//! And a **fourth class** that THE ATLAS forced: content reachable only
//! through a specific acquisition. Four of the switchyard's nine floors cannot
//! be walked to from its mouth at all - an orb lands you on a siding inside
//! them - so counting them as "never offered" would report a design decision
//! as a bug.
//!
//! Every gap is classified rather than counted. A door nothing reached because
//! no run got that deep is the wall at rung 13, and it says so; a door runs
//! walked past on a rung they *did* reach is a finding.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_agent::seen::Seen;
use gearmaster_console::{Difficulty, Mode};
use gearmaster_engine::dungeon::DUNGEONS;
use gearmaster_engine::event::{Trigger, COUNTY_EVENTS, EVENTS};
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

/// Floors no walk from the mouth can reach.
fn islands(d: &gearmaster_engine::dungeon::Dungeon) -> Vec<usize> {
    let n = d.floors.len();
    let mut seen = vec![false; n];
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(at) = stack.pop() {
        for e in d.floors[at].exits {
            if e.to < n && !seen[e.to] {
                seen[e.to] = true;
                stack.push(e.to);
            }
        }
    }
    (0..n).filter(|&i| !seen[i]).collect()
}

fn main() {
    let n: usize = std::env::var("COVER_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(64);
    let budget: usize =
        std::env::var("COVER_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let d = Doctrine { budget, coverage: 1.0, ..Doctrine::default() };

    let mut seen = Seen::default();
    let mut stops: std::collections::BTreeMap<String, usize> = Default::default();
    // Both modes: Rogue meets doors a Grinder walks past, because a Grinder
    // farms and a Rogue has to move.
    for mode in [Mode::Grinder, Mode::Rogue] {
        for seed in seeds(n) {
            let e = pilot::play_remembering(seed, mode, Difficulty::Medium, d, &mut seen);
            if let Some(door) = e.stuck_at {
                *stops.entry(door).or_default() += 1;
            }
        }
    }

    // ---- the denominators, off the tables ------------------------------
    // A county door has no rung: its `at` is dead, because it stands on a
    // tile rather than on the road. `None` says so rather than printing 0.
    let doors: Vec<(&str, Option<usize>, usize, &Trigger)> = EVENTS
        .iter()
        .map(|e| (e.id, Some(e.at + 1), e.choices.len(), &e.trigger))
        .chain(COUNTY_EVENTS.iter().map(|e| (e.id, None, e.choices.len(), &e.trigger)))
        .collect();
    let total_choices: usize = doors.iter().map(|(_, _, c, _)| c).sum();

    /// How a door is reached, said in one word.
    ///
    /// `flag: "never"` is `event.rs`'s own **sentinel** for a door nothing on
    /// a rung can reach - it is pushed through `forced_event` by something
    /// else entirely. A ledger that reported those as "runs stood there and it
    /// did not appear" would be describing the design and calling it a gap,
    /// which is the mirror of the mistake the fourth class exists to avoid.
    fn how(t: &Trigger) -> &'static str {
        match t {
            Trigger::Rung => "stands on its rung",
            Trigger::QuickKill { .. } => "wants a fast kill",
            Trigger::SlowKill { .. } => "wants a slow kill",
            Trigger::Whispered { .. } => "wants a rumour",
            Trigger::WhenFlagged { flag: "never", .. } => "delivered, not walked to",
            Trigger::WhenFlagged { .. } => "wants a flag",
        }
    }

    let offered = doors.iter().filter(|(id, ..)| seen.doors_offered.contains_key(*id)).count();
    let answered = doors.iter().filter(|(id, ..)| seen.choices_taken.contains_key(*id)).count();
    let branched = doors
        .iter()
        .filter(|(id, _, c, _)| seen.choices_taken.get(*id).is_some_and(|t| t.len() == *c))
        .count();
    let taken_choices = seen.branches();

    let mut out = String::new();
    let pct = |a: usize, b: usize| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };

    writeln!(out, "# Coverage — what a forward, player-legal play has reached\n").unwrap();
    writeln!(
        out,
        "Written by `cargo run --release -p gearmaster-lab --bin cover`. \
         {} runs: {} seeds in each of two modes, at Medium, {} presses each, \
         with the coverage dial at maximum.\n",
        seen.runs, n, budget
    )
    .unwrap();
    writeln!(
        out,
        "Nothing below is read out of a table. Every count is a place a run \
         stood.\n\nThe deepest rung any run reached is **{}**.\n",
        seen.deepest_rung
    )
    .unwrap();

    writeln!(out, "## The three columns\n").unwrap();
    writeln!(out, "| | offered | answered | branched |").unwrap();
    writeln!(out, "|---|---:|---:|---:|").unwrap();
    writeln!(
        out,
        "| doors ({}) | **{}** ({:.0}%) | **{}** ({:.0}%) | **{}** ({:.0}%) |",
        doors.len(),
        offered,
        pct(offered, doors.len()),
        answered,
        pct(answered, doors.len()),
        branched,
        pct(branched, doors.len())
    )
    .unwrap();
    writeln!(
        out,
        "| choices ({}) | - | **{}** ({:.0}%) | - |\n",
        total_choices,
        taken_choices,
        pct(taken_choices, total_choices)
    )
    .unwrap();

    // ---- classify every gap ---------------------------------------------
    // How many runs stood on a door's rung. A county door is asked of the
    // county instead: how many runs went down there at all.
    let witnesses = |rung: Option<usize>| -> usize {
        match rung {
            Some(r) => seen.rungs_stood.get(&r).copied().unwrap_or(0),
            None => {
                if seen.county_tiles.is_empty() {
                    0
                } else {
                    seen.runs
                }
            }
        }
    };
    /// How many runs have to have stood on a rung before "it never appeared"
    /// is evidence about the door rather than about the ceiling.
    const ENOUGH: usize = 8;

    let mut thin = Vec::new();
    let mut walked_past = Vec::new();
    let mut refused = Vec::new();
    let mut partial = Vec::new();
    let mut forced = Vec::new();
    for &(id, rung, choices, trigger) in &doors {
        let was_offered = seen.doors_offered.contains_key(id);
        let taken = seen.choices_taken.get(id).map(|t| t.len()).unwrap_or(0);
        let w = witnesses(rung);
        if !was_offered {
            if matches!(trigger, Trigger::WhenFlagged { flag: "never", .. }) {
                forced.push((id, rung));
            } else if w < ENOUGH {
                thin.push((id, rung, w));
            } else {
                walked_past.push((id, rung, w, how(trigger)));
            }
        } else if taken == 0 {
            refused.push((id, rung, w));
        } else if taken < choices {
            partial.push((id, rung, taken, choices));
        }
    }

    writeln!(out, "## Every gap, classified\n").unwrap();
    writeln!(
        out,
        "### Too few runs got there to say anything — **{}** doors\n",
        thin.len()
    )
    .unwrap();
    writeln!(
        out,
        "Fewer than {} of {} runs ever stood on the rung these are pinned to, \
         so their absence is a fact about the pilot's ceiling and not about \
         them. Closing this class is a *player* problem and it belongs to A6.\n",
        ENOUGH, seen.runs
    )
    .unwrap();
    for (id, rung, w) in thin.iter().take(60) {
        writeln!(
            out,
            "  - `{}` — {}, {} runs stood there",
            id,
            rung.map(|r| format!("rung {}", r)).unwrap_or_else(|| "in the county".into()),
            w
        )
        .unwrap();
    }

    writeln!(
        out,
        "\n### Offered and never answered — **{}** doors\n",
        refused.len()
    )
    .unwrap();
    writeln!(
        out,
        "A run stood in front of these and took nothing. Either no choice was \
         open, or the run ended there.\n"
    )
    .unwrap();
    for (id, rung, w) in &refused {
        writeln!(
            out,
            "  - `{}` — {}, {} runs stood there",
            id,
            rung.map(|r| format!("rung {}", r)).unwrap_or_else(|| "in the county".into()),
            w
        )
        .unwrap();
    }

    writeln!(
        out,
        "\n### Reached at a rung runs *did* get to, and still never offered — **{}** doors\n",
        walked_past.len()
    )
    .unwrap();
    writeln!(
        out,
        "**This is the class worth reading.** Runs were on the rung and the \
         door did not appear, which means its trigger asks for something no \
         run had — a flag, a word, a fight fast enough. Each one is either \
         content behind a condition nothing meets, or a condition that is \
         harder than it reads.\n"
    )
    .unwrap();
    for (id, rung, w, how) in &walked_past {
        writeln!(
            out,
            "  - `{}` — {}, {} runs stood there · **{}**",
            id,
            rung.map(|r| format!("rung {}", r)).unwrap_or_else(|| "in the county".into()),
            w,
            how
        )
        .unwrap();
    }

    writeln!(
        out,
        "\n### Delivered rather than walked to — **{}** doors\n",
        forced.len()
    )
    .unwrap();
    writeln!(
        out,
        "`flag: \"never\"` is `event.rs`'s own sentinel for a door nothing on a \
         rung can reach: something else pushes it through `forced_event`. \
         Reporting these as gaps would be describing the design and calling it \
         a fault - the mirror of the mistake the acquisition class exists to \
         avoid.\n"
    )
    .unwrap();
    for (id, rung) in &forced {
        writeln!(
            out,
            "  - `{}` — {}",
            id,
            rung.map(|r| format!("nominally rung {}", r)).unwrap_or_else(|| "in the county".into())
        )
        .unwrap();
    }

    writeln!(out, "\n### Answered, but not every branch — **{}** doors\n", partial.len()).unwrap();
    for (id, rung, taken, choices) in partial.iter().take(40) {
        writeln!(
            out,
            "  - `{}` — {}, {} of {} branches",
            id,
            rung.map(|r| format!("rung {}", r)).unwrap_or_else(|| "in the county".into()),
            taken,
            choices
        )
        .unwrap();
    }

    // ---- the fourth class -------------------------------------------------
    writeln!(out, "\n## Reachable only through a specific acquisition\n").unwrap();
    writeln!(
        out,
        "THE ATLAS cut the switchyard into two islands. These floors cannot be \
         walked to from any mouth — an Orb of Travel lands a run on a siding \
         inside them — so a ledger that counted them as missed would be \
         reporting a design decision as a bug.\n"
    )
    .unwrap();
    writeln!(out, "| dungeon | floors | walked to | islands |").unwrap();
    writeln!(out, "|---|---:|---:|---|").unwrap();
    for dg in DUNGEONS {
        let stood = seen.floors.get(dg.id).map(|s| s.len()).unwrap_or(0);
        let isl = islands(dg);
        writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            dg.id,
            dg.floors.len(),
            stood,
            if isl.is_empty() { "-".to_string() } else { format!("{:?}", isl) }
        )
        .unwrap();
    }

    // ---- towns, classes, county ------------------------------------------
    writeln!(out, "\n## Towns\n").unwrap();
    writeln!(out, "| town | gate reached | doors gone through |").unwrap();
    writeln!(out, "|---|---|---|").unwrap();
    for t in TOWNS {
        let doors = seen
            .town_doors
            .get(t.name)
            .map(|m| {
                m.iter().map(|(k, n)| format!("{} x{}", k, n)).collect::<Vec<_>>().join(", ")
            })
            .unwrap_or_else(|| "-".into());
        writeln!(
            out,
            "| {} | {} | {} |",
            t.name,
            if seen.gates.contains(t.name) { "yes" } else { "**no**" },
            doors
        )
        .unwrap();
    }

    writeln!(
        out,
        "\n## Classes\n\n{} of {} drunk: {}\n",
        seen.classes.len(),
        gearmaster_engine::class::CLASSES.len(),
        seen.classes.iter().cloned().collect::<Vec<_>>().join(", ")
    )
    .unwrap();

    writeln!(
        out,
        "## THE HUNDRED\n\n{} tiles stood on across {} runs. {} brawls walked into.\n",
        seen.county_tiles.len(),
        seen.runs,
        seen.brawls
    )
    .unwrap();

    if !stops.is_empty() {
        writeln!(out, "## Doors a run could not get past\n").unwrap();
        writeln!(out, "A door standing with no open choice stops the road. \
                       Each of these ended a run.\n").unwrap();
        for (door, count) in &stops {
            writeln!(out, "  - `{}` — {} runs", door, count).unwrap();
        }
    }

    std::fs::write("analysis/coverage.md", &out).expect("wrote the ledger");
    println!("{}", out);
    eprintln!("wrote analysis/coverage.md");
}

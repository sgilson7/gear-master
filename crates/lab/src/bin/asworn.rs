//! What the five borrowed boards actually score.
//!
//!     cargo run --release -p gearmaster-lab --bin asworn
//!
//! Q9's dresser reported every candidate board for THE SURVEYOR at exactly
//! 0.302 off a band of 0.300, and the `was` column - the creature as it ships
//! today - at the same 12.0 s. A number that does not move when the gear does
//! is not a measurement of the gear.
//!
//! So this asks the acceptance gate about the creatures **as they stand**,
//! which nothing in the repo does: `pack_francis` judges a board it is
//! proposing against the creature it is replacing, and both sides of that
//! comparison can be off the line together.

use gearmaster_engine::combat::{ALTERNATES, LADDER};
use gearmaster_oracle::gate::{self, Gate, References, Verdict};
use gearmaster_oracle::{Board, Oracle};

/// The five THE HUNDRED left in borrowed boards (`bestiary.rs:804`).
const COUNTY: [&str; 5] =
    ["THE SURVEYOR", "THE DROVER", "THE DRIVEN", "THE COMMISSIONER", "THE PARISH"];

fn main() {
    let refs = References::standard();
    let oracle = Oracle::new();
    println!("# The five borrowed boards, against the acceptance gate\n");
    println!(
        "`cargo run --release -p gearmaster-lab --bin asworn`. The creatures as they ship, \
         judged\nby the gate `pack_francis` judges a *proposal* with.\n"
    );
    println!("| creature | band | line | early | preset | owner | friend | verdict |");
    println!("|---|---:|---:|---:|---:|---:|---:|---|");
    for name in COUNTY {
        let Some(spec) = ALTERNATES.iter().chain(LADDER.iter()).find(|s| s.name == name) else {
            println!("| {} | — | — | | | | | *not found* |", name);
            continue;
        };
        // **At its own band.** `FRAMES` says where each of these stands -
        // THE SURVEYOR at 35, THE DROVER at 42 - and the first version of this
        // judged all five at 41 because that is what the harness defaulted to.
        let (rung, _) = gearmaster_engine::bestiary::FRAMES
            .iter()
            .find(|f| f.name == name)
            .map(|f| (f.band, f.theme))
            .unwrap_or((40, gearmaster_engine::bestiary::MonsterTheme::Beast));
        let g = Gate { refs: &refs, rung, rank: spec.rank };
        let rows = g.rows(&oracle, spec);
        // Judging a creature against itself: the only question is whether the
        // board it wears puts it on the line, and `judge` wants a `Board` for
        // the rank check, so it gets the one it is already wearing.
        let board = Board {
            gear: spec
                .gear
                .iter()
                .map(|&(n, s, x, y, r)| {
                    (
                        gearmaster_engine::piece::CATALOG
                            .iter()
                            .position(|d| d.name == n)
                            .unwrap_or(0),
                        s,
                        x,
                        y,
                        r,
                    )
                })
                .collect(),
            chunks: spec.items.to_vec(),
            rows: [0; 5],
        };
        let v = g.judge(&rows, &rows, &board);
        println!(
            "| **{}** | {} | {:.1}s | {}{:.1}s | {}{:.1}s | {}{:.1}s | {}{:.1}s | {} |",
            name,
            rung + 1,
            gate::target_ms(rung) as f64 / 1000.0,
            if rows[0][1].won { "W" } else { "L" },
            rows[0][1].ms as f64 / 1000.0,
            if rows[1][1].won { "W" } else { "L" },
            rows[1][1].ms as f64 / 1000.0,
            if rows[2][1].won { "W" } else { "L" },
            rows[2][1].ms as f64 / 1000.0,
            if rows[3][1].won { "W" } else { "L" },
            rows[3][1].ms as f64 / 1000.0,
            match v {
                Verdict::Accepted { off } => format!("**accepted**, {:.3} off", off),
                Verdict::OffCurve { off, allowed, .. } =>
                    format!("off-curve, {:.3} against {:.3}", off, allowed),
                other => format!("{:?}", other),
            }
        );
    }
    println!(
        "\nThe `owner` and `friend` columns are what the gate's curve is read from at this \
         band."
    );
}

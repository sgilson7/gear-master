//! Q9: dress a creature from the best run that can be found for it.
//!
//!     DRESS_FOR="THE SURVEYOR" cargo run --release -p gearmaster-lab --bin dress
//!
//! `harvest` turns **one** run's board into a creature. This runs it over many
//! runs and keeps the best, because which board a run is holding when it
//! reaches a band is the one thing a harvest cannot control and the one thing
//! that decides whether it lands on the line.
//!
//! The five county creatures wear borrowed boards - each one a ladder
//! creature's board spliced in whole (`bestiary.rs:804`) - and this is the
//! machine for replacing them with boards the economy actually produced.
//!
//! **It writes nothing.** `make pack`'s save once rewrote a creature nobody
//! was editing; the owner reads every diff.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::bestiary::MonsterTheme;
use gearmaster_engine::combat::{simulate_at, Difficulty as D, MonsterSpec, ALTERNATES, LADDER};
use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_lab::boards;
use gearmaster_oracle::gate::{self, Gate, References, Verdict};
use gearmaster_oracle::{fidelity, Board, Oracle};

fn subject(want: &str) -> (&'static MonsterSpec, usize) {
    if let Some(i) = LADDER.iter().position(|s| s.name == want) {
        return (&LADDER[i], i);
    }
    let i = ALTERNATES.iter().position(|s| s.name == want).unwrap_or(0);
    (&ALTERNATES[i], frame_of(&want).map_or(40, |(b, _)| b))
}

/// Where a creature stands and what it is meant to be.
///
/// **Off-ladder creatures carry their own band and theme** in `FRAMES`, and
/// the first version of this harness did not ask: it defaulted every one of
/// them to rung 40 and took the theme from `theme_for(40)`. So THE SURVEYOR -
/// band 35, Warden - was dressed as a band-41 Drainer, judged against a
/// 17.2 s line it was never meant to meet, and reported as reading 0.00 as a
/// Drainer, which was true and beside the point.
fn frame_of(name: &str) -> Option<(usize, MonsterTheme)> {
    gearmaster_engine::bestiary::FRAMES
        .iter()
        .find(|f| f.name == name)
        .map(|f| (f.band, f.theme))
}


fn distance(v: &Verdict) -> f64 {
    match v {
        Verdict::Accepted { off } => *off,
        Verdict::OffCurve { off, .. } => *off,
        _ => f64::INFINITY,
    }
}

fn main() {
    let want = std::env::var("DRESS_FOR").unwrap_or_else(|_| "THE SURVEYOR".into());
    let (spec, rung) = subject(&want);
    let theme = MonsterTheme::ALL
        .into_iter()
        .find(|t| std::env::var("DRESS_THEME").is_ok_and(|v| t.name().eq_ignore_ascii_case(&v)))
        .or_else(|| frame_of(spec.name).map(|(_, t)| t))
        .or_else(|| gearmaster_engine::bestiary::theme_for(rung));
    let tries: u64 = std::env::var("DRESS_TRIES").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
    let wanted: Vec<SlotKind> = match theme {
        Some(t) => t.slots().to_vec(),
        None => SlotKind::ALL.to_vec(),
    };

    println!(
        "Dressing {} (rung {}){}, from up to {} runs.\n",
        spec.name,
        rung + 1,
        match theme {
            Some(t) => format!(", as a {}", t.name()),
            None => String::new(),
        },
        tries
    );
    println!(
        "  the line wants {:.1}s at rung {}, within {:.0}%\n",
        gate::target_ms(rung) as f64 / 1000.0,
        rung + 1,
        gate::band_for(rung) * 100.0
    );

    let refs = References::standard();
    let oracle = Oracle::new();
    let g = Gate { refs: &refs, rung, rank: spec.rank };
    let was = g.rows(&oracle, spec);
    let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };

    let mut best: Option<(f64, u64, Board, Verdict, f64)> = None;
    println!("| run | reached | items | verdict | off | reads as |");
    println!("|---|---:|---:|---|---:|---:|");
    for i in 0..tries {
        let seed =
            0xC434_E4A6_8C59_0000u64.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mode = if i % 2 == 0 { Mode::Grinder } else { Mode::Rogue };
        let e = pilot::play(seed, mode, Difficulty::Medium, d);
        // A run that never got near the band cannot dress a creature standing
        // in it, and replaying one costs more than skipping it.
        if e.best_rung + 8 < rung {
            continue;
        }
        let mut c = Console::start(seed, mode, Difficulty::Medium);
        for line in &e.transcript {
            if let Some(v) = Verb::parse(line) {
                c.apply(v);
            }
        }
        let run = c.into_run();
        let cut = boards::cut(&run, &wanted);
        if cut.gear.is_empty() {
            continue;
        }
        let board = Board { gear: cut.gear, chunks: cut.chunks, rows: [0; 5] };
        let candidate = gearmaster_oracle::as_creature(spec, &board);
        let got = g.rows(&oracle, &candidate);
        let verdict = g.judge(&was, &got, &board);
        let off = distance(&verdict);
        let reads = theme.map_or(0.0, |t| {
            let (_, rstats, rits, _) = &refs.boards[0];
            fidelity::of(t, &simulate_at(*rstats, rits, &candidate, D::Medium)).score
        });
        println!(
            "| `{:#010X}` | {} | {} | {} | {} | {:.2} |",
            seed as u32,
            e.best_rung,
            board.chunks.len(),
            match &verdict {
                Verdict::Accepted { .. } => "**accepted**".to_string(),
                v => format!("{:?}", v).split(' ').next().unwrap().to_string(),
            },
            if off.is_finite() { format!("{:.3}", off) } else { "—".into() },
            reads
        );
        // Ranked by the gate first and the theme second: a board that is not
        // a fight at the right weight is not made one by reading well.
        let key = off - 0.05 * reads;
        if best.as_ref().is_none_or(|(bk, ..)| key < *bk) {
            best = Some((key, seed, board, verdict, reads));
        }
    }

    let Some((_, seed, board, verdict, reads)) = best else {
        println!("\nNo run got near band {}. Raise DRESS_TRIES.", rung + 1);
        return;
    };
    println!("\n## Best: `{:#018X}`\n", seed);
    let candidate = gearmaster_oracle::as_creature(spec, &board);
    let got = g.rows(&oracle, &candidate);
    for (i, (label, ..)) in refs.boards.iter().enumerate() {
        println!(
            "    {:<8} was {}{:.1}s   now {}{:.1}s",
            label,
            if was[i][1].won { "W" } else { "L" },
            was[i][1].ms as f64 / 1000.0,
            if got[i][1].won { "W" } else { "L" },
            got[i][1].ms as f64 / 1000.0
        );
    }
    println!("\n  verdict: {:?}", verdict);
    if let Some(t) = theme {
        println!("  reads as a {}: {:.2}", t.name(), reads);
    }
    println!("\nGEAR");
    for &(def, slot, x, y, rot) in &board.gear {
        println!(
            "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
            CATALOG[def].name, slot, x, y, rot
        );
    }
    println!("ITEMS &{:?}", board.chunks);
    println!(
        "\nNothing was written. Paste it by hand and read the diff - `make pack`'s\n\
         save once rewrote a creature nobody was editing."
    );
}

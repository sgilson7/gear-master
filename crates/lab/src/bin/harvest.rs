//! Q9: creature boards from a run that was actually played.
//!
//!     HARVEST_FOR="The Drover" cargo run --release -p gearmaster-lab --bin harvest
//!
//! A board the agent built is a board **the economy produced** - which
//! `pack_francis` never was, because it draws from the whole catalogue by
//! rating. Turning one into a creature is a smaller job than packing from
//! scratch: keep the assembled items, drop gear a creature may not wear, cut
//! to the theme's grids, trim to the rung's budget, and run A2's gate.
//!
//! It writes a `gear:` / `items:` block to paste. **It does not touch
//! `combat.rs`** - `make pack`'s save once rewrote a creature nobody was
//! editing, and the owner reads every diff.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode, Verb};
use gearmaster_engine::bestiary::MonsterTheme;
use gearmaster_engine::combat::{Difficulty as D, MonsterSpec, ALTERNATES, LADDER};
use gearmaster_engine::piece::{is_boss_only, is_event_only, SlotKind, CATALOG};
use gearmaster_oracle::gate::{self, Gate, References};
use gearmaster_oracle::{fidelity, Board, Oracle};

/// The creature to dress, and where it stands.
fn subject() -> (&'static MonsterSpec, usize) {
    let want = std::env::var("HARVEST_FOR").unwrap_or_else(|_| "Francis".into());
    if let Some(i) = LADDER.iter().position(|s| s.name == want) {
        return (&LADDER[i], i);
    }
    let i = ALTERNATES.iter().position(|s| s.name == want).unwrap_or(0);
    // An off-ladder creature is judged at the band its neighbours sit in.
    (&ALTERNATES[i], 40)
}

fn main() {
    let (spec, rung) = subject();
    let seed = std::env::var("HARVEST_SEED")
        .ok()
        .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0xC434_E4A6_8C59_06EE);
    let theme = MonsterTheme::ALL
        .into_iter()
        .find(|t| std::env::var("HARVEST_THEME").is_ok_and(|v| t.name().eq_ignore_ascii_case(&v)))
        .or_else(|| gearmaster_engine::bestiary::theme_for(rung));

    println!(
        "Dressing {} (rung {}), from seed {:#018X}{}.\n",
        spec.name,
        rung + 1,
        seed,
        match theme {
            Some(t) => format!(", as a {}", t.name()),
            None => String::new(),
        }
    );

    // ---- a board a run actually built -------------------------------
    let d = Doctrine { patience: 24, budget: 600_000, coverage: 0.0 };
    let e = pilot::play(seed, Mode::Grinder, Difficulty::Medium, d);
    let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
    // Replay to the deepest board it held, which is where it stopped.
    for line in &e.transcript {
        if let Some(v) = Verb::parse(line) {
            c.apply(v);
        }
    }
    let v = c.view();
    println!(
        "  the run reached rung {} and its board holds {} items across {} cells",
        e.best_rung,
        v.grids.iter().map(|g| g.items.iter().filter(|i| i.assembled).count()).sum::<usize>(),
        v.grids.iter().map(|g| g.cells.iter().filter(|c| c.piece.is_some()).count()).sum::<usize>()
    );

    // ---- cut it down to a creature -----------------------------------
    //
    // Three rules, and every one of them is the engine's rather than mine:
    // a creature's gear must all assemble (`MonsterSpec::unassembled`), it may
    // not wear boss or event gear, and a theme fills two or three grids.
    let wanted: Vec<SlotKind> = match theme {
        Some(t) => t.slots().to_vec(),
        None => SlotKind::ALL.to_vec(),
    };
    let mut gear: Vec<(usize, SlotKind, u8, u8, u8)> = Vec::new();
    let mut chunks: Vec<usize> = Vec::new();
    let mut dropped: Vec<String> = Vec::new();

    let run = c.into_run();
    for k in wanted.iter().copied() {
        for item in run.report(k).items.iter().filter(|i| i.assembled) {
            let names: Vec<&str> = item.pieces.iter().map(|&p| run.registry.def(p).name).collect();
            if names.iter().any(|n| is_boss_only(n) || is_event_only(n)) {
                dropped.push(format!("{} - holds gear a creature may not wear", item.name.full));
                continue;
            }
            let slot = run.loadout.slot(k);
            let mut placed = 0;
            for &p in &item.pieces {
                let Some((x, y)) = slot.anchor_of(p) else { continue };
                gear.push((run.registry.def_index(p), k, x, y, run.registry.rotation(p)));
                placed += 1;
            }
            if placed > 0 {
                chunks.push(placed);
            }
        }
    }

    println!(
        "  cut to {}: {} pieces in {} items{}",
        wanted.iter().map(|s| format!("{:?}", s).to_lowercase()).collect::<Vec<_>>().join(" + "),
        gear.len(),
        chunks.len(),
        if dropped.is_empty() { String::new() } else { format!(", {} dropped", dropped.len()) }
    );
    for d in &dropped {
        println!("    dropped {}", d);
    }
    if gear.is_empty() {
        println!("\nnothing to dress it in. Try a deeper seed.");
        return;
    }

    // ---- what it would give ------------------------------------------
    let board = Board { gear: gear.clone(), chunks: chunks.clone(), rows: [0; 5] };
    let refs = References::standard();
    let oracle = Oracle::new();
    let candidate = gearmaster_oracle::as_creature(spec, &board);
    let g = Gate { refs: &refs, rung, rank: spec.rank };
    let was = g.rows(&oracle, spec);
    let got = g.rows(&oracle, &candidate);
    let verdict = g.judge(&was, &got, &board);

    println!("\n  against the four reference boards, at Medium:");
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
    println!(
        "\n  the line wants {:.1}s at rung {}, within {:.0}%",
        gate::target_ms(rung) as f64 / 1000.0,
        rung + 1,
        gate::band_for(rung) * 100.0
    );
    println!("  verdict: {:?}", verdict);

    if let Some(t) = theme {
        let (_, stats, items, _) = &refs.boards[0];
        let log = gearmaster_engine::combat::simulate_at(*stats, items, &candidate, D::Medium);
        let f = fidelity::of(t, &log);
        println!(
            "  reads as a {}: {:.2}   ({})",
            t.name(),
            f.score,
            f.parts.iter().map(|(w, v)| format!("{} {:.2}", w, v)).collect::<Vec<_>>().join(" · ")
        );
    }

    // ---- the block to paste ------------------------------------------
    println!("\nGEAR");
    for &(def, slot, x, y, rot) in &gear {
        println!(
            "            (\"{}\", SlotKind::{:?}, {}, {}, {}),",
            CATALOG[def].name, slot, x, y, rot
        );
    }
    println!("ITEMS &{:?}", chunks);
    println!(
        "\nNothing was written. Paste it by hand and read the diff - `make pack`'s\n\
         save once rewrote a creature nobody was editing."
    );
}

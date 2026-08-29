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
use gearmaster_engine::piece::{SlotKind, CATALOG};
use gearmaster_oracle::gate::{self, Gate, References};
use gearmaster_oracle::{fidelity, Board, Oracle};

/// The creature to dress, and where it stands.
fn subject() -> (&'static MonsterSpec, usize) {
    let want = std::env::var("HARVEST_FOR").unwrap_or_else(|_| "Francis".into());
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


fn main() {
    let (spec, rung) = subject();
    let seed = std::env::var("HARVEST_SEED")
        .ok()
        .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0xC434_E4A6_8C59_06EE);
    let theme = MonsterTheme::ALL
        .into_iter()
        .find(|t| std::env::var("HARVEST_THEME").is_ok_and(|v| t.name().eq_ignore_ascii_case(&v)))
        .or_else(|| frame_of(spec.name).map(|(_, t)| t))
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
    let run = c.into_run();
    let gearmaster_lab::boards::Cut { mut gear, mut chunks, dropped } =
        gearmaster_lab::boards::cut(&run, &wanted);

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

    // ---- trim it onto the line ----------------------------------------
    //
    // **A board a run reached rung 52 with is too much creature.** The first
    // harvest that had a deep run behind it came in at 0.302 off a band of
    // 0.300 - two thousandths over, killing the two shallow reference boards
    // in 1.3 s against a line of 17.2 - because a player's board at rung 52 is
    // not a rung-41 creature and nothing had said so.
    //
    // So: drop one item at a time, keep the drop that moves the verdict
    // nearest the line, and stop when it is accepted or when dropping stops
    // helping. Greedy, and greedy is the right shape here - the items are
    // whole and the curve is monotone in how many of them there are.
    let refs = References::standard();
    let oracle = Oracle::new();
    let g = Gate { refs: &refs, rung, rank: spec.rank };
    let was = g.rows(&oracle, spec);

    let judge = |gear: &Vec<(usize, SlotKind, u8, u8, u8)>, chunks: &Vec<usize>| {
        let board = Board { gear: gear.clone(), chunks: chunks.clone(), rows: [0; 5] };
        let candidate = gearmaster_oracle::as_creature(spec, &board);
        let got = g.rows(&oracle, &candidate);
        (g.judge(&was, &got, &board), board, candidate)
    };
    let distance = |v: &gearmaster_oracle::gate::Verdict| match v {
        gearmaster_oracle::gate::Verdict::Accepted { off } => *off,
        gearmaster_oracle::gate::Verdict::OffCurve { off, .. } => *off,
        // Anything else is not a matter of degree, and dropping items cannot
        // walk out of it - a board that holds nothing its rank owes holds less
        // of it with one item fewer.
        _ => f64::INFINITY,
    };

    let (mut verdict, ..) = judge(&gear, &chunks);
    let before = format!("{:?}", verdict);
    let mut trimmed = 0usize;
    // Trimming can only ever make a creature *weaker*, so it is the answer
    // when the board overshoots and no answer at all when it undershoots. The
    // first deep harvest landed at 0.302 off a band of 0.300 because it dies
    // to the owner's board in 12.0 s against a line of 17.2 - too little
    // creature, not too much - and the loop below correctly refused to move.
    // That is what `HARVEST_SEEDS` is for: a different run reaches band 41
    // holding a different board, and searching runs is the only lever a
    // harvest has when the one it was handed is under the line.
    while !verdict.accepted() && trimmed < chunks.len().saturating_sub(1) {
        // Every item, dropped in turn.
        let mut best: Option<(f64, Vec<_>, Vec<usize>, _)> = None;
        let mut at = 0usize;
        for (i, &n) in chunks.iter().enumerate() {
            let mut gear2 = gear.clone();
            gear2.drain(at..at + n);
            let mut chunks2 = chunks.clone();
            chunks2.remove(i);
            if chunks2.is_empty() {
                at += n;
                continue;
            }
            let (v, ..) = judge(&gear2, &chunks2);
            let d = distance(&v);
            if best.as_ref().is_none_or(|(bd, ..)| d < *bd) {
                best = Some((d, gear2, chunks2, v));
            }
            at += n;
        }
        let Some((d, gear2, chunks2, v)) = best else { break };
        if d >= distance(&verdict) {
            break;
        }
        gear = gear2;
        chunks = chunks2;
        verdict = v;
        trimmed += 1;
        println!("  trimmed an item -> {} items, {:.3} off the line", chunks.len(), d);
    }
    if trimmed > 0 {
        println!("  {} -> {:?}", before, verdict);
    }

    // ---- what it would give ------------------------------------------
    let (verdict, _board, candidate) = judge(&gear, &chunks);
    let got = g.rows(&oracle, &candidate);

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

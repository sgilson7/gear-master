//! Every creature, scored against its own theme.
//!
//! A theme filters the pool a creature draws from and nothing has ever asked
//! whether the resulting fight *reads* as the theme. This does, for all
//! eighty-three creatures, against all four reference boards, and prints the
//! table.
//!
//!     cargo run --release -p gearmaster-oracle --bin themes
//!     cargo run --release -p gearmaster-oracle --bin themes -- --full
//!
//! It asserts nothing. It is a measurement, and what it measures is a claim
//! the catalogue makes about itself.

use gearmaster_engine::bestiary::{theme_for, MonsterTheme};
use gearmaster_engine::combat::{simulate_at, Difficulty, MonsterSpec, ALTERNATES, LADDER};
use gearmaster_oracle::fidelity;
use gearmaster_oracle::gate::References;

fn main() {
    let full = std::env::args().any(|a| a == "--full");
    let refs = References::standard();

    println!(
        "Every creature against its own theme, at Medium.\n\n\
         FELT is scored against the two boards a player might actually have at \
         that rung - the four-piece\nboard and the preset. CURVE is scored \
         against the two finished builds the difficulty line is read off.\n\
         Both are the mean of the theme's parts, each a ratio over the fight.\n"
    );
    println!(
        "{:<22} {:<6} {:<9} {:>6} {:>7}  {}",
        "creature", "rung", "theme", "felt", "curve", "what the fight did, against the four-piece board"
    );
    println!("{}", "-".repeat(120));

    let mut by_theme: Vec<(MonsterTheme, Vec<(f64, f64)>)> =
        MonsterTheme::ALL.into_iter().map(|t| (t, Vec::new())).collect();
    let mut rows: Vec<(String, f64, MonsterTheme)> = Vec::new();

    for (i, spec) in LADDER.iter().enumerate() {
        let Some(theme) = theme_for(i) else { continue };
        let (felt, curve, note) = read(&refs, spec, theme);
        println!(
            "{:<22} {:<6} {:<9} {:>6.2} {:>7.2}  {}",
            spec.name, i + 1, theme.name(), felt, curve, note
        );
        by_theme.iter_mut().find(|(t, _)| *t == theme).unwrap().1.push((felt, curve));
        rows.push((spec.name.to_string(), felt, theme));
    }

    if full {
        println!("\nThe ladder past rung 44 is deliberately unthemed, and so is\
                  \nALTERNATES - both are scored against every theme, to say which\
                  \none each is closest to.\n");
        for spec in LADDER.iter().skip(44).chain(ALTERNATES.iter()) {
            let mut best = (MonsterTheme::Beast, 0.0f64);
            for t in MonsterTheme::ALL {
                let (s, ..) = read(&refs, spec, t);
                if s > best.1 {
                    best = (t, s);
                }
            }
            let (.., note) = read(&refs, spec, best.0);
            println!(
                "{:<22} {:<6} {:<9} {:>6.2} {:>7}  {}",
                spec.name, "-", best.0.name(), best.1, "-", note
            );
        }
    }

    println!("\n{}", "-".repeat(120));
    println!(
        "{:<12} {:>4} {:>7} {:>7} {:>7} {:>7}   {}",
        "theme", "n", "felt", "worst", "best", "curve", "the claim"
    );
    for (t, scores) in &by_theme {
        if scores.is_empty() {
            continue;
        }
        let felt = scores.iter().map(|(f, _)| f).sum::<f64>() / scores.len() as f64;
        let curve = scores.iter().map(|(_, c)| c).sum::<f64>() / scores.len() as f64;
        let worst = scores.iter().map(|(f, _)| *f).fold(f64::MAX, f64::min);
        let best = scores.iter().map(|(f, _)| *f).fold(0.0, f64::max);
        println!(
            "{:<12} {:>4} {:>7.2} {:>7.2} {:>7.2} {:>7.2}   {}",
            t.name(),
            scores.len(),
            felt,
            worst,
            best,
            curve,
            t.reads_as()
        );
    }

    rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    println!("\nThe ten that read least like what they say they are:");
    for (name, score, theme) in rows.iter().take(10) {
        let parts = parts_of(&refs, name, *theme);
        println!("  {:<22} {:<9} {:>5.2}   {}", name, theme.name(), score, parts);
    }
}

/// Score one creature twice, and say in one line what its fight did.
///
/// Twice, because **a fidelity score against a board that cannot feel the
/// theme measures the board.** Curse resistance is clamped to 100 and scales a
/// curse's duration to nothing at that point (`curse.rs:137`); the owner's
/// finished build carries 145 and the friend's 135, so every curse either of
/// them meets lands for zero milliseconds. Three themes speak mostly in
/// curses, and against those two boards they are silent - which is a fact
/// about the yardstick and not about the creature.
fn read(refs: &References, spec: &MonsterSpec, theme: MonsterTheme) -> (f64, f64, String) {
    let mut felt = (0.0, 0);
    let mut curve = (0.0, 0);
    let mut note = String::new();
    for (label, stats, items, _) in &refs.boards {
        let log = simulate_at(*stats, items, spec, Difficulty::Medium);
        let f = fidelity::of(theme, &log);
        if *label == "early" || *label == "preset" {
            felt.0 += f.score;
            felt.1 += 1;
        } else {
            curve.0 += f.score;
            curve.1 += 1;
        }
        if *label == "early" {
            let r = &f.reading;
            note = format!(
                "{:>2} blows avg {:>3}, burn {:>4}, mind {:>4}, curses {:>2}, drain {:>3}, {:>2} acts, {:.1}s",
                r.blows, r.mean_blow, r.burn_damage, r.mind_damage, r.curses, r.drained,
                r.activations, r.ms as f64 / 1000.0
            );
        }
    }
    (
        if felt.1 == 0 { 0.0 } else { felt.0 / felt.1 as f64 },
        if curve.1 == 0 { 0.0 } else { curve.0 / curve.1 as f64 },
        note,
    )
}

fn parts_of(refs: &References, name: &str, theme: MonsterTheme) -> String {
    let Some(spec) = LADDER.iter().find(|s| s.name == name) else { return String::new() };
    let (_, stats, items, _) = &refs.boards[0];
    let log = simulate_at(*stats, items, spec, Difficulty::Medium);
    fidelity::of(theme, &log)
        .parts
        .iter()
        .map(|(what, v)| format!("{} {:.2}", what, v))
        .collect::<Vec<_>>()
        .join(" · ")
}

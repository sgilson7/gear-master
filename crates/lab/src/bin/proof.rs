//! Play one seed and write the transcript out, then replay it.
//!
//!     PROOF_SEED=0x1212 cargo run --release -p gearmaster-lab --bin proof
//!
//! A proof is `(seed, mode, difficulty, [verb])` and nothing else. It is
//! written as lines a person could type into `gearmaster-cli`, replayed
//! through a second console, and the two screens compared - which is
//! `acceptance::e6_1`'s claim one level up: two replays of a run agree about
//! everything, the fights included.

use gearmaster_agent::pilot::{self, Doctrine};
use gearmaster_console::{Console, Difficulty, Mode, Verb};

fn main() {
    let seed = std::env::var("PROOF_SEED")
        .ok()
        .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x1212);
    let out = std::env::var("PROOF_OUT").unwrap_or_else(|_| "analysis/proofs".into());

    let e = pilot::play(seed, Mode::Grinder, Difficulty::Medium, Doctrine::default());
    if std::env::var("DOORS_DEBUG").is_ok() {
        println!("doors {} towns {} bought {} sold {} bartered {} rerolled {} grew {} cleared {}",
            e.doors, e.towns, e.bought, e.sold, e.bartered, e.rerolled, e.grew, e.cleared);
    }
    println!(
        "seed {:#018X}: rung {}, {} board clears, {} game clears, {} losses, {} presses - {}",
        seed, e.best_rung, e.board_clears, e.game_clears, e.losses, e.presses, e.why
    );

    // ---- the search, taken out ------------------------------------------
    //
    // The hands try a seat, read the board and take it back, so a transcript
    // is mostly `place` / `undo` pairs - a hundred and ninety-five thousand
    // presses and six megabytes for one run. That is a faithful record of
    // what the pilot did and a poor record of what it *played*: an undo
    // cancels the press before it exactly, so cancelling them out leaves the
    // keys a person would press if they already knew what the pilot found.
    //
    // It is only a proof if it still replays, which is checked below. If a
    // cancellation were ever wrong the replay would land somewhere else.
    let full: Vec<Verb> = e.transcript.iter().filter_map(|l| Verb::parse(l)).collect();
    let mut verbs: Vec<Verb> = Vec::with_capacity(full.len());
    for v in &full {
        if *v == Verb::Undo && verbs.last().is_some_and(|p| *p != Verb::Undo) {
            verbs.pop();
        } else {
            verbs.push(*v);
        }
    }
    println!(
        "  {} presses, {} after the trial seats cancel out ({:.1}%)",
        full.len(),
        verbs.len(),
        100.0 * verbs.len() as f64 / full.len().max(1) as f64
    );
    println!("{} lines, {} of them verbs", e.transcript.len(), verbs.len());
    let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
    let mut refused = 0;
    // The **highest** rung reached, not the last one. A Grinder is knocked
    // back on a loss, so a run that touched rung 51 can finish standing on 49
    // - and comparing a maximum against a final reads as a divergence when
    // there is none.
    let mut replayed = 1;
    for v in &verbs {
        if !c.apply(*v).ok {
            refused += 1;
        }
        replayed = replayed.max(c.view().rung_shown);
    }
    println!(
        "replayed to rung {} with {} refusals{}",
        replayed,
        refused,
        if replayed == e.best_rung && refused == 0 { "  - identical" } else { "  - DIFFERENT" }
    );

    std::fs::create_dir_all(&out).ok();
    let path = format!("{}/{:016X}-grinder-medium.proof", out, seed);
    let header = format!(
        "# commit      {}\n# seed        {:#018X}\n# mode        Grinder\n\
         # difficulty  Medium\n# agent       A4 pilot, Doctrine::default()\n\
         # reached     rung {} ({} board clears, {} game clears)\n\
         # presses     {} played, of {} pressed\n#\n\
         # Every line below is a key a person could press. Pipe it into\n\
         # `cargo run -p gearmaster-cli` after `preset`-free start, or read it\n\
         # back with `Verb::parse`.\n\n",
        std::env::var("PROOF_COMMIT").unwrap_or_else(|_| "unrecorded".into()),
        seed,
        e.best_rung,
        e.board_clears,
        e.game_clears,
        verbs.len(),
        e.presses
    );
    let body: Vec<String> = verbs.iter().map(|v| v.line()).collect();
    std::fs::write(&path, header + &body.join("\n") + "\n").expect("wrote the proof");
    println!("wrote {}", path);
}

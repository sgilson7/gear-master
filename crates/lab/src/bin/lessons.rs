//! Collect what the hands already learn and throw away.
//!
//!     cargo run --release -p gearmaster-lab --bin lessons
//!
//! Writes a plain binary of `(24 floats, 1 float)` rows to `runs/lessons.bin`,
//! which is gitignored: it is reproducible from this command and the seeds in
//! its header, so it is not source.

use gearmaster_agent::hands;
use gearmaster_agent::lesson::{Lesson, FEATURES};
use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::rng::Rng;
use gearmaster_engine::run::Run;
use gearmaster_engine::share;
use std::io::Write;

fn seeds(n: usize) -> Vec<u64> {
    let mut out = vec![0x5EED_1234_ABCD_0001u64, 0x6060, 0x1111, 0x1212];
    let mut r = Rng::new(0x501_7E5);
    while out.len() < n {
        out.push(r.next_u64());
    }
    out.truncate(n);
    out
}

/// Trays to pack: the three finished builds, in loads of twelve, which is the
/// same shape a run meets them in.
fn pieces_of(code: &str) -> Vec<usize> {
    share::import(code).expect("a code the repo ships").placed.iter().map(|&(d, ..)| d).collect()
}

fn main() {
    let n: usize = std::env::var("LESSON_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
    let mut lessons: Vec<Lesson> = Vec::new();

    // From real play: the boards a run actually builds, at the rungs it builds
    // them. This is the distribution the prior has to be right about.
    for seed in seeds(n) {
        let mut c = Console::start(seed, Mode::Grinder, Difficulty::Medium);
        for _ in 0..40 {
            let menu = c.menu();
            if menu.is_empty() {
                break;
            }
            if !c.tray_ids().is_empty() {
                hands::pack_recording(&mut c, 40_000, &mut lessons);
            }
            let Some(fight) =
                menu.iter().find(|v| matches!(v, gearmaster_console::Verb::Fight)).copied()
            else {
                break;
            };
            if !c.apply(fight).ok {
                break;
            }
        }
    }

    // And from the three finished trays, which is where the deep board shapes
    // are - a run reaching rung 47 is packing something like these.
    for code in [share::A_WINNING_RUN, share::A_FRIENDS_RUN, share::A_PERFECT_RUN] {
        let defs = pieces_of(code);
        let mut run = Run::new();
        run.clear_all();
        run.owned.clear();
        for load in defs.chunks(gearmaster_engine::run::INVENTORY_CAP) {
            for &d in load {
                let id = run.registry.alloc(d);
                run.owned.push(id);
            }
            let mut c = Console::standing_in(run, 0);
            hands::pack_recording(&mut c, 400_000, &mut lessons);
            run = c.into_run();
            for id in run.inventory() {
                run.owned.retain(|&o| o != id);
            }
        }
    }

    std::fs::create_dir_all("runs").ok();
    let mut f = std::io::BufWriter::new(std::fs::File::create("runs/lessons.bin").unwrap());
    for l in &lessons {
        for v in l.x {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
        f.write_all(&l.y.to_le_bytes()).unwrap();
    }
    f.flush().unwrap();

    let positive = lessons.iter().filter(|l| l.y > 0.0).count();
    let mut ys: Vec<f32> = lessons.iter().map(|l| l.y).collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "{} lessons, {} features each\n  {} improved the board ({:.1}%)\n  \
         worth: min {:.2}, median {:.2}, max {:.2}\n  wrote runs/lessons.bin ({} KB)",
        lessons.len(),
        FEATURES,
        positive,
        100.0 * positive as f64 / lessons.len().max(1) as f64,
        ys.first().copied().unwrap_or(0.0),
        ys.get(ys.len() / 2).copied().unwrap_or(0.0),
        ys.last().copied().unwrap_or(0.0),
        lessons.len() * (FEATURES + 1) * 4 / 1024
    );
}

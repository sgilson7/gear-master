//! Q3's gate: does the learned quartermaster pack as well as the control?
//!
//!     cargo run --release -p gearmaster-lab --bin qcheck
//!
//! The same repack-from-tray benchmark A3 set and the hand-written packer
//! passed at 48/50 and 49/50. Building is blind - the agent sees a tray, five
//! grids and a shop, and presses keys - and the ladder walk afterwards is the
//! harness's.

use gearmaster_console::{Console, Difficulty, Mode};
use gearmaster_engine::combat::{simulate_at, Outcome, LADDER, SUDDEN_DEATH_MS};
use gearmaster_engine::piece::PieceId;
use gearmaster_engine::run::Run;
use gearmaster_engine::share;
use gearmaster_trades::env::{Move, Packing};
use gearmaster_trades::QNet;
use std::time::Instant;

fn pieces_of(code: &str) -> Vec<usize> {
    share::import(code).expect("a code the repo ships").placed.iter().map(|&(d, ..)| d).collect()
}

fn preset_pieces() -> Vec<usize> {
    let mut run = Run::new();
    run.apply_preset();
    run.owned.iter().map(|&id| run.registry.def_index(id)).collect()
}

fn empty_run() -> Run {
    let mut run = Run::new();
    run.mode = Mode::Grinder;
    run.clear_all();
    run.owned.clear();
    run
}

fn ladder(run: &Run) -> (usize, usize, u32) {
    let (stats, items) = (run.player_stats(), run.combat_items());
    let (mut won, mut decided, mut ttk) = (0, 0, Vec::new());
    for spec in LADDER {
        let log = simulate_at(stats, &items, spec, Difficulty::Medium);
        if log.outcome == Outcome::Victory {
            won += 1;
            ttk.push(log.duration_ms);
            if log.duration_ms < SUDDEN_DEATH_MS {
                decided += 1;
            }
        }
    }
    ttk.sort_unstable();
    (won, decided, ttk.get(ttk.len() / 2).copied().unwrap_or(0))
}

fn main() {
    let path = std::env::var("QCHECK_NET").unwrap_or_else(|_| "runs/quartermaster.txt".into());
    let net = QNet::load(&path);
    println!(
        "Repack-from-tray. {}\n",
        match &net {
            Some(_) => format!("The learned quartermaster, from {}.", path),
            None => "No network found - nothing to check.".into(),
        }
    );
    let Some(net) = net else { return };
    println!(
        "{:<10} {:>6} {:>7} {:>7} {:>8} {:>9} {:>8}",
        "tray", "pieces", "items", "steps", "cleared", "median", "wall"
    );
    println!("{}", "-".repeat(64));

    for (label, defs) in [
        ("preset", preset_pieces()),
        ("owner", pieces_of(share::A_WINNING_RUN)),
        ("friend", pieces_of(share::A_FRIENDS_RUN)),
        ("perfect", pieces_of(share::A_PERFECT_RUN)),
    ] {
        let n = defs.len();
        let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
        let t = Instant::now();
        let mut run = empty_run();
        let mut steps = 0usize;
        for load in defs.chunks(gearmaster_engine::run::INVENTORY_CAP) {
            for &d in load {
                let id = run.registry.alloc(d);
                run.owned.push(id);
            }
            let mut c = Console::standing_in(run, 0);
            let mut e = Packing::new(60);
            loop {
                let ms = e.moves(&c);
                if ms.is_empty() {
                    break;
                }
                let v = c.view();
                let verbs: Vec<_> = ms
                    .iter()
                    .filter_map(|m| match m {
                        Move::Press(v) => Some(*v),
                        Move::Done => None,
                    })
                    .collect();
                // `Done` is scored as the all-zero action, the same way the
                // trainer scores it.
                let b = gearmaster_trades::feature::board(&v);
                let done_q =
                    net.q(&gearmaster_trades::feature::pair(&b, &[0.0; gearmaster_trades::feature::MOVE]));
                let best = net.best(&v, &verbs);
                let m = match best {
                    Some((i, q)) if q >= done_q => Move::Press(verbs[i]),
                    _ => Move::Done,
                };
                *kinds.entry(match m {
                    Move::Done => "Done",
                    Move::Press(v) => gearmaster_trades::partition::name_of(v),
                })
                .or_insert(0usize) += 1;
                e.step(&mut c, m);
                steps += 1;
                if e.finished {
                    break;
                }
            }
            run = c.into_run();
            let left: Vec<PieceId> = run.inventory();
            for id in left {
                run.owned.retain(|&o| o != id);
            }
        }
        let el = t.elapsed().as_secs_f64();
        let (won, decided, median) = ladder(&run);
        let items = run.combat_items().len();
        println!(
            "{:<10} {:>6} {:>7} {:>7} {:>4}/{:<3} {:>8.2}s {:>7.1}s",
            label,
            n,
            items,
            steps,
            won,
            decided,
            median as f64 / 1000.0,
            el
        );
        let mut k: Vec<_> = kinds.into_iter().collect();
        k.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        println!(
            "           what it pressed: {}",
            k.into_iter().take(5).map(|(n, c)| format!("{} x{}", n, c)).collect::<Vec<_>>().join(", ")
        );
    }
    println!(
        "\nthe gate: >= 48/50 from the owner's tray and >= 49/50 from the friend's,\n\
         which is what the hand-written packer does (A3)."
    );
}
